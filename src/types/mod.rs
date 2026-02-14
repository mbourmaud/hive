use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Task type for structured plans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskType {
    /// Environment setup (install deps, verify build) — handled by Hive before launch
    Setup,
    /// PR/MR creation — handled by Hive after all work tasks complete
    Pr,
    /// Implementation work — dispatched to teammates
    Work,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Setup => write!(f, "setup"),
            TaskType::Pr => write!(f, "pr"),
            TaskType::Work => write!(f, "work"),
        }
    }
}

/// A structured task parsed from a plan's `## Tasks` section.
#[derive(Debug, Clone)]
pub struct StructuredTask {
    /// Task number (from `### N. Title`)
    pub number: usize,
    /// Task title (from `### N. Title`)
    pub title: String,
    /// Task body text (everything after metadata bullets)
    pub body: String,
    /// Task type: setup, pr, or work (default: work)
    pub task_type: TaskType,
    /// Model to use for this task (e.g., "sonnet", "haiku", "opus")
    pub model: Option<String>,
    /// Whether this task can run in parallel with other parallel tasks
    pub parallel: bool,
    /// Files this task owns / will modify
    pub files: Vec<String>,
    /// Task numbers this task depends on
    pub depends_on: Vec<usize>,
}

/// Plan — a markdown file with metadata extracted from content/filename.
#[derive(Debug, Clone)]
pub struct Plan {
    pub id: String,
    /// Raw markdown content — sent directly to the team lead as prompt
    pub content: String,
    pub target_branch: Option<String>,
    /// Base branch to create worktree from (defaults to origin/master or origin/main)
    pub base_branch: Option<String>,
    /// Structured tasks parsed from `## Tasks` section
    pub structured_tasks: Vec<StructuredTask>,
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
            structured_tasks: Vec::new(),
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
mod tests;
