pub mod task_sync;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// A task from the team lead's consolidated tasks.json file.
/// Uses different field names than AgentTeamTask (e.g. "title" instead of "subject",
/// numeric id instead of string).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TasksJsonTask {
    id: serde_json::Value, // Can be number or string
    #[serde(alias = "subject")]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default, alias = "dependencies")]
    blocked_by: Vec<serde_json::Value>,
}

/// Wrapper for the tasks.json file format: {"tasks": [...]}
#[derive(Debug, Deserialize)]
struct TasksJsonWrapper {
    tasks: Vec<TasksJsonTask>,
}

/// Read the Agent Teams task list for a team.
pub fn read_task_list(team_name: &str) -> Result<Vec<AgentTeamTask>> {
    let tasks_dir = team_tasks_dir(team_name);

    if !tasks_dir.exists() {
        return Ok(Vec::new());
    }

    let mut tasks = Vec::new();

    // Read individual task files first (1.json, 2.json, etc.)
    for entry in fs::read_dir(&tasks_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            // Skip tasks.json — handled separately below
            if path.file_name().and_then(|n| n.to_str()) == Some("tasks.json") {
                continue;
            }
            let contents = fs::read_to_string(&path)?;
            if let Ok(task) = serde_json::from_str::<AgentTeamTask>(&contents) {
                tasks.push(task);
            }
        }
    }

    // Build a map of internal task ID → agent name (subject) for owner resolution.
    // Internal tasks (1.json, 2.json, ...) have the real agent names as subject.
    // tasks.json may use generic names like "teammate-1" that need mapping.
    let agent_names: HashMap<usize, String> = tasks
        .iter()
        .filter_map(|t| {
            let is_internal = t
                .metadata
                .as_ref()
                .and_then(|m| m.get("_internal"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if is_internal {
                t.id.parse::<usize>()
                    .ok()
                    .map(|idx| (idx, t.subject.clone()))
            } else {
                None
            }
        })
        .collect();

    // Then read tasks.json (team lead's consolidated task list).
    // These come last so they take precedence in the HashMap (same IDs as
    // internal teammate tasks but with actual work item details).
    let tasks_json_path = tasks_dir.join("tasks.json");
    if tasks_json_path.exists() {
        if let Ok(contents) = fs::read_to_string(&tasks_json_path) {
            if let Ok(wrapper) = serde_json::from_str::<TasksJsonWrapper>(&contents) {
                for t in wrapper.tasks {
                    let id = match &t.id {
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::String(s) => s.clone(),
                        _ => continue,
                    };
                    // Resolve generic owner names (teammate-N) to real agent names
                    let owner = t.owner.map(|o| {
                        if o.starts_with("teammate-") {
                            if let Some(idx) = o
                                .strip_prefix("teammate-")
                                .and_then(|n| n.parse::<usize>().ok())
                            {
                                if let Some(real_name) = agent_names.get(&idx) {
                                    return real_name.clone();
                                }
                            }
                        }
                        o
                    });
                    let blocked_by = t
                        .blocked_by
                        .iter()
                        .filter_map(|v| match v {
                            serde_json::Value::Number(n) => Some(n.to_string()),
                            serde_json::Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect();
                    tasks.push(AgentTeamTask {
                        id,
                        subject: t.title,
                        description: t.description,
                        status: t.status,
                        owner,
                        active_form: None,
                        blocked_by,
                        blocks: Vec::new(),
                        metadata: None, // No _internal flag → will show in TUI
                        created_at: None,
                        updated_at: None,
                    });
                }
            }
        }
    }

    Ok(tasks)
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
