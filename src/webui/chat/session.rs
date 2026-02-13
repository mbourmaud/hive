use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::webui::anthropic::types::{Message, ToolDefinition};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Idle,
    Busy,
    Error(String),
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
}

pub type SessionStore = Arc<Mutex<HashMap<String, ChatSession>>>;
