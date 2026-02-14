use std::path::PathBuf;

use super::super::task_sync::TeamMember;
use super::super::task_sync::TeamTaskInfo;

/// Serializable snapshot persisted to `.hive/drones/<name>/tasks-snapshot.json`.
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct PersistedSnapshot {
    pub tasks: Vec<PersistedTask>,
    pub members: Vec<TeamMember>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct PersistedTask {
    pub id: String,
    pub subject: String,
    #[serde(default)]
    pub description: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    #[serde(default)]
    pub is_internal: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

pub fn snapshot_path(drone_name: &str) -> PathBuf {
    PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("tasks-snapshot.json")
}

/// Snapshot path with absolute project root.
pub fn snapshot_path_at(project_root: &std::path::Path, drone_name: &str) -> PathBuf {
    project_root
        .join(".hive/drones")
        .join(drone_name)
        .join("tasks-snapshot.json")
}

/// Persist current task state to `.hive/drones/<name>/tasks-snapshot.json`.
pub fn persist_snapshot(drone_name: &str, tasks: &[TeamTaskInfo], members: &[TeamMember]) {
    let persisted = build_persisted_snapshot(tasks, members);
    if let Ok(json) = serde_json::to_string(&persisted) {
        let _ = std::fs::write(snapshot_path(drone_name), json);
    }
}

/// Persist snapshot with absolute project root.
pub fn persist_snapshot_at(
    project_root: &std::path::Path,
    drone_name: &str,
    tasks: &[TeamTaskInfo],
    members: &[TeamMember],
) {
    let persisted = build_persisted_snapshot(tasks, members);
    if let Ok(json) = serde_json::to_string(&persisted) {
        let _ = std::fs::write(snapshot_path_at(project_root, drone_name), json);
    }
}

fn build_persisted_snapshot(tasks: &[TeamTaskInfo], members: &[TeamMember]) -> PersistedSnapshot {
    PersistedSnapshot {
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
    }
}

/// Load persisted snapshot from `.hive/drones/<name>/tasks-snapshot.json`.
pub fn load_persisted_snapshot(drone_name: &str) -> Option<(Vec<TeamTaskInfo>, Vec<TeamMember>)> {
    load_persisted_snapshot_from_path(&snapshot_path(drone_name))
}

/// Load persisted snapshot with absolute project root.
pub fn load_persisted_snapshot_at(
    project_root: &std::path::Path,
    drone_name: &str,
) -> Option<(Vec<TeamTaskInfo>, Vec<TeamMember>)> {
    load_persisted_snapshot_from_path(&snapshot_path_at(project_root, drone_name))
}

pub fn load_persisted_snapshot_from_path(
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
