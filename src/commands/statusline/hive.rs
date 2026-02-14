use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use super::common;
use super::{BOLD, BRIGHT_GREEN, BRIGHT_RED, GRAY, RESET, SEP, YELLOW};
use crate::types::{DroneState, DroneStatus};

pub fn build_line2(current_dir: &str) -> Option<String> {
    let hive_root = find_hive_root(current_dir)?;
    let drones = list_drones_at(&hive_root).ok()?;

    if drones.is_empty() {
        return None;
    }

    // Change CWD to hive root so common:: helpers work with relative paths
    let original_dir = std::env::current_dir().ok();
    let _ = std::env::set_current_dir(&hive_root);

    let now = chrono::Utc::now();
    let mut drone_parts: Vec<String> = Vec::new();

    for (name, status) in &drones {
        // Skip stopped/zombie/cleaning
        match status.status {
            DroneState::Stopped | DroneState::Zombie | DroneState::Cleaning => continue,
            _ => {}
        }

        // Skip completed drones older than 1 hour
        if status.status == DroneState::Completed {
            if let Some(updated_dt) = common::parse_timestamp(&status.updated) {
                let age = now.signed_duration_since(updated_dt);
                if age.num_seconds() > common::DEFAULT_INACTIVE_THRESHOLD_SECS {
                    continue;
                }
            }
        }

        let (completed, total) = common::agent_teams_progress(name);
        let elapsed = common::elapsed_since(&status.started).unwrap_or_default();

        let formatted = format_drone(name, status, completed, total, &elapsed);
        if let Some(f) = formatted {
            drone_parts.push(f);
        }
    }

    // Restore original CWD
    if let Some(orig) = original_dir {
        let _ = std::env::set_current_dir(orig);
    }

    if drone_parts.is_empty() {
        return None;
    }

    let version = env!("CARGO_PKG_VERSION");
    let prefix = format!("{YELLOW}{BOLD}\u{1F41D} hive v{version}{RESET}");
    let drones_str = drone_parts.join(SEP);
    Some(format!("{prefix}{SEP}{drones_str}"))
}

pub fn format_drone(
    name: &str,
    status: &DroneStatus,
    completed: usize,
    total: usize,
    elapsed: &str,
) -> Option<String> {
    match status.status {
        DroneState::InProgress | DroneState::Starting | DroneState::Resuming => {
            let pid_alive = common::read_drone_pid(name)
                .map(common::is_process_running)
                .unwrap_or(false);

            if pid_alive {
                Some(format!(
                    "{YELLOW}\u{1F41D} {name} {completed}/{total} {elapsed}{RESET}"
                ))
            } else {
                Some(format!(
                    "{GRAY}\u{1F41D} {name} \u{23F8} {completed}/{total} {elapsed}{RESET}"
                ))
            }
        }
        DroneState::Completed => Some(format!(
            "{YELLOW}\u{1F41D} {name}{RESET} {BRIGHT_GREEN}\u{2713}{RESET} {completed}/{total} {elapsed}"
        )),
        DroneState::Error => Some(format!(
            "{YELLOW}\u{1F41D} {name}{RESET} {BRIGHT_RED}\u{2717}{RESET} {completed}/{total}"
        )),
        _ => None,
    }
}

pub fn find_hive_root(start_dir: &str) -> Option<PathBuf> {
    let mut dir = PathBuf::from(start_dir);
    loop {
        let hive_drones = dir.join(".hive").join("drones");
        if hive_drones.is_dir() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn list_drones_at(hive_root: &Path) -> Result<Vec<(String, DroneStatus)>> {
    let drones_dir = hive_root.join(".hive").join("drones");

    if !drones_dir.exists() {
        return Ok(Vec::new());
    }

    let mut drones = Vec::new();

    for entry in fs::read_dir(&drones_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let drone_name = entry.file_name().to_string_lossy().into_owned();
        let status_path = entry.path().join("status.json");

        if status_path.exists() {
            if let Ok(contents) = fs::read_to_string(&status_path) {
                if let Ok(status) = serde_json::from_str::<DroneStatus>(&contents) {
                    drones.push((drone_name, status));
                }
            }
        }
    }

    drones.sort_by(|a, b| b.1.updated.cmp(&a.1.updated));

    Ok(drones)
}
