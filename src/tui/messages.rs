use serde::{Deserialize, Serialize};

/// Message types in the chat interface
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    /// User prompt
    User(String),
    /// Assistant text response
    Assistant(String),
    /// Tool use event (tool name, args summary)
    ToolUse { tool: String, args: String },
    /// Tool result event (success/failure)
    ToolResult { success: bool, result: String },
    /// Error from Claude process
    Error(String),
}

/// Claude stream-json event types
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ClaudeEvent {
    #[serde(rename = "assistant")]
    Assistant { text: String },

    #[serde(rename = "tool_use")]
    ToolUse {
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        #[serde(default)]
        content: String,
        #[serde(default)]
        is_error: bool,
    },

    #[serde(rename = "error")]
    Error { message: String },
}

impl Message {
    /// Create a message from a Claude stream event
    #[allow(dead_code)]
    pub fn from_claude_event(event: ClaudeEvent) -> Self {
        match event {
            ClaudeEvent::Assistant { text } => Message::Assistant(text),
            ClaudeEvent::ToolUse { name, input } => {
                let args = serde_json::to_string_pretty(&input).unwrap_or_else(|_| "{}".to_string());
                Message::ToolUse { tool: name, args }
            }
            ClaudeEvent::ToolResult { content, is_error } => Message::ToolResult {
                success: !is_error,
                result: content,
            },
            ClaudeEvent::Error { message } => Message::Error(message),
        }
    }
}
