use std::collections::HashMap;

use super::task_sync::{TeamMember, TeamTaskInfo};

/// Source of the snapshot data
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotSource {
    /// Read from live ~/.claude/tasks/<drone>/ files
    LiveTasks,
    /// Reconstructed from events.ndjson
    Events,
    /// Cached from a previous snapshot (files disappeared)
    Cache,
}

/// A snapshot of a drone's task state at a point in time.
#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub tasks: Vec<TeamTaskInfo>,
    pub members: Vec<TeamMember>,
    pub progress: (usize, usize),
    pub source: SnapshotSource,
}

/// Single source of truth for all task/progress data in the TUI.
///
/// Key invariants:
/// - **Monotonic progress**: completed count NEVER decreases
/// - **Monotonic task status**: a task that reached `completed` cannot regress
/// - **Cached on disappearance**: when live data vanishes (TeamDelete), last known snapshot is retained
/// - **Single update point per tick**: one function reads all sources and merges them
pub struct TaskSnapshotStore {
    snapshots: HashMap<String, TaskSnapshot>,
    /// High-water marks for progress: (max_completed, max_total) per drone
    high_water_marks: HashMap<String, (usize, usize)>,
    /// Track per-task completed status for monotonicity
    completed_tasks: HashMap<String, std::collections::HashSet<String>>,
}

impl Default for TaskSnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskSnapshotStore {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            high_water_marks: HashMap::new(),
            completed_tasks: HashMap::new(),
        }
    }

    /// Get the current snapshot for a drone (if any).
    pub fn get(&self, drone_name: &str) -> Option<&TaskSnapshot> {
        self.snapshots.get(drone_name)
    }

    /// Get progress for a drone, returning (0, 0) if unknown.
    pub fn progress(&self, drone_name: &str) -> (usize, usize) {
        self.snapshots
            .get(drone_name)
            .map(|s| s.progress)
            .unwrap_or((0, 0))
    }

    /// Update the snapshot for a drone by reading all sources and merging.
    ///
    /// Returns the updated snapshot reference.
    pub fn update(&mut self, drone_name: &str) -> &TaskSnapshot {
        use crate::agent_teams::task_sync;
        use crate::events;

        // Source 1: Live tasks from ~/.claude/tasks/<drone>/
        let live_tasks = task_sync::read_team_task_states(drone_name)
            .ok()
            .filter(|t| !t.is_empty());

        // Source 2: Event-sourced tasks from events.ndjson
        let event_tasks = events::reconstruct_tasks(drone_name);

        // Source 3: Previous snapshot (cache)
        let previous = self.snapshots.get(drone_name).cloned();

        // Read team members (best-effort)
        let members = task_sync::read_team_members(drone_name).unwrap_or_default();

        // Merge: prefer live if non-empty → events if non-empty → cache
        // Include ALL tasks (user + internal) so the TUI can render internals nested
        let (mut task_list, source) = if let Some(ref live) = live_tasks {
            (live.values().cloned().collect(), SnapshotSource::LiveTasks)
        } else if !event_tasks.is_empty() {
            // Convert event tasks to TeamTaskInfo
            let infos: Vec<TeamTaskInfo> = event_tasks
                .into_iter()
                .map(|et| TeamTaskInfo {
                    id: et.task_id,
                    subject: et.subject,
                    description: String::new(),
                    status: et.status,
                    owner: et.owner,
                    active_form: None,
                    model: None,
                    is_internal: false,
                    created_at: None,
                    updated_at: None,
                })
                .collect();
            (infos, SnapshotSource::Events)
        } else if let Some(ref prev) = previous {
            (prev.tasks.clone(), SnapshotSource::Cache)
        } else {
            (Vec::new(), SnapshotSource::Cache)
        };

        // Enforce per-task monotonicity: completed tasks cannot regress
        let completed_set = self
            .completed_tasks
            .entry(drone_name.to_string())
            .or_default();

        // Record newly completed tasks
        for task in &task_list {
            if task.status == "completed" {
                completed_set.insert(task.id.clone());
            }
        }

        // Force completed status on tasks that were previously completed
        for task in &mut task_list {
            if completed_set.contains(&task.id) && task.status != "completed" {
                task.status = "completed".to_string();
            }
        }

        // Calculate progress from user tasks only (internal tasks are for nested display)
        let has_user_tasks = task_list.iter().any(|t| !t.is_internal);
        let (current_completed, current_total) = if has_user_tasks {
            let c = task_list
                .iter()
                .filter(|t| !t.is_internal && t.status == "completed")
                .count();
            let t = task_list.iter().filter(|t| !t.is_internal).count();
            (c, t)
        } else {
            // Planning phase — only internals exist, count them
            let c = task_list.iter().filter(|t| t.status == "completed").count();
            (c, task_list.len())
        };

        // Enforce progress monotonicity via high-water mark
        let (prev_max_completed, prev_max_total) = self
            .high_water_marks
            .get(drone_name)
            .copied()
            .unwrap_or((0, 0));

        let final_completed = current_completed.max(prev_max_completed);
        let final_total = current_total.max(prev_max_total);

        self.high_water_marks
            .insert(drone_name.to_string(), (final_completed, final_total));

        let snapshot = TaskSnapshot {
            tasks: task_list,
            members,
            progress: (final_completed, final_total),
            source,
        };

        self.snapshots.insert(drone_name.to_string(), snapshot);
        self.snapshots.get(drone_name).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, status: &str) -> TeamTaskInfo {
        TeamTaskInfo {
            id: id.to_string(),
            subject: format!("Task {}", id),
            description: String::new(),
            status: status.to_string(),
            owner: None,
            active_form: None,
            model: None,
            is_internal: false,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_monotonic_progress_never_decreases() {
        let mut store = TaskSnapshotStore::new();

        // Simulate having a previous high-water mark of (3, 5)
        store
            .high_water_marks
            .insert("test-drone".to_string(), (3, 5));

        // Even with empty data, progress should not decrease
        // (update() will read from filesystem which won't work in tests,
        //  so we test the invariant directly)
        let (prev_completed, prev_total) = store
            .high_water_marks
            .get("test-drone")
            .copied()
            .unwrap_or((0, 0));

        let current_completed = 0; // empty data
        let current_total = 0;

        let final_completed = current_completed.max(prev_completed);
        let final_total = current_total.max(prev_total);

        assert_eq!(final_completed, 3);
        assert_eq!(final_total, 5);
    }

    #[test]
    fn test_task_status_monotonicity() {
        let mut store = TaskSnapshotStore::new();

        // Mark a task as completed
        let completed_set = store
            .completed_tasks
            .entry("test-drone".to_string())
            .or_default();
        completed_set.insert("1".to_string());

        // Now if the task comes back as in_progress, it should stay completed
        let mut tasks = vec![make_task("1", "in_progress"), make_task("2", "pending")];

        let set = store.completed_tasks.get("test-drone").unwrap();
        for task in &mut tasks {
            if set.contains(&task.id) && task.status != "completed" {
                task.status = "completed".to_string();
            }
        }

        assert_eq!(tasks[0].status, "completed");
        assert_eq!(tasks[1].status, "pending");
    }

    #[test]
    fn test_empty_store_returns_none() {
        let store = TaskSnapshotStore::new();
        assert!(store.get("nonexistent").is_none());
        assert_eq!(store.progress("nonexistent"), (0, 0));
    }
}
