use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: Instant,
}

impl ChatMessage {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            role,
            content,
            timestamp: Instant::now(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content.into())
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content.into())
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content.into())
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Error, content.into())
    }
}

#[derive(Debug, Clone)]
pub struct ToolUseInfo {
    pub tool_name: String,
    pub tool_id: String,
    pub input_preview: String,
}

#[derive(Debug, Clone)]
pub enum ClaudeEvent {
    AssistantText(String),
    ToolUse(ToolUseInfo),
    ToolResult {
        tool_id: String,
        output_preview: String,
        is_error: bool,
    },
    PermissionRequest {
        tool_name: String,
        args_preview: String,
    },
    Finished,
    Error(String),
}
