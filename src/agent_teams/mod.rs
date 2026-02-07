pub mod task_sync;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::types::Prd;

/// An Agent Teams task, mapped from a PRD story.
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

/// Format PRD content as readable text for the team lead prompt.
pub fn format_prd_for_prompt(prd: &Prd) -> String {
    format!("# {}\n\n{}", prd.title, prd.plan)
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
    for entry in fs::read_dir(&tasks_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let contents = fs::read_to_string(&path)?;
            if let Ok(task) = serde_json::from_str::<AgentTeamTask>(&contents) {
                tasks.push(task);
            }
        }
    }

    Ok(tasks)
}

/// Clean up Agent Teams directories for a team.
pub fn cleanup_team(team_name: &str) -> Result<()> {
    let tasks_dir = team_tasks_dir(team_name);
    if tasks_dir.exists() {
        fs::remove_dir_all(&tasks_dir)
            .context("Failed to remove Agent Teams tasks directory")?;
    }

    let teams_dir = team_dir(team_name);
    if teams_dir.exists() {
        fs::remove_dir_all(&teams_dir)
            .context("Failed to remove Agent Teams team directory")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Prd;

    fn make_test_prd() -> Prd {
        Prd {
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
    fn test_format_prd_for_prompt() {
        let prd = make_test_prd();
        let output = format_prd_for_prompt(&prd);

        assert!(output.starts_with("# Test PRD"));
        assert!(output.contains("## Goal"));
        assert!(output.contains("Implement feature X"));
        assert!(output.contains("- Task 1"));
    }

    #[test]
    fn test_format_prd_minimal() {
        let prd = Prd {
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
        let output = format_prd_for_prompt(&prd);

        assert_eq!(output, "# Minimal\n\nJust do it");
    }
}
