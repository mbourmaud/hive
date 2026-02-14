use anyhow::Result;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::commands::common::{list_drones, load_prd};
use crate::events::{EventReader, HiveEvent};
use crate::notification;
use crate::types::DroneState;

use super::super::cost::parse_cost_from_log;
use super::detection;
use super::TuiState;

impl TuiState {
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

        self.process_notifications();
        self.process_events();
        self.refresh_cost_data();

        detection::detect_zombies(self);
        detection::detect_completion_markers(self);
        detection::detect_pr_completion(self);
        detection::detect_pr_merges(self);
        detection::detect_idle_drones(self);
        detection::detect_stalled_drones(self);

        self.sort_and_build_display_order();

        Ok(())
    }

    fn process_notifications(&mut self) {
        for (name, status) in &self.drones {
            let prev_progress = self.snapshot_store.progress(name);
            let snapshot = self.snapshot_store.update(name);
            let (completed_count, total_count) = snapshot.progress;
            let prev_count = prev_progress.0;
            let prev_state = self.last_drone_states.get(name).cloned();

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
    }

    fn process_events(&mut self) {
        for (name, _) in &self.drones {
            let reader = self
                .event_readers
                .entry(name.clone())
                .or_insert_with(|| EventReader::new(name));

            let new_events = reader.read_new();
            for event in new_events {
                if matches!(event, HiveEvent::Stop { .. })
                    && !self.auto_stopped_drones.contains(name)
                {
                    let _ = crate::agent_teams::auto_complete_tasks(name);
                    self.auto_stopped_drones.insert(name.clone());
                    let _ = crate::commands::kill_clean::kill_quiet(name.clone());
                }

                self.last_event_time.insert(name.clone(), Instant::now());
                self.last_events.insert(name.clone(), event);
            }
        }
    }

    fn refresh_cost_data(&mut self) {
        self.cost_refresh_counter += 1;
        if self.cost_refresh_counter >= 30 {
            self.cost_refresh_counter = 0;
            for (name, _) in &self.drones {
                self.cost_cache
                    .insert(name.clone(), parse_cost_from_log(name));
            }
        }
    }

    fn sort_and_build_display_order(&mut self) {
        use crate::commands::common::{parse_timestamp, DEFAULT_INACTIVE_THRESHOLD_SECS};
        use chrono::Utc;

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

        if !self.display_order.is_empty() && self.selected_index >= self.display_order.len() {
            self.selected_index = self.display_order.len() - 1;
        }
    }
}
