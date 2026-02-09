use anyhow::Result;
use chrono::Utc;
use ratatui::style::Color;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::commands::common::{
    elapsed_since, is_pr_merged, is_pr_open, is_process_running, list_drones, load_prd,
    parse_timestamp, read_drone_pid, reconcile_progress, reconcile_progress_with_prd,
    DEFAULT_INACTIVE_THRESHOLD_SECS,
};
use crate::events::{EventReader, HiveEvent};
use crate::notification;
use crate::types::{DroneState, DroneStatus, Plan};

use super::cost::{parse_cost_from_log, CostSummary};

pub(crate) struct TuiState {
    // Selection
    pub selected_index: usize,
    pub scroll_offset: usize,
    // Messages
    pub message: Option<String>,
    pub message_color: Color,
    // Views
    pub messages_view: Option<String>,
    pub messages_scroll: usize,
    pub messages_selected_index: usize,
    pub expanded_drones: HashSet<String>,
    // Tracking
    pub _auto_resumed_drones: HashSet<String>,
    pub auto_stopped_drones: HashSet<String>,
    pub last_completed_counts: HashMap<String, usize>,
    pub last_drone_states: HashMap<String, DroneState>,
    // Events & data
    pub event_readers: HashMap<String, EventReader>,
    pub last_events: HashMap<String, HiveEvent>,
    pub cost_cache: HashMap<String, CostSummary>,
    pub cost_refresh_counter: u32,
    pub merge_check_counter: u32,
    pub pr_completion_check_counter: u32,
    /// Tracks when we first saw a task as in_progress: (drone_name, task_id) -> Instant
    pub task_start_times: HashMap<(String, String), Instant>,
    /// Tracks when we first detected a zombie drone
    pub zombie_first_seen: HashMap<String, Instant>,
    // Computed per-tick
    pub drones: Vec<(String, DroneStatus)>,
    pub display_order: Vec<usize>,
    pub prd_cache: HashMap<String, Plan>,
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

        Ok(Self {
            selected_index: 0,
            scroll_offset: 0,
            message: None,
            message_color: Color::Green,
            messages_view: None,
            messages_scroll: 0,
            messages_selected_index: usize::MAX,
            expanded_drones,
            _auto_resumed_drones: HashSet::new(),
            auto_stopped_drones: HashSet::new(),
            last_completed_counts: HashMap::new(),
            last_drone_states: HashMap::new(),
            event_readers: HashMap::new(),
            last_events: HashMap::new(),
            cost_cache: HashMap::new(),
            cost_refresh_counter: 0,
            merge_check_counter: 0,
            pr_completion_check_counter: 0,
            task_start_times: HashMap::new(),
            zombie_first_seen: HashMap::new(),
            drones: Vec::new(),
            display_order: Vec::new(),
            prd_cache: HashMap::new(),
        })
    }

    pub fn tick(&mut self) -> Result<()> {
        self.drones = list_drones()?;

        // Desktop notifications for state changes
        for (name, status) in &self.drones {
            // Get live progress from ~/.claude/tasks/<drone>/ instead of stale status.json
            let (completed_count, total_count) = reconcile_progress(status);
            let prev_count = self.last_completed_counts.get(name).copied().unwrap_or(0);
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

            self.last_completed_counts
                .insert(name.clone(), completed_count);
            self.last_drone_states
                .insert(name.clone(), status.status.clone());
        }

        // Read new events from hooks (incremental ndjson tailing)
        for (name, _status) in &self.drones {
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
                    self.auto_stopped_drones.insert(name.clone());
                    let _ = crate::commands::kill_clean::kill_quiet(name.clone());
                }

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
        for (name, status) in &mut self.drones {
            if matches!(
                status.status,
                DroneState::InProgress | DroneState::Starting | DroneState::Resuming
            ) {
                let pid_alive = read_drone_pid(name)
                    .map(is_process_running)
                    .unwrap_or(false);
                if !pid_alive {
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

            for (name, status) in &mut self.drones {
                if matches!(status.status, DroneState::InProgress) {
                    // Check if PR exists and is open
                    if is_pr_open(&status.branch) {
                        // Check if all tasks are completed by reading tasks.json
                        let tasks_path = PathBuf::from(format!(
                            "{}/.claude/tasks/{}/tasks.json",
                            std::env::var("HOME").unwrap_or_default(),
                            name
                        ));

                        let all_tasks_done = if tasks_path.exists() {
                            fs::read_to_string(&tasks_path)
                                .ok()
                                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                                .and_then(|v| v.get("tasks").and_then(|t| t.as_array()).cloned())
                                .map(|tasks| {
                                    tasks.iter().all(|t| {
                                        t.get("status")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s == "completed")
                                            .unwrap_or(false)
                                    })
                                })
                                .unwrap_or(false)
                        } else {
                            false
                        };

                        if all_tasks_done {
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

                            // Notification
                            notification::notify(
                                &format!("Hive - {}", name),
                                "Drone completed (PR created)!",
                            );
                        }
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
                .filter(|(_, s)| is_pr_merged(&s.branch))
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

        // Sort: in_progress first, then blocked, then completed
        self.drones.sort_by_key(|(_, status)| match status.status {
            DroneState::InProgress | DroneState::Starting | DroneState::Resuming => 0,
            DroneState::Error => 1,
            DroneState::Zombie | DroneState::Stopped | DroneState::Cleaning => 2,
            DroneState::Completed => 3,
        });

        // Load PRDs for story info (needed for archive calculation)
        self.prd_cache = self
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

        for (idx, (_, status)) in self.drones.iter().enumerate() {
            if status.status == DroneState::Completed {
                let (valid_completed, prd_story_count) = self
                    .prd_cache
                    .get(&status.prd)
                    .map(|prd| reconcile_progress_with_prd(status, prd))
                    .unwrap_or((status.completed.len(), status.total));

                if valid_completed >= prd_story_count {
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

    pub fn clear_message(&mut self) {
        if self.message.is_some() {
            self.message = None;
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

        if status.status == DroneState::Completed {
            // Use updated timestamp as completion time
            if let Some(duration) = duration_between(&status.started, &status.updated) {
                format_duration(duration)
            } else {
                elapsed_since(&status.started).unwrap_or_else(|| "?".to_string())
            }
        } else if matches!(status.status, DroneState::Stopped | DroneState::Zombie) {
            if let Some(duration) = duration_between(&status.started, &status.updated) {
                format_duration(duration)
            } else {
                elapsed_since(&status.started).unwrap_or_else(|| "?".to_string())
            }
        } else {
            elapsed_since(&status.started).unwrap_or_else(|| "?".to_string())
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
            active_agents: HashMap::new(),
        };

        let elapsed = TuiState::drone_elapsed(&status);
        // Should return "?" as fallback for empty timestamp
        assert_eq!(elapsed, "?", "Should return '?' for empty timestamp");
    }
}
