pub(super) mod detection;
mod tick;

use anyhow::Result;
use ratatui::style::Color;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use crate::agent_teams::snapshot::TaskSnapshotStore;
use crate::commands::common::{elapsed_since, list_drones};
use crate::events::{EventReader, HiveEvent};
use crate::types::{DroneState, DroneStatus, Plan};

use super::cost::CostSummary;

/// A record of a completed tool call, for the tools view.
#[derive(Debug, Clone)]
pub struct ToolRecord {
    pub tool: String,
    pub tool_use_id: String,
    #[allow(dead_code)]
    pub timestamp: Instant,
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
    pub pr_state_cache: HashMap<String, (String, Instant)>,
    /// Countdown clean: (drone_name, when_started)
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
    pub fn drone_elapsed(status: &DroneStatus) -> String {
        use crate::commands::common::{duration_between, format_duration};

        match status.status {
            DroneState::Completed | DroneState::Stopped | DroneState::Zombie => {
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
mod tests;
