use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::task_sync::{TeamMember, TeamTaskInfo};
use super::{read_task_list_safe, AgentTeamTask};

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

/// Serializable snapshot persisted to `.hive/drones/<name>/tasks-snapshot.json`.
#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedSnapshot {
    tasks: Vec<PersistedTask>,
    members: Vec<TeamMember>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedTask {
    id: String,
    subject: String,
    #[serde(default)]
    description: String,
    status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_form: Option<String>,
    #[serde(default)]
    is_internal: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_at: Option<u64>,
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

        // Try live Agent Teams filesystem
        let fs_tasks = read_task_list_safe(drone_name);

        let (mut task_list, members, source) = if !todo_tasks.is_empty() {
            // TodoWrite gives us real task names — prefer it
            let infos = todo_tasks;
            if let Some(ref root) = self.project_root {
                persist_snapshot_at(root, drone_name, &infos, &members);
            } else {
                persist_snapshot(drone_name, &infos, &members);
            }
            (infos, members, SnapshotSource::Tasks)
        } else if !fs_tasks.is_empty() {
            // Fall back to Agent Teams task files (just agent names)
            let infos: Vec<TeamTaskInfo> = fs_tasks.into_iter().map(map_task).collect();
            if let Some(ref root) = self.project_root {
                persist_snapshot_at(root, drone_name, &infos, &members);
            } else {
                persist_snapshot(drone_name, &infos, &members);
            }
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
        };

        // Enforce per-task monotonicity: completed tasks cannot regress
        let completed_set = self
            .completed_tasks
            .entry(drone_name.to_string())
            .or_default();

        for task in &task_list {
            if task.status == "completed" {
                completed_set.insert(task.id.clone());
            }
        }

        for task in &mut task_list {
            if completed_set.contains(&task.id) && task.status != "completed" {
                task.status = "completed".to_string();
            }
        }

        // Calculate progress from all tasks
        let current_completed = task_list.iter().filter(|t| t.status == "completed").count();
        let current_total = task_list.len();

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
            progress: (final_completed, final_total),
            source,
        };

        self.snapshots.insert(drone_name.to_string(), snapshot);
        self.snapshots.get(drone_name).unwrap()
    }
}

fn snapshot_path(drone_name: &str) -> PathBuf {
    PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("tasks-snapshot.json")
}

/// Snapshot path with absolute project root.
fn snapshot_path_at(project_root: &std::path::Path, drone_name: &str) -> PathBuf {
    project_root
        .join(".hive/drones")
        .join(drone_name)
        .join("tasks-snapshot.json")
}

/// Persist current task state to `.hive/drones/<name>/tasks-snapshot.json`.
fn persist_snapshot(drone_name: &str, tasks: &[TeamTaskInfo], members: &[TeamMember]) {
    let persisted = PersistedSnapshot {
        tasks: tasks
            .iter()
            .map(|t| PersistedTask {
                id: t.id.clone(),
                subject: t.subject.clone(),
                description: t.description.clone(),
                status: t.status.clone(),
                owner: t.owner.clone(),
                active_form: t.active_form.clone(),
                is_internal: t.is_internal,
                created_at: t.created_at,
                updated_at: t.updated_at,
            })
            .collect(),
        members: members.to_vec(),
    };

    if let Ok(json) = serde_json::to_string(&persisted) {
        let _ = std::fs::write(snapshot_path(drone_name), json);
    }
}

/// Load persisted snapshot from `.hive/drones/<name>/tasks-snapshot.json`.
fn load_persisted_snapshot(drone_name: &str) -> Option<(Vec<TeamTaskInfo>, Vec<TeamMember>)> {
    load_persisted_snapshot_from_path(&snapshot_path(drone_name))
}

/// Load persisted snapshot with absolute project root.
fn load_persisted_snapshot_at(
    project_root: &std::path::Path,
    drone_name: &str,
) -> Option<(Vec<TeamTaskInfo>, Vec<TeamMember>)> {
    load_persisted_snapshot_from_path(&snapshot_path_at(project_root, drone_name))
}

fn load_persisted_snapshot_from_path(
    path: &std::path::Path,
) -> Option<(Vec<TeamTaskInfo>, Vec<TeamMember>)> {
    let contents = std::fs::read_to_string(path).ok()?;
    let persisted: PersistedSnapshot = serde_json::from_str(&contents).ok()?;

    let tasks = persisted
        .tasks
        .into_iter()
        .map(|t| TeamTaskInfo {
            id: t.id,
            subject: t.subject,
            description: t.description,
            status: t.status,
            owner: t.owner,
            active_form: t.active_form,
            model: None,
            is_internal: t.is_internal,
            created_at: t.created_at,
            updated_at: t.updated_at,
        })
        .collect();

    Some((tasks, persisted.members))
}

/// Persist snapshot with absolute project root.
fn persist_snapshot_at(
    project_root: &std::path::Path,
    drone_name: &str,
    tasks: &[TeamTaskInfo],
    members: &[TeamMember],
) {
    let persisted = PersistedSnapshot {
        tasks: tasks
            .iter()
            .map(|t| PersistedTask {
                id: t.id.clone(),
                subject: t.subject.clone(),
                description: t.description.clone(),
                status: t.status.clone(),
                owner: t.owner.clone(),
                active_form: t.active_form.clone(),
                is_internal: t.is_internal,
                created_at: t.created_at,
                updated_at: t.updated_at,
            })
            .collect(),
        members: members.to_vec(),
    };

    if let Ok(json) = serde_json::to_string(&persisted) {
        let _ = std::fs::write(snapshot_path_at(project_root, drone_name), json);
    }
}

/// Load tasks from `todos.json` (written by the TodoWrite hook).
///
/// The hook captures `[{content, status, activeForm}]` — real task names and progress
/// from the team lead's TodoWrite calls.
fn load_todos(path: &std::path::Path) -> Vec<TeamTaskInfo> {
    #[derive(serde::Deserialize)]
    struct TodoEntry {
        content: String,
        #[serde(default = "default_pending")]
        status: String,
        #[serde(default, rename = "activeForm")]
        active_form: Option<String>,
    }

    fn default_pending() -> String {
        "pending".to_string()
    }

    let contents = match std::fs::read_to_string(path) {
        Ok(c) if !c.trim().is_empty() => c,
        _ => return Vec::new(),
    };

    let entries: Vec<TodoEntry> = match serde_json::from_str(&contents) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .into_iter()
        .enumerate()
        .map(|(i, e)| TeamTaskInfo {
            id: (i + 1).to_string(),
            subject: e.content,
            description: String::new(),
            status: e.status,
            owner: None,
            active_form: e.active_form,
            model: None,
            is_internal: false,
            created_at: None,
            updated_at: None,
        })
        .collect()
}

/// Map an `AgentTeamTask` (from filesystem JSON) to `TeamTaskInfo` (for TUI display).
pub fn map_task(t: AgentTeamTask) -> TeamTaskInfo {
    let is_internal = t
        .metadata
        .as_ref()
        .and_then(|m| m.get("_internal"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    TeamTaskInfo {
        id: t.id,
        subject: t.subject,
        description: t.description,
        status: t.status,
        owner: t.owner,
        active_form: t.active_form,
        model: None,
        is_internal,
        created_at: t.created_at,
        updated_at: t.updated_at,
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

        store
            .high_water_marks
            .insert("test-drone".to_string(), (3, 5));

        let (prev_completed, prev_total) = store
            .high_water_marks
            .get("test-drone")
            .copied()
            .unwrap_or((0, 0));

        let current_completed = 0;
        let current_total = 0;

        let final_completed = current_completed.max(prev_completed);
        let final_total = current_total.max(prev_total);

        assert_eq!(final_completed, 3);
        assert_eq!(final_total, 5);
    }

    #[test]
    fn test_task_status_monotonicity() {
        let mut store = TaskSnapshotStore::new();

        let completed_set = store
            .completed_tasks
            .entry("test-drone".to_string())
            .or_default();
        completed_set.insert("1".to_string());

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

    #[test]
    fn test_map_task_internal_flag() {
        let task = AgentTeamTask {
            id: "1".to_string(),
            subject: "internal-task".to_string(),
            description: String::new(),
            status: "pending".to_string(),
            owner: None,
            active_form: None,
            blocked_by: Vec::new(),
            blocks: Vec::new(),
            metadata: Some(serde_json::json!({"_internal": true})),
            created_at: Some(1000),
            updated_at: Some(2000),
            files: None,
        };

        let info = map_task(task);
        assert!(info.is_internal);
        assert_eq!(info.created_at, Some(1000));
        assert_eq!(info.updated_at, Some(2000));
    }

    #[test]
    fn test_map_task_not_internal_by_default() {
        let task = AgentTeamTask {
            id: "2".to_string(),
            subject: "user-task".to_string(),
            description: String::new(),
            status: "in_progress".to_string(),
            owner: Some("worker".to_string()),
            active_form: Some("Working".to_string()),
            blocked_by: Vec::new(),
            blocks: Vec::new(),
            metadata: None,
            created_at: None,
            updated_at: None,
            files: None,
        };

        let info = map_task(task);
        assert!(!info.is_internal);
        assert_eq!(info.owner, Some("worker".to_string()));
        assert_eq!(info.active_form, Some("Working".to_string()));
    }

    #[test]
    fn test_persist_and_load_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let drone_name = "persist-test";
        let drone_dir = dir.path().join(".hive/drones").join(drone_name);
        std::fs::create_dir_all(&drone_dir).unwrap();

        // Must run from temp dir so snapshot_path resolves
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let tasks = vec![
            TeamTaskInfo {
                id: "1".to_string(),
                subject: "Task A".to_string(),
                description: "Do A".to_string(),
                status: "completed".to_string(),
                owner: Some("worker-1".to_string()),
                active_form: None,
                model: None,
                is_internal: false,
                created_at: Some(1000),
                updated_at: Some(2000),
            },
            TeamTaskInfo {
                id: "2".to_string(),
                subject: "Task B".to_string(),
                description: String::new(),
                status: "in_progress".to_string(),
                owner: None,
                active_form: Some("Working".to_string()),
                model: None,
                is_internal: true,
                created_at: None,
                updated_at: None,
            },
        ];
        let members = vec![TeamMember {
            name: "worker-1".to_string(),
            agent_type: "general-purpose".to_string(),
            model: "sonnet".to_string(),
            cwd: String::new(),
        }];

        persist_snapshot(drone_name, &tasks, &members);

        let (loaded_tasks, loaded_members) = load_persisted_snapshot(drone_name).unwrap();

        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(loaded_tasks.len(), 2);
        assert_eq!(loaded_tasks[0].subject, "Task A");
        assert_eq!(loaded_tasks[0].status, "completed");
        assert_eq!(loaded_tasks[0].owner, Some("worker-1".to_string()));
        assert!(loaded_tasks[1].is_internal);
        assert_eq!(loaded_tasks[1].active_form, Some("Working".to_string()));

        assert_eq!(loaded_members.len(), 1);
        assert_eq!(loaded_members[0].name, "worker-1");
        assert_eq!(loaded_members[0].model, "sonnet");
    }

    #[test]
    fn test_load_persisted_snapshot_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = load_persisted_snapshot("no-such-drone");

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_none());
    }
}
