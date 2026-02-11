use anyhow::Result;
use chrono::Utc;
use ratatui::style::Color;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::agent_teams::snapshot::TaskSnapshotStore;
use crate::commands::common::{
    command_with_timeout, elapsed_since, is_process_running, list_drones, load_prd,
    parse_timestamp, read_drone_pid, DEFAULT_INACTIVE_THRESHOLD_SECS,
};
use crate::events::{EventReader, HiveEvent};
use crate::notification;
use crate::types::{DroneState, DroneStatus, Plan};

use super::cost::{parse_cost_from_log, CostSummary};

/// A record of a completed tool call, for the tools view.
#[derive(Debug, Clone)]
pub struct ToolRecord {
    pub tool: String,
    pub tool_use_id: String,
    #[allow(dead_code)]
    pub timestamp: Instant,
}

/// Max tool records per drone (ring buffer)
const MAX_TOOL_RECORDS: usize = 50;

/// Check PR state with timeout. Returns true if PR is in expected state.
/// Uses a cache reference to avoid redundant gh calls (cache entries valid for 60s).
fn check_pr_state(
    cache: &HashMap<String, (String, Instant)>,
    branch: &str,
    expected_state: &str,
) -> bool {
    // Check cache first (valid for 60 seconds)
    if let Some((cached_state, when)) = cache.get(branch) {
        if when.elapsed() < Duration::from_secs(60) {
            return cached_state == expected_state;
        }
    }

    // Query gh with timeout (5 seconds)
    let mut cmd = std::process::Command::new("gh");
    cmd.args(["pr", "view", branch, "--json", "state", "-q", ".state"]);

    let result = command_with_timeout(&mut cmd, 5)
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    result == expected_state
}

pub(crate) struct TuiState {
    // Selection
    pub selected_index: usize,
    pub scroll_offset: usize,
    // Messages
    pub message: Option<String>,
    pub message_color: Color,
    pub message_time: Option<Instant>,
    // Views
    pub messages_view: Option<String>,
    pub messages_scroll: usize,
    pub messages_selected_index: usize,
    pub expanded_drones: HashSet<String>,
    // Tracking
    pub auto_stopped_drones: HashSet<String>,
    pub last_drone_states: HashMap<String, DroneState>,
    // Events & data
    pub event_readers: HashMap<String, EventReader>,
    pub last_events: HashMap<String, HiveEvent>,
    pub cost_cache: HashMap<String, CostSummary>,
    pub cost_refresh_counter: u32,
    pub merge_check_counter: u32,
    pub pr_completion_check_counter: u32,
    /// Tracks when we first detected a zombie drone
    pub zombie_first_seen: HashMap<String, Instant>,
    // Computed per-tick
    pub drones: Vec<(String, DroneStatus)>,
    pub display_order: Vec<usize>,
    pub plan_cache: HashMap<String, Plan>,
    /// Single source of truth for task/progress data (monotonic, cached on disappearance)
    pub snapshot_store: TaskSnapshotStore,
    /// Cached PR state to avoid calling gh on every tick cycle
    pub pr_state_cache: HashMap<String, (String, Instant)>, // branch -> (state, when_checked)
    /// Countdown clean: (drone_name, when_started, countdown_secs)
    pub pending_clean: Option<(String, Instant)>,
    /// Tracks when all tasks were first detected as completed for idle auto-stop (#58)
    pub all_tasks_done_since: HashMap<String, Instant>,
    /// Tracks the last time a new event was received per drone (#58)
    pub last_event_time: HashMap<String, Instant>,
    /// Per-drone tool call history from PostToolUse events
    pub tool_history: HashMap<String, VecDeque<ToolRecord>>,
    /// Full-screen tools view: Some(drone_name) when active
    pub tools_view: Option<String>,
    /// TTL cache for list_drones() — only re-read every second
    pub drones_cache_time: Option<Instant>,
}

impl TuiState {
    pub fn new() -> Result<Self> {
        let initial_drones = list_drones()?;
        let expanded_drones: HashSet<String> = initial_drones
            .iter()
            .filter(|(_, status)| {
                matches!(
                    status.status,
                    DroneState::InProgress
                        | DroneState::Starting
                        | DroneState::Resuming
                        | DroneState::Error
                )
            })
            .map(|(name, _)| name.clone())
            .collect();

        let state = Self {
            selected_index: 0,
            scroll_offset: 0,
            message: None,
            message_color: Color::Green,
            message_time: None,
            messages_view: None,
            messages_scroll: 0,
            messages_selected_index: usize::MAX,
            expanded_drones,
            auto_stopped_drones: HashSet::new(),
            last_drone_states: HashMap::new(),
            event_readers: HashMap::new(),
            last_events: HashMap::new(),
            cost_cache: HashMap::new(),
            cost_refresh_counter: 29,
            merge_check_counter: 0,
            pr_completion_check_counter: 0,
            zombie_first_seen: HashMap::new(),
            drones: Vec::new(),
            display_order: Vec::new(),
            plan_cache: HashMap::new(),
            snapshot_store: TaskSnapshotStore::new(),
            pr_state_cache: HashMap::new(),
            pending_clean: None,
            all_tasks_done_since: HashMap::new(),
            last_event_time: HashMap::new(),
            tool_history: HashMap::new(),
            tools_view: None,
            drones_cache_time: None,
        };

        Ok(state)
    }

    pub fn tick(&mut self) -> Result<()> {
        // TTL cache: only re-read drones from disk every second
        let stale = self
            .drones_cache_time
            .map(|t| t.elapsed() > Duration::from_secs(1))
            .unwrap_or(true);
        if stale {
            self.drones = list_drones()?;
            self.drones_cache_time = Some(Instant::now());
        }

        // Desktop notifications for state changes
        for (name, status) in &self.drones {
            // Get progress from snapshot store (single source of truth, monotonic)
            let prev_progress = self.snapshot_store.progress(name);
            let snapshot = self.snapshot_store.update(name);
            let (completed_count, total_count) = snapshot.progress;
            let prev_count = prev_progress.0;
            let prev_state = self.last_drone_states.get(name).cloned();

            // Task completed
            if completed_count > prev_count && prev_count > 0 {
                if completed_count >= total_count && total_count > 0 {
                    notification::notify(
                        &format!("Hive - {}", name),
                        &format!("Done! {}/{} tasks", completed_count, total_count),
                    );
                } else {
                    notification::notify(
                        &format!("Hive - {}", name),
                        &format!("Task completed ({}/{})", completed_count, total_count),
                    );
                }
            }

            // Error notification
            if let Some(prev) = prev_state {
                if prev != DroneState::Error && status.status == DroneState::Error {
                    notification::notify(
                        &format!("Hive - {} ERROR", name),
                        &format!("Error in {}", status.last_error.as_deref().unwrap_or("?")),
                    );
                }
            }

            self.last_drone_states
                .insert(name.clone(), status.status.clone());
        }

        // Read new events from hooks (incremental ndjson tailing)
        for (name, _) in &self.drones {
            let reader = self
                .event_readers
                .entry(name.clone())
                .or_insert_with(|| EventReader::new(name));

            let new_events = reader.read_new();
            for event in new_events {
                // Auto-stop on Stop event
                if matches!(event, HiveEvent::Stop { .. })
                    && !self.auto_stopped_drones.contains(name)
                {
                    // Auto-complete in_progress tasks before stopping (#56)
                    let _ = crate::agent_teams::auto_complete_tasks(name);
                    self.auto_stopped_drones.insert(name.clone());
                    let _ = crate::commands::kill_clean::kill_quiet(name.clone());
                }

                // Capture ToolDone events into tool_history ring buffer
                if let HiveEvent::ToolDone {
                    ref tool,
                    ref tool_use_id,
                    ..
                } = event
                {
                    let history = self.tool_history.entry(name.clone()).or_default();
                    if history.len() >= MAX_TOOL_RECORDS {
                        history.pop_front();
                    }
                    history.push_back(ToolRecord {
                        tool: tool.clone(),
                        tool_use_id: tool_use_id.clone().unwrap_or_default(),
                        timestamp: Instant::now(),
                    });
                }

                self.last_event_time.insert(name.clone(), Instant::now());
                self.last_events.insert(name.clone(), event);
            }
        }

        // Refresh cost data every ~30 ticks (3 seconds at 100ms poll)
        self.cost_refresh_counter += 1;
        if self.cost_refresh_counter >= 30 {
            self.cost_refresh_counter = 0;
            for (name, _) in &self.drones {
                self.cost_cache
                    .insert(name.clone(), parse_cost_from_log(name));
            }
        }

        // Zombie detection: mark drones whose process died but status is still active
        // Grace period: don't mark as zombie if a Stop event exists (graceful exit)
        let now_utc = Utc::now();
        for (name, status) in &mut self.drones {
            if matches!(
                status.status,
                DroneState::InProgress | DroneState::Starting | DroneState::Resuming
            ) {
                // Startup grace period: skip zombie detection for recently started/resumed drones
                // The PID file may not be written yet during the first few seconds
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
                    // Auto-complete in_progress tasks when process dies (#56)
                    let _ = crate::agent_teams::auto_complete_tasks(name);

                    // Check if there's a Stop event — if so, the drone exited gracefully
                    if crate::events::has_stop_event(name) {
                        // Graceful exit — mark as Completed or Stopped, not Zombie
                        status.status = DroneState::Stopped;
                    } else {
                        status.status = DroneState::Zombie;
                        // Record first-seen timestamp for zombie age display
                        self.zombie_first_seen
                            .entry(name.clone())
                            .or_insert_with(Instant::now);
                    }
                    let status_path = PathBuf::from(".hive/drones").join(name).join("status.json");
                    let _ = fs::write(
                        &status_path,
                        serde_json::to_string_pretty(&*status).unwrap_or_default(),
                    );
                }
            }
        }

        // Clean up zombie_first_seen entries for drones that are no longer zombie
        self.zombie_first_seen.retain(|name, _| {
            self.drones
                .iter()
                .any(|(n, s)| n == name && s.status == DroneState::Zombie)
        });

        // Completion marker detection: check for .hive_complete file
        for (name, status) in &mut self.drones {
            if matches!(
                status.status,
                DroneState::InProgress | DroneState::Starting | DroneState::Resuming
            ) {
                let marker = PathBuf::from(&status.worktree).join(".hive_complete");
                if marker.exists() {
                    // Mark as completed
                    status.status = DroneState::Completed;
                    status.updated = Utc::now().to_rfc3339();

                    // Update status.json
                    let status_path = PathBuf::from(".hive/drones")
                        .join(&**name)
                        .join("status.json");
                    let _ = fs::write(
                        &status_path,
                        serde_json::to_string_pretty(&*status).unwrap_or_default(),
                    );

                    // Kill the process
                    let _ = crate::commands::kill_clean::kill_quiet(name.to_string());

                    // Clean up marker
                    let _ = fs::remove_file(&marker);

                    // Notification
                    notification::notify(&format!("Hive - {}", name), "Drone completed!");
                }
            }
        }

        // PR-based completion detection (fallback): every ~300 ticks (30 seconds)
        // Check if InProgress drones have created a PR
        self.pr_completion_check_counter += 1;
        if self.pr_completion_check_counter >= 300 {
            self.pr_completion_check_counter = 0;

            // Collect candidates first to avoid borrow issues
            let candidates: Vec<(String, String)> = self
                .drones
                .iter()
                .filter(|(_, s)| matches!(s.status, DroneState::InProgress))
                .map(|(name, s)| (name.clone(), s.branch.clone()))
                .collect();

            for (name, branch) in candidates {
                let pr_open = check_pr_state(&self.pr_state_cache, &branch, "OPEN");
                if pr_open {
                    let (completed, total) = self.snapshot_store.progress(&name);
                    let all_tasks_done = total > 0 && completed >= total;

                    if all_tasks_done {
                        // Find and update the drone status
                        if let Some((_, status)) = self.drones.iter_mut().find(|(n, _)| n == &name)
                        {
                            status.status = DroneState::Completed;
                            status.updated = Utc::now().to_rfc3339();

                            let status_path = PathBuf::from(".hive/drones")
                                .join(&name)
                                .join("status.json");
                            let _ = fs::write(
                                &status_path,
                                serde_json::to_string_pretty(&*status).unwrap_or_default(),
                            );
                        }

                        let _ = crate::commands::kill_clean::kill_quiet(name.to_string());
                        notification::notify(
                            &format!("Hive - {}", name),
                            "Drone completed (PR created)!",
                        );
                    }
                }
            }
        }

        // PR merge detection: every ~600 ticks (60 seconds), check completed/stopped drones
        self.merge_check_counter += 1;
        if self.merge_check_counter >= 600 {
            self.merge_check_counter = 0;

            let merged: Vec<String> = self
                .drones
                .iter()
                .filter(|(_, s)| matches!(s.status, DroneState::Completed | DroneState::Stopped))
                .filter(|(_, s)| check_pr_state(&self.pr_state_cache, &s.branch, "MERGED"))
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

        // Idle detection: auto-stop drones that have all tasks done and no new events (#58)
        // If all tasks completed + no new events for 2 minutes → auto-complete the drone
        const IDLE_TIMEOUT_SECS: u64 = 120;
        let idle_candidates: Vec<String> = self
            .drones
            .iter()
            .filter(|(_, s)| matches!(s.status, DroneState::InProgress))
            .filter(|(name, _)| !self.auto_stopped_drones.contains(name))
            .filter_map(|(name, _)| {
                let (completed, total) = self.snapshot_store.progress(name);
                if total > 0 && completed >= total {
                    // Track when we first saw all tasks done
                    let first_seen = self
                        .all_tasks_done_since
                        .entry(name.clone())
                        .or_insert_with(Instant::now);
                    // Also check no new events recently
                    let last_event = self.last_event_time.get(name).copied();
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
                    // Tasks not all done — remove from tracking
                    self.all_tasks_done_since.remove(name);
                    None
                }
            })
            .collect();

        for name in &idle_candidates {
            let _ = crate::agent_teams::auto_complete_tasks(name);
            self.auto_stopped_drones.insert(name.clone());
            let _ = crate::commands::kill_clean::kill_quiet(name.clone());

            if let Some((_, status)) = self.drones.iter_mut().find(|(n, _)| n == name) {
                status.status = DroneState::Completed;
                status.updated = Utc::now().to_rfc3339();

                let status_path = PathBuf::from(".hive/drones").join(name).join("status.json");
                let _ = fs::write(
                    &status_path,
                    serde_json::to_string_pretty(&*status).unwrap_or_default(),
                );
            }

            notification::notify(
                &format!("Hive - {}", name),
                "Drone auto-completed (all tasks done, idle timeout)",
            );
        }

        // Stall detection: notify user when a drone appears stuck (rate limit, etc.) (#61)
        // If InProgress + tasks NOT all done + no events for 10 minutes + process alive → stalled
        const STALL_TIMEOUT_SECS: u64 = 600; // 10 minutes
        for (name, status) in &self.drones {
            if !matches!(status.status, DroneState::InProgress) {
                continue;
            }
            if self.auto_stopped_drones.contains(name) {
                continue;
            }

            let (completed, total) = self.snapshot_store.progress(name);
            if total > 0 && completed >= total {
                continue; // all done, idle detection handles this
            }

            let process_alive = read_drone_pid(name)
                .map(is_process_running)
                .unwrap_or(false);
            if !process_alive {
                continue; // zombie detection handles dead processes
            }

            let last_event = self.last_event_time.get(name).copied();
            let stalled = last_event
                .map(|t| t.elapsed() > Duration::from_secs(STALL_TIMEOUT_SECS))
                .unwrap_or(false);

            if stalled {
                // Only notify once — check if we already set the message
                let stall_key = format!("stall-{}", name);
                if !self.auto_stopped_drones.contains(&stall_key) {
                    self.auto_stopped_drones.insert(stall_key);
                    notification::notify(
                        &format!("Hive - {} STALLED", name),
                        "No activity for 10 min (rate limit?). Run: hive stop && hive start to restart.",
                    );
                }
            }
        }

        // Sort: in_progress first, then blocked, then completed
        self.drones.sort_by_key(|(_, status)| match status.status {
            DroneState::InProgress | DroneState::Starting | DroneState::Resuming => 0,
            DroneState::Error => 1,
            DroneState::Zombie | DroneState::Stopped | DroneState::Cleaning => 2,
            DroneState::Completed => 3,
        });

        // Load plans for archive calculation
        self.plan_cache = self
            .drones
            .iter()
            .filter_map(|(_, status)| {
                let prd_path = PathBuf::from(".hive").join("plans").join(&status.prd);
                load_prd(&prd_path).map(|prd| (status.prd.clone(), prd))
            })
            .collect();

        // Build display order: active drones first, then archived
        let now = Utc::now();
        self.display_order.clear();
        let mut archived_order: Vec<usize> = Vec::new();

        for (idx, (name, status)) in self.drones.iter().enumerate() {
            if status.status == DroneState::Completed {
                let (valid_completed, task_count) = self.snapshot_store.progress(name);

                if valid_completed >= task_count {
                    let inactive_secs = parse_timestamp(&status.updated)
                        .map(|updated| now.signed_duration_since(updated).num_seconds())
                        .unwrap_or(0);

                    if inactive_secs >= DEFAULT_INACTIVE_THRESHOLD_SECS {
                        archived_order.push(idx);
                        continue;
                    }
                }
            }
            self.display_order.push(idx);
        }
        self.display_order.extend(archived_order);

        // Clamp selected index to display order
        if !self.display_order.is_empty() && self.selected_index >= self.display_order.len() {
            self.selected_index = self.display_order.len() - 1;
        }

        // Auto-resume removed (no stories in plan mode)

        Ok(())
    }

    pub fn set_message(&mut self, msg: String, color: Color) {
        self.message = Some(msg);
        self.message_color = color;
        self.message_time = Some(Instant::now());
    }

    pub fn clear_message(&mut self) {
        if let Some(when) = self.message_time {
            if when.elapsed() > Duration::from_secs(3) {
                self.message = None;
                self.message_time = None;
            }
        }
    }

    /// Helper: get the elapsed time display string for a drone.
    ///
    /// For completed/stopped drones, shows the duration between start and completion.
    /// For running drones, shows elapsed time since start.
    ///
    /// Returns "?" if the timestamp cannot be parsed (defensive fallback).
    pub fn drone_elapsed(status: &DroneStatus) -> String {
        use crate::commands::common::{duration_between, format_duration};

        match status.status {
            DroneState::Completed | DroneState::Stopped | DroneState::Zombie => {
                // Use duration between start and last update (completion/stop time)
                duration_between(&status.started, &status.updated)
                    .map(format_duration)
                    .unwrap_or_else(|| {
                        elapsed_since(&status.started).unwrap_or_else(|| "?".to_string())
                    })
            }
            _ => elapsed_since(&status.started).unwrap_or_else(|| "?".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DroneState, DroneStatus, ExecutionMode};
    use std::collections::HashMap;

    #[test]
    fn test_drone_elapsed_with_running_drone() {
        // Create a status with a timestamp 5 minutes ago
        let five_mins_ago = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
        let status = DroneStatus {
            drone: "test-drone".to_string(),
            prd: "test.json".to_string(),
            branch: "test-branch".to_string(),
            worktree: "/tmp/test".to_string(),
            local_mode: false,
            execution_mode: ExecutionMode::AgentTeam,
            backend: "agent_team".to_string(),
            status: DroneState::InProgress,
            current_task: None,
            completed: vec![],
            story_times: HashMap::new(),
            total: 0,
            started: five_mins_ago.clone(),
            updated: chrono::Utc::now().to_rfc3339(),
            error_count: 0,
            last_error: None,
            lead_model: None,
            active_agents: HashMap::new(),
        };

        let elapsed = TuiState::drone_elapsed(&status);
        // Should show something like "5m 0s" or similar
        assert!(!elapsed.is_empty(), "Elapsed time should not be empty");
        assert!(
            elapsed.contains("m") || elapsed.contains("s"),
            "Should format as time string, got: {}",
            elapsed
        );
    }

    #[test]
    fn test_drone_elapsed_with_completed_drone() {
        // Create a status that started 10 minutes ago and completed 2 minutes ago
        let ten_mins_ago = (chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
        let two_mins_ago = (chrono::Utc::now() - chrono::Duration::minutes(2)).to_rfc3339();

        let status = DroneStatus {
            drone: "test-drone".to_string(),
            prd: "test.json".to_string(),
            branch: "test-branch".to_string(),
            worktree: "/tmp/test".to_string(),
            local_mode: false,
            execution_mode: ExecutionMode::AgentTeam,
            backend: "agent_team".to_string(),
            status: DroneState::Completed,
            current_task: None,
            completed: vec![],
            story_times: HashMap::new(),
            total: 0,
            started: ten_mins_ago,
            updated: two_mins_ago,
            error_count: 0,
            last_error: None,
            lead_model: None,
            active_agents: HashMap::new(),
        };

        let elapsed = TuiState::drone_elapsed(&status);
        // Should show ~8 minutes (difference between started and updated)
        assert!(!elapsed.is_empty(), "Elapsed time should not be empty");
        assert!(
            elapsed.contains("m") || elapsed.contains("s"),
            "Should format as time string, got: {}",
            elapsed
        );
    }

    #[test]
    fn test_drone_elapsed_with_invalid_timestamp() {
        // Test with an invalid timestamp
        let status = DroneStatus {
            drone: "test-drone".to_string(),
            prd: "test.json".to_string(),
            branch: "test-branch".to_string(),
            worktree: "/tmp/test".to_string(),
            local_mode: false,
            execution_mode: ExecutionMode::AgentTeam,
            backend: "agent_team".to_string(),
            status: DroneState::InProgress,
            current_task: None,
            completed: vec![],
            story_times: HashMap::new(),
            total: 0,
            started: "not-a-valid-timestamp".to_string(),
            updated: chrono::Utc::now().to_rfc3339(),
            error_count: 0,
            last_error: None,
            lead_model: None,
            active_agents: HashMap::new(),
        };

        let elapsed = TuiState::drone_elapsed(&status);
        // Should return "?" as fallback for unparseable timestamp
        assert_eq!(elapsed, "?", "Should return '?' for invalid timestamp");
    }

    #[test]
    fn test_drone_elapsed_with_empty_timestamp() {
        // Test with an empty timestamp
        let status = DroneStatus {
            drone: "test-drone".to_string(),
            prd: "test.json".to_string(),
            branch: "test-branch".to_string(),
            worktree: "/tmp/test".to_string(),
            local_mode: false,
            execution_mode: ExecutionMode::AgentTeam,
            backend: "agent_team".to_string(),
            status: DroneState::InProgress,
            current_task: None,
            completed: vec![],
            story_times: HashMap::new(),
            total: 0,
            started: "".to_string(),
            updated: chrono::Utc::now().to_rfc3339(),
            error_count: 0,
            last_error: None,
            lead_model: None,
            active_agents: HashMap::new(),
        };

        let elapsed = TuiState::drone_elapsed(&status);
        // Should return "?" as fallback for empty timestamp
        assert_eq!(elapsed, "?", "Should return '?' for empty timestamp");
    }
}
