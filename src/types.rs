use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plan (formerly PRD) structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    /// Title of the PRD (also accepts "name" for backwards compatibility)
    #[serde(alias = "name")]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub created_at: String,
    pub target_platforms: Option<Vec<String>>,
    pub target_branch: Option<String>,
    /// Base branch to create worktree from (defaults to origin/master or origin/main)
    /// For master/main, always uses origin/ version (up-to-date remote)
    pub base_branch: Option<String>,
    /// Freeform markdown plan — sent directly to the team lead
    #[serde(default)]
    pub plan: String,
}

/// Drone execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneStatus {
    pub drone: String,
    pub prd: String,
    pub branch: String,
    pub worktree: String,
    pub local_mode: bool,
    /// Execution mode (always AgentTeam, kept for backwards compat)
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    /// Execution backend: always "agent_team"
    #[serde(default = "default_backend")]
    pub backend: String,
    pub status: DroneState,
    #[serde(default, alias = "current_story")]
    pub current_task: Option<String>,
    #[serde(default)]
    pub completed: Vec<String>,
    /// Legacy field — kept for deserialization compat, skipped on serialization
    #[serde(default, skip_serializing)]
    pub story_times: HashMap<String, StoryTiming>,
    pub total: usize,
    pub started: String,
    pub updated: String,
    pub error_count: usize,
    #[serde(default, alias = "last_error_story")]
    pub last_error: Option<String>,
    /// Active agents and their current task (for Agent Teams mode)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub active_agents: HashMap<String, String>,
}

fn default_backend() -> String {
    "agent_team".to_string()
}

/// Story timing information (deprecated, kept for backwards compat deserialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryTiming {
    pub started: Option<String>,
    pub completed: Option<String>,
}

/// Drone execution state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DroneState {
    Starting,
    Resuming,
    InProgress,
    Completed,
    Error,
    Stopped,
    Cleaning,
    Zombie,
}

/// Drone execution mode (always AgentTeam; Worktree kept as alias for backwards compat)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Agent Teams mode: Claude Code native multi-agent coordination
    #[default]
    AgentTeam,
}

impl<'de> serde::Deserialize<'de> for ExecutionMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Map old "worktree" values to AgentTeam for backwards compat
        match s.as_str() {
            "agent_team" | "worktree" => Ok(ExecutionMode::AgentTeam),
            _ => Ok(ExecutionMode::AgentTeam),
        }
    }
}

impl serde::Serialize for ExecutionMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("agent_team")
    }
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "agent_team")
    }
}

impl std::fmt::Display for DroneState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DroneState::Starting => write!(f, "starting"),
            DroneState::Resuming => write!(f, "resuming"),
            DroneState::InProgress => write!(f, "in_progress"),
            DroneState::Completed => write!(f, "completed"),
            DroneState::Error => write!(f, "error"),
            DroneState::Stopped => write!(f, "stopped"),
            DroneState::Cleaning => write!(f, "cleaning"),
            DroneState::Zombie => write!(f, "zombie"),
        }
    }
}

/// Hive configuration (global and local)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveConfig {
    pub version: String,
    pub project: Option<String>,
    pub worktree_base: Option<String>,
    pub default_model: Option<String>,
    pub timestamp: String,
}

impl Default for HiveConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            project: None,
            worktree_base: None,
            default_model: Some("sonnet".to_string()),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plan_prd() {
        let json = r###"{
            "id": "my-feature",
            "title": "My Feature",
            "version": "1.0.0",
            "target_branch": "feature/my-feature",
            "base_branch": "main",
            "plan": "## Goal\nBuild X\n\n## Requirements\n- Thing A"
        }"###;

        let prd: Plan = serde_json::from_str(json).unwrap();
        assert_eq!(prd.id, "my-feature");
        assert_eq!(prd.plan, "## Goal\nBuild X\n\n## Requirements\n- Thing A");
    }

    #[test]
    fn test_parse_minimal_prd() {
        let json = r#"{
            "id": "minimal",
            "title": "Minimal PRD",
            "plan": "Do the thing"
        }"#;

        let prd: Plan = serde_json::from_str(json).unwrap();
        assert_eq!(prd.id, "minimal");
        assert_eq!(prd.plan, "Do the thing");
    }

    #[test]
    fn test_parse_drone_status() {
        let json = r#"{
            "drone": "test-drone",
            "prd": "test-prd.json",
            "branch": "hive/test",
            "worktree": "/path/to/worktree",
            "local_mode": false,
            "status": "in_progress",
            "current_story": "TEST-001",
            "completed": [],
            "story_times": {},
            "total": 5,
            "started": "2024-01-01T00:00:00Z",
            "updated": "2024-01-01T00:00:00Z",
            "error_count": 0,
            "last_error_story": null
        }"#;

        let status: DroneStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.drone, "test-drone");
        assert_eq!(status.status, DroneState::InProgress);
        assert_eq!(status.current_task, Some("TEST-001".to_string()));
    }

    #[test]
    fn test_parse_hive_config() {
        let json = r#"{
            "version": "1.0.0",
            "project": "test-project",
            "worktree_base": "/tmp/hive",
            "default_model": "sonnet",
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;

        let config: HiveConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.project, Some("test-project".to_string()));
    }

    #[test]
    fn test_drone_state_display() {
        assert_eq!(DroneState::Starting.to_string(), "starting");
        assert_eq!(DroneState::Resuming.to_string(), "resuming");
        assert_eq!(DroneState::InProgress.to_string(), "in_progress");
        assert_eq!(DroneState::Completed.to_string(), "completed");
        assert_eq!(DroneState::Error.to_string(), "error");
        assert_eq!(DroneState::Stopped.to_string(), "stopped");
        assert_eq!(DroneState::Cleaning.to_string(), "cleaning");
        assert_eq!(DroneState::Zombie.to_string(), "zombie");
    }

    #[test]
    fn test_default_config() {
        let config = HiveConfig::default();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.default_model, Some("sonnet".to_string()));
        assert!(config.project.is_none());
    }
}
