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
///
/// If the PRD has a freeform `plan` field, uses that directly.
/// Otherwise falls back to formatting individual stories (backwards compat).
pub fn format_prd_for_prompt(prd: &Prd) -> String {
    // New thin format: freeform plan
    if let Some(ref plan) = prd.plan {
        return format!("# {}\n\n{}", prd.title, plan);
    }

    // Legacy format: structured stories
    let mut sections = Vec::new();

    sections.push(format!("# {}\n{}", prd.title, prd.description));

    for story in &prd.stories {
        let mut parts = vec![format!("## Story {}: {}", story.id, story.title)];
        parts.push(format!("Description: {}", story.description));

        if let Some(ref criteria) = story.acceptance_criteria {
            if !criteria.is_empty() {
                parts.push(format!(
                    "Acceptance Criteria:\n{}",
                    criteria
                        .iter()
                        .map(|c| format!("- {}", c))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
        }

        if !story.definition_of_done.is_empty() {
            parts.push(format!(
                "Definition of Done:\n{}",
                story
                    .definition_of_done
                    .iter()
                    .map(|d| format!("- {}", d))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if !story.verification_commands.is_empty() {
            parts.push(format!(
                "Verification:\n{}",
                story
                    .verification_commands
                    .iter()
                    .map(|v| format!("```\n{}\n```", v))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if !story.depends_on.is_empty() {
            parts.push(format!("Depends on: {}", story.depends_on.join(", ")));
        }

        if !story.files.is_empty() {
            parts.push(format!("Files: {}", story.files.join(", ")));
        }

        sections.push(parts.join("\n"));
    }

    sections.join("\n\n")
}

/// Map task numeric IDs back to story IDs using metadata.
pub fn task_id_to_story_id(tasks: &[AgentTeamTask]) -> HashMap<String, String> {
    tasks
        .iter()
        .filter_map(|t| {
            t.metadata
                .as_ref()
                .and_then(|m| m.get("storyId"))
                .and_then(|v| v.as_str())
                .map(|sid| (t.id.clone(), sid.to_string()))
        })
        .collect()
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
    use crate::types::{Prd, Story};

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
            plan: None,
            stories: vec![
                Story {
                    id: "S1".to_string(),
                    title: "Story 1".to_string(),
                    description: "First story".to_string(),
                    acceptance_criteria: Some(vec!["Criterion A".to_string()]),
                    definition_of_done: vec!["Done".to_string()],
                    verification_commands: vec!["cargo test".to_string()],
                    notes: None,
                    actions: vec![],
                    files: vec!["src/a.rs".to_string()],
                    tools: vec![],
                    context: Default::default(),
                    testing: Default::default(),
                    error_handling: None,
                    agent_controls: None,
                    communication: None,
                    depends_on: vec![],
                    parallel: true,
                },
                Story {
                    id: "S2".to_string(),
                    title: "Story 2".to_string(),
                    description: "Second story".to_string(),
                    acceptance_criteria: None,
                    definition_of_done: vec![],
                    verification_commands: vec![],
                    notes: None,
                    actions: vec![],
                    files: vec![],
                    tools: vec![],
                    context: Default::default(),
                    testing: Default::default(),
                    error_handling: None,
                    agent_controls: None,
                    communication: None,
                    depends_on: vec!["S1".to_string()],
                    parallel: false,
                },
            ],
        }
    }

    #[test]
    fn test_format_prd_for_prompt() {
        let prd = make_test_prd();
        let output = format_prd_for_prompt(&prd);

        assert!(output.contains("# Test PRD"));
        assert!(output.contains("## Story S1: Story 1"));
        assert!(output.contains("Description: First story"));
        assert!(output.contains("- Criterion A"));
        assert!(output.contains("- Done"));
        assert!(output.contains("cargo test"));
        assert!(output.contains("Files: src/a.rs"));

        assert!(output.contains("## Story S2: Story 2"));
        assert!(output.contains("Depends on: S1"));
        // S2 has no files, so "Files:" should not appear for S2
        // S1 has files, so "Files:" appears once
    }

    #[test]
    fn test_format_prd_with_plan() {
        let prd = Prd {
            id: "lean".to_string(),
            title: "Lean PRD".to_string(),
            description: String::new(),
            version: "1.0".to_string(),
            created_at: String::new(),
            target_platforms: None,
            target_branch: None,
            base_branch: None,
            plan: Some("## Goal\nBuild feature X\n\n## Requirements\n- Thing A\n- Thing B".to_string()),
            stories: vec![],
        };
        let output = format_prd_for_prompt(&prd);

        assert!(output.starts_with("# Lean PRD"));
        assert!(output.contains("## Goal"));
        assert!(output.contains("- Thing A"));
        // Should NOT contain story formatting
        assert!(!output.contains("## Story"));
    }

    #[test]
    fn test_format_prd_plan_takes_precedence_over_stories() {
        let mut prd = make_test_prd();
        prd.plan = Some("Custom plan text".to_string());
        let output = format_prd_for_prompt(&prd);

        // Plan should win even when stories exist
        assert!(output.contains("Custom plan text"));
        assert!(!output.contains("## Story"));
    }

    #[test]
    fn test_task_id_to_story_id() {
        let tasks = vec![
            AgentTeamTask {
                id: "1".to_string(),
                subject: "Story 1".to_string(),
                description: "desc".to_string(),
                status: "pending".to_string(),
                owner: None,
                active_form: None,
                blocked_by: vec![],
                blocks: vec![],
                metadata: Some(serde_json::json!({"storyId": "S1"})),
                created_at: None,
                updated_at: None,
            },
            AgentTeamTask {
                id: "2".to_string(),
                subject: "Story 2".to_string(),
                description: "desc".to_string(),
                status: "pending".to_string(),
                owner: None,
                active_form: None,
                blocked_by: vec![],
                blocks: vec![],
                metadata: Some(serde_json::json!({"storyId": "S2"})),
                created_at: None,
                updated_at: None,
            },
        ];
        let mapping = task_id_to_story_id(&tasks);
        assert_eq!(mapping.get("1").unwrap(), "S1");
        assert_eq!(mapping.get("2").unwrap(), "S2");
    }
}
