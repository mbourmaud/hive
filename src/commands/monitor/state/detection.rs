use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::commands::common::{
    command_with_timeout, is_process_running, parse_timestamp, read_drone_pid,
};
use crate::notification;
use crate::types::DroneState;

use super::TuiState;

/// Persist a drone's current status to its `status.json` file on disk.
fn persist_drone_status(name: &str, status: &crate::types::DroneStatus) {
    let status_path = PathBuf::from(".hive/drones").join(name).join("status.json");
    let _ = fs::write(
        &status_path,
        serde_json::to_string_pretty(status).unwrap_or_default(),
    );
}

/// Check PR state with timeout. Returns true if PR is in expected state.
fn check_pr_state(
    cache: &std::collections::HashMap<String, (String, Instant)>,
    branch: &str,
    expected_state: &str,
) -> bool {
    if let Some((cached_state, when)) = cache.get(branch) {
        if when.elapsed() < Duration::from_secs(60) {
            return cached_state == expected_state;
        }
    }

    let mut cmd = std::process::Command::new("gh");
    cmd.args(["pr", "view", branch, "--json", "state", "-q", ".state"]);

    let result = command_with_timeout(&mut cmd, 5)
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    result == expected_state
}

/// Mark drones whose process died but status is still active as zombie/stopped.
pub fn detect_zombies(state: &mut TuiState) {
    let now_utc = Utc::now();
    for (name, status) in &mut state.drones {
        if !matches!(
            status.status,
            DroneState::InProgress | DroneState::Starting | DroneState::Resuming
        ) {
            continue;
        }

        if matches!(status.status, DroneState::Starting | DroneState::Resuming) {
            let age_secs = parse_timestamp(&status.updated)
                .map(|t| now_utc.signed_duration_since(t).num_seconds())
                .unwrap_or(0);
            if age_secs < 30 {
                continue;
            }
        }

        let pid_alive = read_drone_pid(name)
            .map(is_process_running)
            .unwrap_or(false);
        if !pid_alive {
            let _ = crate::agent_teams::auto_complete_tasks(name);

            if crate::events::has_stop_event(name) {
                status.status = DroneState::Stopped;
            } else {
                status.status = DroneState::Zombie;
                state
                    .zombie_first_seen
                    .entry(name.clone())
                    .or_insert_with(Instant::now);
            }
            persist_drone_status(name, status);
        }
    }

    state.zombie_first_seen.retain(|name, _| {
        state
            .drones
            .iter()
            .any(|(n, s)| n == name && s.status == DroneState::Zombie)
    });
}

/// Check for .hive_complete marker files indicating drone completion.
pub fn detect_completion_markers(state: &mut TuiState) {
    for (name, status) in &mut state.drones {
        if !matches!(
            status.status,
            DroneState::InProgress | DroneState::Starting | DroneState::Resuming
        ) {
            continue;
        }
        let marker = PathBuf::from(&status.worktree).join(".hive_complete");
        if marker.exists() {
            status.status = DroneState::Completed;
            status.updated = Utc::now().to_rfc3339();
            persist_drone_status(name, status);
            let _ = crate::commands::kill_clean::kill_quiet(name.to_string());
            let _ = fs::remove_file(&marker);
            notification::notify(&format!("Hive - {}", name), "Drone completed!");
        }
    }
}

/// Check if InProgress drones have an open PR with all tasks done.
pub fn detect_pr_completion(state: &mut TuiState) {
    state.pr_completion_check_counter += 1;
    if state.pr_completion_check_counter < 300 {
        return;
    }
    state.pr_completion_check_counter = 0;

    let candidates: Vec<(String, String)> = state
        .drones
        .iter()
        .filter(|(_, s)| matches!(s.status, DroneState::InProgress))
        .map(|(name, s)| (name.clone(), s.branch.clone()))
        .collect();

    for (name, branch) in candidates {
        let pr_open = check_pr_state(&state.pr_state_cache, &branch, "OPEN");
        if !pr_open {
            continue;
        }
        let (completed, total) = state.snapshot_store.progress(&name);
        if total == 0 || completed < total {
            continue;
        }

        if let Some((_, status)) = state.drones.iter_mut().find(|(n, _)| n == &name) {
            status.status = DroneState::Completed;
            status.updated = Utc::now().to_rfc3339();
            persist_drone_status(&name, status);
        }
        let _ = crate::commands::kill_clean::kill_quiet(name.to_string());
        notification::notify(&format!("Hive - {}", name), "Drone completed (PR created)!");
    }
}

/// Auto-clean drones whose PRs have been merged.
pub fn detect_pr_merges(state: &mut TuiState) {
    state.merge_check_counter += 1;
    if state.merge_check_counter < 600 {
        return;
    }
    state.merge_check_counter = 0;

    let merged: Vec<String> = state
        .drones
        .iter()
        .filter(|(_, s)| matches!(s.status, DroneState::Completed | DroneState::Stopped))
        .filter(|(_, s)| check_pr_state(&state.pr_state_cache, &s.branch, "MERGED"))
        .map(|(name, _)| name.clone())
        .collect();

    for name in &merged {
        crate::commands::kill_clean::clean_background(name.clone());
        notification::notify(
            "Hive",
            &format!("PR merged — drone '{}' auto-cleaned", name),
        );
    }
}

/// Auto-stop drones that have all tasks done and no new events for 2 minutes.
pub fn detect_idle_drones(state: &mut TuiState) {
    const IDLE_TIMEOUT_SECS: u64 = 120;
    let idle_candidates: Vec<String> = state
        .drones
        .iter()
        .filter(|(_, s)| matches!(s.status, DroneState::InProgress))
        .filter(|(name, _)| !state.auto_stopped_drones.contains(name))
        .filter_map(|(name, _)| {
            let (completed, total) = state.snapshot_store.progress(name);
            if total > 0 && completed >= total {
                let first_seen = state
                    .all_tasks_done_since
                    .entry(name.clone())
                    .or_insert_with(Instant::now);
                let last_event = state.last_event_time.get(name).copied();
                let idle_long_enough =
                    first_seen.elapsed() > Duration::from_secs(IDLE_TIMEOUT_SECS);
                let no_recent_events = last_event
                    .map(|t| t.elapsed() > Duration::from_secs(IDLE_TIMEOUT_SECS))
                    .unwrap_or(true);
                if idle_long_enough && no_recent_events {
                    Some(name.clone())
                } else {
                    None
                }
            } else {
                state.all_tasks_done_since.remove(name);
                None
            }
        })
        .collect();

    for name in &idle_candidates {
        let _ = crate::agent_teams::auto_complete_tasks(name);
        state.auto_stopped_drones.insert(name.clone());
        let _ = crate::commands::kill_clean::kill_quiet(name.clone());

        if let Some((_, status)) = state.drones.iter_mut().find(|(n, _)| n == name) {
            status.status = DroneState::Completed;
            status.updated = Utc::now().to_rfc3339();
            persist_drone_status(name, status);
        }

        notification::notify(
            &format!("Hive - {}", name),
            "Drone auto-completed (all tasks done, idle timeout)",
        );
    }
}

/// Notify when a drone appears stuck (no events for 10 min, process alive).
pub fn detect_stalled_drones(state: &mut TuiState) {
    const STALL_TIMEOUT_SECS: u64 = 600;
    for (name, status) in &state.drones {
        if !matches!(status.status, DroneState::InProgress) {
            continue;
        }
        if state.auto_stopped_drones.contains(name) {
            continue;
        }

        let (completed, total) = state.snapshot_store.progress(name);
        if total > 0 && completed >= total {
            continue;
        }

        let process_alive = read_drone_pid(name)
            .map(is_process_running)
            .unwrap_or(false);
        if !process_alive {
            continue;
        }

        let last_event = state.last_event_time.get(name).copied();
        let stalled = last_event
            .map(|t| t.elapsed() > Duration::from_secs(STALL_TIMEOUT_SECS))
            .unwrap_or(false);

        if stalled {
            let stall_key = format!("stall-{}", name);
            if !state.auto_stopped_drones.contains(&stall_key) {
                state.auto_stopped_drones.insert(stall_key);
                notification::notify(
                    &format!("Hive - {} STALLED", name),
                    "No activity for 10 min (rate limit?). Run: hive stop && hive start to restart.",
                );
            }
        }
    }
}
