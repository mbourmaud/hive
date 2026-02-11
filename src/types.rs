use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plan — a markdown file with metadata extracted from content/filename.
#[derive(Debug, Clone)]
pub struct Plan {
    pub id: String,
    /// Raw markdown content — sent directly to the team lead as prompt
    pub content: String,
    pub target_branch: Option<String>,
    /// Base branch to create worktree from (defaults to origin/master or origin/main)
    pub base_branch: Option<String>,
}

impl Plan {
    /// Extract a title from the first `# ...` heading in the markdown, if present.
    pub fn title(&self) -> &str {
        self.content
            .lines()
            .find(|line| line.starts_with("# "))
            .map(|line| line.trim_start_matches("# ").trim())
            .unwrap_or(&self.id)
    }
}

/// Legacy JSON plan format — kept for backward compatibility with existing .json plans.
#[derive(Debug, Clone, Deserialize)]
pub struct LegacyJsonPlan {
    pub id: String,
    #[serde(alias = "name")]
    pub title: String,
    #[serde(default)]
    pub plan: String,
    pub target_branch: Option<String>,
    pub base_branch: Option<String>,
}

impl From<LegacyJsonPlan> for Plan {
    fn from(legacy: LegacyJsonPlan) -> Self {
        // Use the freeform `plan` field as content, prefixed with the title
        let content = if legacy.plan.is_empty() {
            format!("# {}", legacy.title)
        } else {
            format!("# {}\n\n{}", legacy.title, legacy.plan)
        };
        Plan {
            id: legacy.id,
            content,
            target_branch: legacy.target_branch,
            base_branch: legacy.base_branch,
        }
    }
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
    /// Model used for the team lead (e.g. "opus", "sonnet")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead_model: Option<String>,
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
        // All values map to AgentTeam (only mode; "worktree" kept for backwards compat)
        let _s = String::deserialize(deserializer)?;
        Ok(ExecutionMode::AgentTeam)
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
    fn test_plan_from_markdown() {
        let plan = Plan {
            id: "my-feature".to_string(),
            content: "# My Feature\n\n## Goal\nBuild X\n\n## Requirements\n- Thing A".to_string(),
            target_branch: Some("feature/my-feature".to_string()),
            base_branch: Some("main".to_string()),
        };

        assert_eq!(plan.id, "my-feature");
        assert_eq!(plan.title(), "My Feature");
        assert!(plan.content.contains("## Goal"));
    }

    #[test]
    fn test_plan_title_fallback() {
        let plan = Plan {
            id: "no-heading".to_string(),
            content: "Just some content without a heading".to_string(),
            target_branch: None,
            base_branch: None,
        };

        // Falls back to id when no heading is present
        assert_eq!(plan.title(), "no-heading");
    }

    #[test]
    fn test_legacy_json_plan_conversion() {
        let json = r###"{
            "id": "my-feature",
            "title": "My Feature",
            "version": "1.0.0",
            "target_branch": "feature/my-feature",
            "base_branch": "main",
            "plan": "## Goal\nBuild X\n\n## Requirements\n- Thing A",
            "tasks": [
                {"title": "Task A", "description": "Do A"}
            ]
        }"###;

        let legacy: LegacyJsonPlan = serde_json::from_str(json).unwrap();
        let plan: Plan = legacy.into();
        assert_eq!(plan.id, "my-feature");
        assert_eq!(plan.title(), "My Feature");
        assert!(plan.content.contains("## Goal"));
        assert!(plan.content.contains("Build X"));
    }

    #[test]
    fn test_legacy_json_plan_empty_plan_field() {
        let json = r#"{
            "id": "minimal",
            "title": "Minimal PRD",
            "plan": ""
        }"#;

        let legacy: LegacyJsonPlan = serde_json::from_str(json).unwrap();
        let plan: Plan = legacy.into();
        assert_eq!(plan.id, "minimal");
        assert_eq!(plan.content, "# Minimal PRD");
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
