use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

use super::read_task_list;

/// Rich task info for monitoring display
#[derive(Debug, Clone)]
pub struct TeamTaskInfo {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub status: String,
    pub owner: Option<String>,
    pub active_form: Option<String>,
    pub model: Option<String>,
    /// Whether this is an internal teammate tracking task
    pub is_internal: bool,
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

/// Read task states for TUI display without writing to status.json.
pub fn read_team_task_states(team_name: &str) -> Result<HashMap<String, TeamTaskInfo>> {
    let tasks = read_task_list(team_name)?;

    // Build owner->model map from team members
    let member_models: HashMap<String, String> = read_team_members(team_name)
        .unwrap_or_default()
        .into_iter()
        .filter(|m| !m.model.is_empty())
        .map(|m| (m.name, m.model))
        .collect();

    let mut states = HashMap::new();
    for task in tasks {
        let is_internal = task
            .metadata
            .as_ref()
            .and_then(|m| m.get("_internal"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let model = task
            .owner
            .as_ref()
            .and_then(|owner| member_models.get(owner))
            .cloned();
        states.insert(
            task.id.clone(),
            TeamTaskInfo {
                id: task.id,
                subject: task.subject,
                description: task.description,
                status: task.status,
                owner: task.owner,
                active_form: task.active_form,
                model,
                is_internal,
            },
        );
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

    // Try reading from live Claude data first
    if inboxes_dir.exists() {
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
        if !result.is_empty() {
            return Ok(result);
        }
    }

    // Fallback: read from .hive/drones/<name>/messages.ndjson
    read_persisted_messages_fallback(team_name)
}

/// Fallback: read messages from .hive/drones/<name>/messages.ndjson
fn read_persisted_messages_fallback(team_name: &str) -> Result<HashMap<String, Vec<InboxMessage>>> {
    use std::path::PathBuf;

    let messages_file = PathBuf::from(".hive/drones")
        .join(team_name)
        .join("messages.ndjson");

    if !messages_file.exists() {
        return Ok(HashMap::new());
    }

    let contents = fs::read_to_string(&messages_file)?;
    let mut result: HashMap<String, Vec<InboxMessage>> = HashMap::new();

    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
            let from = msg
                .get("from")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let to = msg
                .get("to")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let content = msg
                .get("content")
                .and_then(|v| v.as_str())
                .or_else(|| msg.get("summary").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            let timestamp = msg
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let inbox_msg = InboxMessage {
                from,
                text: content,
                timestamp,
                read: false,
            };

            result.entry(to).or_default().push(inbox_msg);
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
    // Filter out the team-lead/coordinator, only return actual workers
    Ok(config
        .members
        .into_iter()
        .filter(|m| m.agent_type != "team-lead" && m.agent_type != "coordinator")
        .collect())
}
