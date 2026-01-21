use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PRD (Product Requirements Document) structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prd {
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
    pub stories: Vec<Story>,
}

/// Individual story within a PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub acceptance_criteria: Option<Vec<String>>,
    #[serde(default)]
    pub definition_of_done: Vec<String>,
    #[serde(default)]
    pub verification_commands: Vec<String>,
    pub notes: Option<String>,
}

/// Drone execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneStatus {
    pub drone: String,
    pub prd: String,
    pub branch: String,
    pub worktree: String,
    pub local_mode: bool,
    pub status: DroneState,
    pub current_story: Option<String>,
    pub completed: Vec<String>,
    pub story_times: HashMap<String, StoryTiming>,
    pub total: usize,
    pub started: String,
    pub updated: String,
    pub error_count: usize,
    pub last_error_story: Option<String>,
    pub blocked_reason: Option<String>,
    pub blocked_questions: Vec<String>,
    pub awaiting_human: bool,
}

/// Story timing information
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
    Blocked,
    Stopped,
}

impl std::fmt::Display for DroneState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DroneState::Starting => write!(f, "starting"),
            DroneState::Resuming => write!(f, "resuming"),
            DroneState::InProgress => write!(f, "in_progress"),
            DroneState::Completed => write!(f, "completed"),
            DroneState::Error => write!(f, "error"),
            DroneState::Blocked => write!(f, "blocked"),
            DroneState::Stopped => write!(f, "stopped"),
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
    fn test_parse_prd() {
        let json = r#"{
            "id": "test-prd",
            "title": "Test PRD",
            "description": "A test PRD",
            "version": "1.0.0",
            "created_at": "2024-01-01T00:00:00Z",
            "stories": [
                {
                    "id": "TEST-001",
                    "title": "Test Story",
                    "description": "A test story",
                    "definition_of_done": ["Done"],
                    "verification_commands": ["echo test"]
                }
            ]
        }"#;

        let prd: Prd = serde_json::from_str(json).unwrap();
        assert_eq!(prd.id, "test-prd");
        assert_eq!(prd.stories.len(), 1);
        assert_eq!(prd.stories[0].id, "TEST-001");
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
            "last_error_story": null,
            "blocked_reason": null,
            "blocked_questions": [],
            "awaiting_human": false
        }"#;

        let status: DroneStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.drone, "test-drone");
        assert_eq!(status.status, DroneState::InProgress);
        assert_eq!(status.current_story, Some("TEST-001".to_string()));
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
        assert_eq!(DroneState::Blocked.to_string(), "blocked");
        assert_eq!(DroneState::Stopped.to_string(), "stopped");
    }

    #[test]
    fn test_default_config() {
        let config = HiveConfig::default();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.default_model, Some("sonnet".to_string()));
        assert!(config.project.is_none());
    }
}
