use garde::Validate;
use serde::{Deserialize, Serialize};

use super::session::SessionStatus;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSessionRequest {
    #[garde(length(min = 1, max = 4096))]
    pub cwd: String,
    #[serde(default = "default_model")]
    #[garde(length(max = 100))]
    pub model: String,
    #[serde(default)]
    #[garde(skip)]
    pub system_prompt: Option<String>,
    /// Agent filename (without .md) to load from .claude/agents/
    #[serde(default)]
    #[garde(skip)]
    pub agent: Option<String>,
    /// Max agentic tool-use turns (default: 25)
    #[serde(default)]
    #[garde(skip)]
    pub max_turns: Option<usize>,
}

fn default_model() -> String {
    "sonnet".to_string()
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub status: SessionStatus,
    pub cwd: String,
    pub created_at: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SendMessageRequest {
    #[garde(length(min = 1))]
    pub text: String,
    #[serde(default)]
    #[garde(skip)]
    pub model: Option<String>,
    #[serde(default)]
    #[garde(skip)]
    pub images: Vec<ImageAttachmentPayload>,
    /// Effort level: "low", "medium", "high" (controls thinking budget)
    #[serde(default)]
    #[garde(skip)]
    pub effort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImageAttachmentPayload {
    pub data: String,       // base64 data (without data URL prefix)
    pub media_type: String, // "image/png", "image/jpeg", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListItem {
    pub id: String,
    pub status: String,
    pub cwd: String,
    pub created_at: String,
    pub updated_at: String,
    pub title: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSessionRequest {
    #[serde(default)]
    #[garde(skip)]
    pub title: Option<String>,
    #[serde(default)]
    #[garde(skip)]
    pub system_prompt: Option<String>,
}
