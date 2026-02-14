mod loading;
mod persistence;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::read_task_list_safe;
use super::task_sync::{TeamMember, TeamTaskInfo};

pub(crate) use loading::{load_todos, map_task};
pub(crate) use persistence::{
    load_persisted_snapshot, load_persisted_snapshot_at, persist_snapshot, persist_snapshot_at,
};

// Re-export for tests (tests.rs uses `use super::*`)
#[cfg(test)]
pub(crate) use super::AgentTeamTask;

/// Source of the snapshot data
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotSource {
    /// Read from ~/.claude/tasks/ filesystem (live)
    Tasks,
    /// Loaded from .hive/drones/<name>/tasks-snapshot.json (persisted)
    Persisted,
}

/// Info about an agent spawned by the team lead.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub name: String,
    pub model: Option<String>,
    pub active: bool,
}

/// A snapshot of a drone's task state at a point in time.
#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub tasks: Vec<TeamTaskInfo>,
    pub members: Vec<TeamMember>,
    pub agents: Vec<AgentInfo>,
    pub progress: (usize, usize),
    pub source: SnapshotSource,
}

/// Single source of truth for all task/progress data in the TUI.
///
/// Reads task state directly from `~/.claude/tasks/<team>/*.json` (filesystem).
/// Every live read is persisted to `.hive/drones/<name>/tasks-snapshot.json`
/// so that task history survives team cleanup (e.g. `hive stop`).
///
/// Key invariants:
/// - **Monotonic progress**: completed count NEVER decreases
/// - **Monotonic task status**: a task that reached `completed` cannot regress
/// - **Single update point per tick**: one function reads all sources and merges them
pub struct TaskSnapshotStore {
    snapshots: HashMap<String, TaskSnapshot>,
    /// High-water marks for progress: (max_completed, max_total) per drone
    high_water_marks: HashMap<String, (usize, usize)>,
    /// Track per-task completed status for monotonicity
    completed_tasks: HashMap<String, HashSet<String>>,
    /// Optional project root for absolute-path operations (multi-project WebUI)
    project_root: Option<PathBuf>,
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
            project_root: None,
        }
    }

    /// Create a new store with a specific project root (for multi-project WebUI).
    pub fn with_project_root(project_root: PathBuf) -> Self {
        Self {
            snapshots: HashMap::new(),
            high_water_marks: HashMap::new(),
            completed_tasks: HashMap::new(),
            project_root: Some(project_root),
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

    /// Update the snapshot for a drone.
    ///
    /// Priority:
    /// 1. `todos.json` (from TodoWrite hook) — real task names and progress
    /// 2. Live filesystem (`~/.claude/tasks/<team>/*.json`) — agent tracking only
    /// 3. Persisted snapshot (`.hive/drones/<name>/tasks-snapshot.json`)
    ///
    /// When live data is found, persist it to disk for future recovery.
    ///
    /// Returns the updated snapshot reference.
    pub fn update(&mut self, drone_name: &str) -> &TaskSnapshot {
        use crate::agent_teams::task_sync;

        let members = task_sync::read_team_members(drone_name).unwrap_or_default();

        let (task_list, members, source) = self.load_task_data(drone_name, members);
        let mut task_list = task_list;

        // Enforce per-task monotonicity: completed tasks cannot regress
        self.enforce_task_monotonicity(drone_name, &mut task_list);

        // Calculate and enforce progress monotonicity
        let progress = self.enforce_progress_monotonicity(drone_name, &task_list);

        // Build agents from team config members
        let agents: Vec<AgentInfo> = members
            .iter()
            .map(|m| AgentInfo {
                name: m.name.clone(),
                model: if m.model.is_empty() {
                    None
                } else {
                    Some(m.model.clone())
                },
                active: true,
            })
            .collect();

        let snapshot = TaskSnapshot {
            tasks: task_list,
            members,
            agents,
            progress,
            source,
        };

        self.snapshots.insert(drone_name.to_string(), snapshot);
        self.snapshots.get(drone_name).unwrap()
    }

    fn load_task_data(
        &self,
        drone_name: &str,
        members: Vec<TeamMember>,
    ) -> (Vec<TeamTaskInfo>, Vec<TeamMember>, SnapshotSource) {
        // Try todos.json first (written by TodoWrite hook — has real task names)
        let todos_path = if let Some(ref root) = self.project_root {
            root.join(".hive/drones")
                .join(drone_name)
                .join("todos.json")
        } else {
            PathBuf::from(".hive/drones")
                .join(drone_name)
                .join("todos.json")
        };

        let todo_tasks = load_todos(&todos_path);
        let fs_tasks = read_task_list_safe(drone_name);

        if !todo_tasks.is_empty() {
            // TodoWrite gives us real task names — prefer it
            self.persist_data(drone_name, &todo_tasks, &members);
            (todo_tasks, members, SnapshotSource::Tasks)
        } else if !fs_tasks.is_empty() {
            // Fall back to Agent Teams task files (just agent names)
            let infos: Vec<TeamTaskInfo> = fs_tasks.into_iter().map(map_task).collect();
            self.persist_data(drone_name, &infos, &members);
            (infos, members, SnapshotSource::Tasks)
        } else {
            // Live files gone — load from persisted snapshot
            let loaded = if let Some(ref root) = self.project_root {
                load_persisted_snapshot_at(root, drone_name)
            } else {
                load_persisted_snapshot(drone_name)
            };
            match loaded {
                Some((tasks, persisted_members)) => {
                    (tasks, persisted_members, SnapshotSource::Persisted)
                }
                None => (Vec::new(), Vec::new(), SnapshotSource::Persisted),
            }
        }
    }

    fn persist_data(&self, drone_name: &str, tasks: &[TeamTaskInfo], members: &[TeamMember]) {
        if let Some(ref root) = self.project_root {
            persist_snapshot_at(root, drone_name, tasks, members);
        } else {
            persist_snapshot(drone_name, tasks, members);
        }
    }

    fn enforce_task_monotonicity(&mut self, drone_name: &str, task_list: &mut [TeamTaskInfo]) {
        let completed_set = self
            .completed_tasks
            .entry(drone_name.to_string())
            .or_default();

        for task in task_list.iter() {
            if task.status == "completed" {
                completed_set.insert(task.id.clone());
            }
        }

        for task in task_list.iter_mut() {
            if completed_set.contains(&task.id) && task.status != "completed" {
                task.status = "completed".to_string();
            }
        }
    }

    fn enforce_progress_monotonicity(
        &mut self,
        drone_name: &str,
        task_list: &[TeamTaskInfo],
    ) -> (usize, usize) {
        let current_completed = task_list.iter().filter(|t| t.status == "completed").count();
        let current_total = task_list.len();

        let (prev_max_completed, prev_max_total) = self
            .high_water_marks
            .get(drone_name)
            .copied()
            .unwrap_or((0, 0));

        let final_completed = current_completed.max(prev_max_completed);
        let final_total = current_total.max(prev_max_total);

        self.high_water_marks
            .insert(drone_name.to_string(), (final_completed, final_total));

        (final_completed, final_total)
    }
}

#[cfg(test)]
mod tests;
