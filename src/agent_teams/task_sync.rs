use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::read_task_list;
use crate::types::{DroneState, DroneStatus, StoryTiming};

/// Rich task info for monitoring display
#[derive(Debug, Clone)]
pub struct TeamTaskInfo {
    pub id: String,
    pub subject: String,
    pub status: String,
    pub owner: Option<String>,
    pub active_form: Option<String>,
    pub story_id: Option<String>,
}

/// Team member info from config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub name: String,
    #[serde(default, rename = "agentType")]
    pub agent_type: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub cwd: String,
}

/// Team config structure (subset of what Agent Teams writes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub name: String,
    #[serde(default)]
    pub members: Vec<TeamMember>,
}

/// Sync Agent Teams task states into the drone's status.json.
///
/// Reads `~/.claude/tasks/{team-name}/` and updates the corresponding
/// `.hive/drones/{drone-name}/status.json` with reconciled progress.
pub fn sync_team_tasks_to_status(team_name: &str, drone_name: &str) -> Result<()> {
    let tasks = read_task_list(team_name)?;
    if tasks.is_empty() {
        return Ok(());
    }

    let status_path = PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("status.json");

    if !status_path.exists() {
        return Ok(());
    }

    let contents = fs::read_to_string(&status_path)?;
    let mut status: DroneStatus = serde_json::from_str(&contents)?;

    let mut completed: Vec<String> = Vec::new();
    let mut active_stories: Vec<String> = Vec::new();
    let mut active_agents: HashMap<String, String> = HashMap::new();
    let mut has_in_progress = false;

    for task in &tasks {
        match task.status.as_str() {
            "completed" => {
                if !completed.contains(&task.id) {
                    completed.push(task.id.clone());
                }
                // Ensure story_times entry exists
                status
                    .story_times
                    .entry(task.id.clone())
                    .or_insert_with(|| StoryTiming {
                        started: Some(chrono::Utc::now().to_rfc3339()),
                        completed: Some(chrono::Utc::now().to_rfc3339()),
                    });
            }
            "in_progress" => {
                has_in_progress = true;
                let story_id = task.metadata.as_ref()
                    .and_then(|m| m.get("storyId"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&task.id);
                active_stories.push(story_id.to_string());
                if let Some(ref owner) = task.owner {
                    active_agents.insert(owner.clone(), story_id.to_string());
                }
                status
                    .story_times
                    .entry(task.id.clone())
                    .or_insert_with(|| StoryTiming {
                        started: Some(chrono::Utc::now().to_rfc3339()),
                        completed: None,
                    });
            }
            _ => {} // pending
        }
    }

    // Update status fields
    status.completed = completed;
    status.current_story = active_stories.first().cloned();
    status.active_agents = active_agents;

    if status.completed.len() == tasks.len() {
        status.status = DroneState::Completed;
    } else if has_in_progress {
        status.status = DroneState::InProgress;
    }

    status.updated = chrono::Utc::now().to_rfc3339();

    let json = serde_json::to_string_pretty(&status)?;
    fs::write(&status_path, json)?;

    Ok(())
}

/// Read task states for TUI display without writing to status.json.
pub fn read_team_task_states(
    team_name: &str,
) -> Result<HashMap<String, TeamTaskInfo>> {
    let tasks = read_task_list(team_name)?;
    let mut states = HashMap::new();
    for task in tasks {
        let story_id = task.metadata.as_ref()
            .and_then(|m| m.get("storyId"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        states.insert(task.id.clone(), TeamTaskInfo {
            id: task.id,
            subject: task.subject,
            status: task.status,
            owner: task.owner,
            active_form: task.active_form,
            story_id,
        });
    }
    Ok(states)
}

/// An inbox message from Agent Teams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    #[serde(default)]
    pub from: String,
    /// Message content (Agent Teams uses "text" field)
    #[serde(default, alias = "content")]
    pub text: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub read: bool,
}

/// Read inbox messages for all team members.
pub fn read_team_inboxes(team_name: &str) -> Result<HashMap<String, Vec<InboxMessage>>> {
    let inboxes_dir = super::team_dir(team_name).join("inboxes");
    if !inboxes_dir.exists() {
        return Ok(HashMap::new());
    }

    let mut result = HashMap::new();
    for entry in fs::read_dir(&inboxes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let member_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let contents = fs::read_to_string(&path)?;
            if let Ok(messages) = serde_json::from_str::<Vec<InboxMessage>>(&contents) {
                result.insert(member_name, messages);
            }
        }
    }

    Ok(result)
}

/// Read team member info from the Agent Teams config.
pub fn read_team_members(team_name: &str) -> Result<Vec<TeamMember>> {
    let config_path = super::team_dir(team_name).join("config.json");
    if !config_path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&config_path)?;
    let config: TeamConfig = serde_json::from_str(&contents)?;
    // Filter out the team-lead itself, only return actual workers
    Ok(config.members.into_iter()
        .filter(|m| m.agent_type != "team-lead")
        .collect())
}
