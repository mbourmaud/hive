use crate::webui::anthropic::types::{ContentBlock, Message, MessageContent};

const TASK_COMPLETE_SIGNAL: &str = "TASK_COMPLETE";
const TASK_BLOCKED_SIGNAL: &str = "TASK_BLOCKED";

/// Check assistant messages for completion/blocked signals.
pub fn check_completion(messages: &[Message]) -> (bool, Option<String>) {
    for msg in messages.iter().rev() {
        if msg.role != "assistant" {
            continue;
        }
        let text = extract_text(msg);
        if text.contains(TASK_COMPLETE_SIGNAL) {
            return (true, None);
        }
        if let Some(idx) = text.find(TASK_BLOCKED_SIGNAL) {
            let reason = text[idx + TASK_BLOCKED_SIGNAL.len()..].trim().to_string();
            return (false, Some(reason));
        }
    }

    // If the last assistant message has no tool_use, consider it done
    if let Some(last) = messages.last() {
        if last.role == "assistant" && !has_tool_use(last) {
            return (true, None);
        }
    }

    (false, None)
}

/// Extract text content from a message.
fn extract_text(msg: &Message) -> String {
    match &msg.content {
        MessageContent::Text(t) => t.clone(),
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Check if a message contains tool_use blocks.
fn has_tool_use(msg: &Message) -> bool {
    match &msg.content {
        MessageContent::Text(_) => false,
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. })),
    }
}

/// Extract a progress summary from conversation messages.
pub fn extract_progress_summary(messages: &[Message]) -> String {
    let mut summary = String::new();
    for msg in messages {
        if msg.role != "assistant" {
            continue;
        }
        let text = extract_text(msg);
        if !text.is_empty() {
            let trimmed = if text.len() > 500 {
                &text[text.len() - 500..]
            } else {
                &text
            };
            summary.push_str(trimmed);
            summary.push('\n');
        }
    }
    if summary.len() > 2000 {
        summary.truncate(2000);
    }
    summary
}

/// Extract a completion summary from the last assistant message containing TASK_COMPLETE.
/// Used for inter-worker notes.
pub fn extract_completion_summary(messages: &[Message]) -> String {
    for msg in messages.iter().rev() {
        if msg.role != "assistant" {
            continue;
        }
        let text = extract_text(msg);
        if text.contains(TASK_COMPLETE_SIGNAL) {
            // Take up to 500 chars of the message before the signal
            let before_signal = text
                .find(TASK_COMPLETE_SIGNAL)
                .map(|idx| &text[..idx])
                .unwrap_or(&text);
            let trimmed = before_signal.trim();
            if trimmed.len() > 500 {
                return trimmed[trimmed.len() - 500..].to_string();
            }
            return trimmed.to_string();
        }
    }

    // Fallback: last assistant text
    for msg in messages.iter().rev() {
        if msg.role != "assistant" {
            continue;
        }
        let text = extract_text(msg);
        if !text.is_empty() {
            if text.len() > 500 {
                return text[text.len() - 500..].to_string();
            }
            return text;
        }
    }

    String::new()
}
