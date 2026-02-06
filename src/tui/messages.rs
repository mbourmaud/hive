use serde::{Deserialize, Serialize};

/// Represents different types of messages in the chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// User-sent message
    User {
        content: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Assistant response message
    Assistant {
        content: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Tool use event from Claude
    ToolUse {
        tool_name: String,
        args_summary: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Tool result event
    ToolResult {
        success: bool,
        output_summary: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Error message
    Error {
        content: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// System message (bash output, command feedback)
    System {
        content: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl Message {
    pub fn user(content: String) -> Self {
        Message::User {
            content,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn assistant(content: String) -> Self {
        Message::Assistant {
            content,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn tool_use(tool_name: String, args_summary: String) -> Self {
        Message::ToolUse {
            tool_name,
            args_summary,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn tool_result(success: bool, output_summary: String) -> Self {
        Message::ToolResult {
            success,
            output_summary,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(content: String) -> Self {
        Message::Error {
            content,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn system(content: String) -> Self {
        Message::System {
            content,
            timestamp: chrono::Utc::now(),
        }
    }
}
