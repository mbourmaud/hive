use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone)]
pub enum ToolStatus {
    Running,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub enum ChatMessage {
    Text {
        role: MessageRole,
        content: String,
        timestamp: DateTime<Utc>,
    },
    ToolUse {
        tool_name: String,
        args_summary: String,
        status: ToolStatus,
        timestamp: DateTime<Utc>,
    },
    ToolResult {
        tool_name: String,
        success: bool,
        output_preview: String,
        timestamp: DateTime<Utc>,
    },
    Error {
        message: String,
        timestamp: DateTime<Utc>,
    },
}

impl ChatMessage {
    pub fn user(content: String) -> Self {
        Self::Text {
            role: MessageRole::User,
            content,
            timestamp: Utc::now(),
        }
    }

    pub fn assistant(content: String) -> Self {
        Self::Text {
            role: MessageRole::Assistant,
            content,
            timestamp: Utc::now(),
        }
    }

    pub fn system(content: String) -> Self {
        Self::Text {
            role: MessageRole::System,
            content,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self::Error {
            message,
            timestamp: Utc::now(),
        }
    }
}
