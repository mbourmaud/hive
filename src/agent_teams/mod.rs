pub mod snapshot;
pub mod task_sync;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{StructuredTask, TaskType};

/// An Agent Teams task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTeamTask {
    pub id: String,
    pub subject: String,
    pub description: String,
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

/// Get the task list directory for a team.
pub fn team_tasks_dir(team_name: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".claude")
        .join("tasks")
        .join(team_name)
}

/// Get the team directory.
pub fn team_dir(team_name: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".claude")
        .join("teams")
        .join(team_name)
}

/// Read the Agent Teams task list for a team.
pub fn read_task_list(team_name: &str) -> Result<Vec<AgentTeamTask>> {
    let tasks_dir = team_tasks_dir(team_name);

    if !tasks_dir.exists() {
        return Ok(Vec::new());
    }

    let mut tasks = Vec::new();

    // Read individual task files (1.json, 2.json, etc.)
    for entry in fs::read_dir(&tasks_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            // Skip tasks.json (legacy consolidated format, no longer used)
            if path.file_name().and_then(|n| n.to_str()) == Some("tasks.json") {
                continue;
            }
            match fs::read_to_string(&path) {
                Ok(contents) => {
                    if let Ok(task) = serde_json::from_str::<AgentTeamTask>(&contents) {
                        tasks.push(task);
                    } else {
                        eprintln!(
                            "[hive] Malformed task JSON: {:?}",
                            path.file_name().unwrap_or_default()
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[hive] Could not read task file {:?}: {}",
                        path.file_name().unwrap_or_default(),
                        e
                    );
                    continue;
                }
            }
        }
    }

    Ok(tasks)
}

/// Read the Agent Teams task list, never returning Err.
/// On any filesystem error, returns best-effort partial results.
pub fn read_task_list_safe(team_name: &str) -> Vec<AgentTeamTask> {
    read_task_list(team_name).unwrap_or_default()
}

/// Auto-complete all in_progress tasks for a team.
/// Called when the drone's process shuts down to avoid tasks stuck in in_progress forever.
pub fn auto_complete_tasks(team_name: &str) -> Result<()> {
    let tasks_dir = team_tasks_dir(team_name);
    if !tasks_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&tasks_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some("tasks.json") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut task) = serde_json::from_str::<AgentTeamTask>(&contents) else {
            continue;
        };

        if task.status == "in_progress" {
            task.status = "completed".to_string();
            task.updated_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            );
            if let Ok(json) = serde_json::to_string_pretty(&task) {
                let _ = fs::write(&path, json);
            }
        }
    }

    Ok(())
}

/// Pre-seed tasks from structured plan into `~/.claude/tasks/{team}/` and emit
/// TaskCreate events to `events.ndjson` so the TUI shows tasks immediately.
///
/// Filters to Work tasks only (Setup/PR handled by Hive).
/// Maps `depends_on` plan numbers to pre-seeded task IDs for `blocked_by`.
pub fn preseed_tasks(
    team_name: &str,
    tasks: &[StructuredTask],
    drone_dir: &Path,
) -> Result<Vec<AgentTeamTask>> {
    // Filter to Work tasks only
    let work_tasks: Vec<&StructuredTask> = tasks
        .iter()
        .filter(|t| t.task_type == TaskType::Work)
        .collect();

    if work_tasks.is_empty() {
        return Ok(Vec::new());
    }

    // Build mapping: plan task number → pre-seeded task ID (1-based sequential)
    let mut number_to_id: std::collections::HashMap<usize, String> =
        std::collections::HashMap::new();
    for (idx, task) in work_tasks.iter().enumerate() {
        number_to_id.insert(task.number, (idx + 1).to_string());
    }

    let tasks_dir = team_tasks_dir(team_name);
    fs::create_dir_all(&tasks_dir)?;

    let events_path = drone_dir.join("events.ndjson");
    let mut events_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&events_path)?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut seeded = Vec::new();

    for (idx, task) in work_tasks.iter().enumerate() {
        let id = (idx + 1).to_string();

        // Map depends_on plan numbers to pre-seeded task IDs
        let blocked_by: Vec<String> = task
            .depends_on
            .iter()
            .filter_map(|dep| number_to_id.get(dep).cloned())
            .collect();

        // Build metadata with model/files/parallel info
        let mut metadata = serde_json::json!({});
        if let Some(ref model) = task.model {
            metadata["model"] = serde_json::json!(model);
        }
        if task.parallel {
            metadata["parallel"] = serde_json::json!(true);
        }
        if !task.files.is_empty() {
            metadata["files"] = serde_json::json!(task.files);
        }
        metadata["plan_number"] = serde_json::json!(task.number);

        let description = if task.body.is_empty() {
            task.title.clone()
        } else {
            task.body.clone()
        };

        let agent_task = AgentTeamTask {
            id: id.clone(),
            subject: task.title.clone(),
            description: description.clone(),
            status: "pending".to_string(),
            owner: None,
            active_form: None,
            blocked_by,
            blocks: Vec::new(),
            metadata: Some(metadata),
            created_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            ),
            updated_at: None,
        };

        // Write task JSON file
        let task_path = tasks_dir.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&agent_task)?;
        fs::write(&task_path, json)?;

        // Emit TaskCreate event to events.ndjson
        let event = serde_json::json!({
            "event": "TaskCreate",
            "ts": now,
            "subject": task.title,
            "description": description,
        });
        use std::io::Write;
        writeln!(events_file, "{}", serde_json::to_string(&event)?)?;

        seeded.push(agent_task);
    }

    Ok(seeded)
}

/// Clean up Agent Teams directories for a team.
pub fn cleanup_team(team_name: &str) -> Result<()> {
    let tasks_dir = team_tasks_dir(team_name);
    if tasks_dir.exists() {
        fs::remove_dir_all(&tasks_dir).context("Failed to remove Agent Teams tasks directory")?;
    }

    let teams_dir = team_dir(team_name);
    if teams_dir.exists() {
        fs::remove_dir_all(&teams_dir).context("Failed to remove Agent Teams team directory")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preseed_tasks_filters_work_only() {
        let dir = tempfile::tempdir().unwrap();
        let drone_dir = dir.path().join("drone");
        fs::create_dir_all(&drone_dir).unwrap();

        let tasks = vec![
            StructuredTask {
                number: 1,
                title: "Setup".to_string(),
                body: String::new(),
                task_type: TaskType::Setup,
                model: None,
                parallel: false,
                files: Vec::new(),
                depends_on: Vec::new(),
            },
            StructuredTask {
                number: 2,
                title: "Implement feature".to_string(),
                body: "Do the work".to_string(),
                task_type: TaskType::Work,
                model: Some("sonnet".to_string()),
                parallel: true,
                files: vec!["src/main.rs".to_string()],
                depends_on: Vec::new(),
            },
            StructuredTask {
                number: 3,
                title: "Create PR".to_string(),
                body: String::new(),
                task_type: TaskType::Pr,
                model: None,
                parallel: false,
                files: Vec::new(),
                depends_on: vec![2],
            },
        ];

        let team_name = format!("test-preseed-{}", std::process::id());
        let seeded = preseed_tasks(&team_name, &tasks, &drone_dir).unwrap();

        // Only Work tasks should be seeded
        assert_eq!(seeded.len(), 1);
        assert_eq!(seeded[0].subject, "Implement feature");
        assert_eq!(seeded[0].status, "pending");
        assert_eq!(seeded[0].id, "1");

        // Check metadata
        let meta = seeded[0].metadata.as_ref().unwrap();
        assert_eq!(meta["model"], "sonnet");
        assert_eq!(meta["parallel"], true);
        assert_eq!(meta["files"], serde_json::json!(["src/main.rs"]));
        assert_eq!(meta["plan_number"], 2);

        // Check task file was written
        let task_path = team_tasks_dir(&team_name).join("1.json");
        assert!(task_path.exists());

        // Check events.ndjson was written
        let events = fs::read_to_string(drone_dir.join("events.ndjson")).unwrap();
        assert!(events.contains("TaskCreate"));
        assert!(events.contains("Implement feature"));

        // Cleanup
        let _ = cleanup_team(&team_name);
    }

    #[test]
    fn test_preseed_tasks_maps_depends_on() {
        let dir = tempfile::tempdir().unwrap();
        let drone_dir = dir.path().join("drone");
        fs::create_dir_all(&drone_dir).unwrap();

        let tasks = vec![
            StructuredTask {
                number: 2,
                title: "Task A".to_string(),
                body: String::new(),
                task_type: TaskType::Work,
                model: None,
                parallel: false,
                files: Vec::new(),
                depends_on: Vec::new(),
            },
            StructuredTask {
                number: 3,
                title: "Task B".to_string(),
                body: String::new(),
                task_type: TaskType::Work,
                model: None,
                parallel: false,
                files: Vec::new(),
                depends_on: vec![2],
            },
        ];

        let team_name = format!("test-deps-{}", std::process::id());
        let seeded = preseed_tasks(&team_name, &tasks, &drone_dir).unwrap();

        assert_eq!(seeded.len(), 2);
        // Task B (id=2) depends on Task A (id=1)
        assert!(seeded[0].blocked_by.is_empty());
        assert_eq!(seeded[1].blocked_by, vec!["1"]);

        let _ = cleanup_team(&team_name);
    }

    #[test]
    fn test_agent_team_task_serialization() {
        let task = AgentTeamTask {
            id: "1".to_string(),
            subject: "Create auth middleware".to_string(),
            description: "JWT verification".to_string(),
            status: "pending".to_string(),
            owner: None,
            active_form: None,
            blocked_by: Vec::new(),
            blocks: Vec::new(),
            metadata: None,
            created_at: Some(1000),
            updated_at: Some(1000),
        };

        let json = serde_json::to_string_pretty(&task).unwrap();
        let parsed: AgentTeamTask = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "1");
        assert_eq!(parsed.subject, "Create auth middleware");
        assert_eq!(parsed.status, "pending");
    }
}
