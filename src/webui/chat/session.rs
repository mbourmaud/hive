use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::webui::anthropic::types::{Message, ToolDefinition};
use crate::webui::mcp_client::pool::McpPool;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Idle,
    Busy,
    Error(String),
}

/// Effort level controlling thinking budget.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    Low,
    Medium,
    High,
}

impl Effort {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    /// Whether extended thinking should be enabled for this effort level.
    pub fn thinking_enabled(self) -> bool {
        matches!(self, Self::Medium | Self::High)
    }

    /// Budget tokens for the thinking API.
    pub fn thinking_budget(self) -> u32 {
        match self {
            Self::Low => 0,
            Self::Medium => 10_000,
            Self::High => 32_000,
        }
    }
}

/// Chat mode controlling tool availability and output format.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatMode {
    /// Regular coding assistant — all tools available
    Code,
    /// Hive structured plan — no tools, outputs drone-parseable markdown
    HivePlan,
    /// Claude freeform plan — no tools, outputs freeform markdown
    Plan,
}

impl ChatMode {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "code" => Some(Self::Code),
            "hive-plan" => Some(Self::HivePlan),
            "plan" => Some(Self::Plan),
            _ => None,
        }
    }

    /// Whether tools should be disabled in this mode.
    pub fn tools_disabled(self) -> bool {
        matches!(self, Self::HivePlan | Self::Plan)
    }
}

/// A chat session backed by the Anthropic Messages API.
pub struct ChatSession {
    pub id: String,
    pub cwd: PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: SessionStatus,
    pub tx: broadcast::Sender<String>,
    pub title: Option<String>,
    /// Conversation history for multi-turn
    pub messages: Vec<Message>,
    /// Model short name (e.g. "sonnet", "opus", "haiku")
    pub model: String,
    /// Optional system prompt
    pub system_prompt: Option<String>,
    /// Abort flag for cancelling in-flight requests
    pub abort_flag: Arc<AtomicBool>,
    /// Tool definitions available in this session (built-in + MCP)
    pub tools: Vec<ToolDefinition>,
    /// Current effort level (controls thinking budget)
    pub effort: Effort,
    /// Current chat mode (controls tool availability and output format)
    pub chat_mode: ChatMode,
    /// Cumulative token usage for this session
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    /// Optional tool whitelist (if set, only these tools are allowed)
    pub allowed_tools: Option<Vec<String>>,
    /// Optional tool blacklist
    pub disallowed_tools: Option<Vec<String>>,
    /// Optional max agentic turns override (default: 25)
    pub max_turns: Option<usize>,
    /// Per-session MCP connection pool
    pub mcp_pool: Option<Arc<tokio::sync::Mutex<McpPool>>>,
    /// Agent name (if loaded from .claude/agents/)
    pub agent: Option<String>,
}

pub type SessionStore = Arc<Mutex<HashMap<String, ChatSession>>>;
