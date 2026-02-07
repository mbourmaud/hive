use anyhow::Result;
use chrono::Utc;
use ratatui::style::Color;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::commands::common::{
    elapsed_since, is_process_running, list_drones, load_prd, parse_timestamp, read_drone_pid,
    reconcile_progress_with_prd, DEFAULT_INACTIVE_THRESHOLD_SECS,
};
use crate::events::{EventReader, HiveEvent};
use crate::types::{DroneState, DroneStatus, Prd};

use super::cost::{parse_cost_from_log, CostSummary};
use super::drone_actions::{handle_resume_drone, send_desktop_notification};
use super::sparkline::update_activity_history;
use super::ViewMode;

pub(crate) struct TuiState {
    // Selection
    pub selected_index: usize,
    pub selected_story_index: Option<usize>,
    pub scroll_offset: usize,
    // Messages
    pub message: Option<String>,
    pub message_color: Color,
    // Views
    pub blocked_view: Option<String>,
    pub expanded_drones: HashSet<String>,
    pub view_mode: ViewMode,
    pub timeline_scroll: usize,
    // Tracking
    pub auto_resumed_drones: HashSet<String>,
    pub auto_stopped_drones: HashSet<String>,
    pub last_completed_counts: HashMap<String, usize>,
    pub last_drone_states: HashMap<String, DroneState>,
    // Events & data
    pub event_readers: HashMap<String, EventReader>,
    pub last_events: HashMap<String, HiveEvent>,
    pub cost_cache: HashMap<String, CostSummary>,
    pub cost_refresh_counter: u32,
    pub activity_history: HashMap<String, Vec<(std::time::Instant, u64)>>,
    // Computed per-tick
    pub drones: Vec<(String, DroneStatus)>,
    pub display_order: Vec<usize>,
    pub prd_cache: HashMap<String, Prd>,
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
                        | DroneState::Blocked
                        | DroneState::Error
                )
            })
            .map(|(name, _)| name.clone())
            .collect();

        Ok(Self {
            selected_index: 0,
            selected_story_index: None,
            scroll_offset: 0,
            message: None,
            message_color: Color::Green,
            blocked_view: None,
            expanded_drones,
            view_mode: ViewMode::Dashboard,
            timeline_scroll: 0,
            auto_resumed_drones: HashSet::new(),
            auto_stopped_drones: HashSet::new(),
            last_completed_counts: HashMap::new(),
            last_drone_states: HashMap::new(),
            event_readers: HashMap::new(),
            last_events: HashMap::new(),
            cost_cache: HashMap::new(),
            cost_refresh_counter: 0,
            activity_history: HashMap::new(),
            drones: Vec::new(),
            display_order: Vec::new(),
            prd_cache: HashMap::new(),
        })
    }

    pub fn tick(&mut self) -> Result<()> {
        self.drones = list_drones()?;

        // Desktop notifications for state changes
        for (name, status) in &self.drones {
            let completed_count = status.completed.len();
            let prev_count = self.last_completed_counts.get(name).copied().unwrap_or(0);
            let prev_state = self.last_drone_states.get(name).cloned();

            // Story completed
            if completed_count > prev_count && prev_count > 0 {
                if completed_count >= status.total && status.total > 0 {
                    send_desktop_notification(
                        &format!("Hive - {}", name),
                        &format!("Done! {}/{} stories", completed_count, status.total),
                    );
                } else {
                    send_desktop_notification(
                        &format!("Hive - {}", name),
                        &format!("Story completed ({}/{})", completed_count, status.total),
                    );
                }
            }

            // Blocked or error
            if let Some(prev) = prev_state {
                if prev != DroneState::Blocked && status.status == DroneState::Blocked {
                    let reason = status.blocked_reason.as_deref().unwrap_or("Unknown");
                    send_desktop_notification(
                        &format!("Hive - {} BLOCKED", name),
                        reason,
                    );
                }
                if prev != DroneState::Error && status.status == DroneState::Error {
                    send_desktop_notification(
                        &format!("Hive - {} ERROR", name),
                        &format!(
                            "Error in {}",
                            status.last_error_story.as_deref().unwrap_or("?")
                        ),
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

        // Update activity sparkline history
        for (name, _) in &self.drones {
            update_activity_history(&mut self.activity_history, name);
        }

        // Sort: in_progress first, then blocked, then completed
        self.drones.sort_by_key(|(_, status)| match status.status {
            DroneState::InProgress | DroneState::Starting | DroneState::Resuming => 0,
            DroneState::Blocked | DroneState::Error => 1,
            DroneState::Stopped | DroneState::Cleaning => 2,
            DroneState::Completed => 3,
        });

        // Load PRDs for story info (needed for archive calculation)
        self.prd_cache = self
            .drones
            .iter()
            .filter_map(|(_, status)| {
                let prd_path = PathBuf::from(".hive").join("prds").join(&status.prd);
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
    pub fn drone_elapsed(status: &DroneStatus) -> String {
        use crate::commands::common::{duration_between, format_duration};

        if status.status == DroneState::Completed {
            let last_completed = status
                .story_times
                .values()
                .filter_map(|t| t.completed.as_ref())
                .max();
            if let (Some(last), Some(start)) =
                (last_completed, parse_timestamp(&status.started))
            {
                if let Some(end) = parse_timestamp(last) {
                    format_duration(end.signed_duration_since(start))
                } else {
                    elapsed_since(&status.started).unwrap_or_default()
                }
            } else {
                elapsed_since(&status.started).unwrap_or_default()
            }
        } else if status.status == DroneState::Stopped {
            if let Some(duration) = duration_between(&status.started, &status.updated) {
                format_duration(duration)
            } else {
                elapsed_since(&status.started).unwrap_or_default()
            }
        } else {
            elapsed_since(&status.started).unwrap_or_default()
        }
    }
}
