use crate::commands::common::list_drones;
use crate::types::{DroneState, DroneStatus};
use anyhow::Result;

/// Snapshot of drone statuses for the sidebar
pub struct DroneSnapshot {
    pub drones: Vec<(String, DroneStatus)>,
}

impl DroneSnapshot {
    /// Refresh drone statuses from disk
    pub fn refresh() -> Result<Self> {
        let mut drones = list_drones()?;
        // Sort: in_progress first, then blocked/error, then stopped, then completed
        drones.sort_by_key(|(_, status)| match status.status {
            DroneState::InProgress | DroneState::Starting | DroneState::Resuming => 0,
            DroneState::Blocked | DroneState::Error => 1,
            DroneState::Stopped => 2,
            DroneState::Completed => 3,
        });
        Ok(Self { drones })
    }

    pub fn is_empty(&self) -> bool {
        self.drones.is_empty()
    }

    pub fn len(&self) -> usize {
        self.drones.len()
    }
}
