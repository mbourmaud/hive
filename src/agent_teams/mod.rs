pub mod snapshot;
pub mod task_sync;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::types::Plan;

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

/// Format plan content for the team lead prompt.
pub fn format_plan_for_prompt(plan: &Plan) -> String {
    format!("# {}\n\n{}", plan.title, plan.plan)
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

        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut task: AgentTeamTask = match serde_json::from_str(&contents) {
            Ok(t) => t,
            Err(_) => continue,
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
    use crate::types::Plan;

    fn make_test_prd() -> Plan {
        Plan {
            id: "test".to_string(),
            title: "Test PRD".to_string(),
            description: "Test".to_string(),
            version: "1.0".to_string(),
            created_at: String::new(),
            target_platforms: None,
            target_branch: None,
            base_branch: None,
            plan: "## Goal\nImplement feature X\n\n## Tasks\n- Task 1\n- Task 2".to_string(),
        }
    }

    #[test]
    fn test_format_plan_for_prompt() {
        let prd = make_test_prd();
        let output = format_plan_for_prompt(&prd);

        assert!(output.starts_with("# Test PRD"));
        assert!(output.contains("## Goal"));
        assert!(output.contains("Implement feature X"));
        assert!(output.contains("- Task 1"));
    }

    #[test]
    fn test_format_plan_minimal() {
        let prd = Plan {
            id: "minimal".to_string(),
            title: "Minimal".to_string(),
            description: String::new(),
            version: "1.0".to_string(),
            created_at: String::new(),
            target_platforms: None,
            target_branch: None,
            base_branch: None,
            plan: "Just do it".to_string(),
        };
        let output = format_plan_for_prompt(&prd);

        assert_eq!(output, "# Minimal\n\nJust do it");
    }
}
