use crate::webui::anthropic::types::{ContentBlock, Message, MessageContent};

/// Default max input tokens (80% of 200K context window).
const DEFAULT_TRUNCATION_THRESHOLD: u64 = 160_000;

/// Estimate token count for a message using char count / 4.
fn estimate_message_tokens(msg: &Message) -> u64 {
    let chars = match &msg.content {
        MessageContent::Text(s) => s.len(),
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .map(|b| match b {
                ContentBlock::Text { text } => text.len(),
                ContentBlock::Thinking { thinking, .. } => thinking.len(),
                ContentBlock::ToolUse { input, .. } => input.to_string().len(),
                ContentBlock::ToolResult { content, .. } => content.len(),
                ContentBlock::Image { .. } => 1000, // rough estimate for image tokens
            })
            .sum(),
    };
    (chars as u64) / 4
}

/// Estimate total tokens for a conversation.
pub fn estimate_total_tokens(messages: &[Message]) -> u64 {
    messages.iter().map(estimate_message_tokens).sum()
}

/// Truncate conversation history to fit within a token budget.
///
/// Strategy:
/// 1. Keep the first user message (establishes context).
/// 2. Keep all messages from the last 3 turns (6 messages: user+assistant pairs).
/// 3. For old tool results, replace content with a truncation notice.
/// 4. If still over budget, drop oldest message pairs.
pub fn truncate_messages(messages: &[Message], estimated_tokens: u64) -> Vec<Message> {
    if estimated_tokens < DEFAULT_TRUNCATION_THRESHOLD || messages.len() <= 6 {
        return messages.to_vec();
    }

    let keep_tail = 6.min(messages.len());
    let tail_start = messages.len() - keep_tail;

    let mut result = Vec::with_capacity(messages.len());

    // Always keep first message
    if let Some(first) = messages.first() {
        result.push(first.clone());
    }

    // Process middle messages: truncate tool results
    let middle_start = 1.min(tail_start);
    for msg in &messages[middle_start..tail_start] {
        result.push(truncate_tool_results(msg));
    }

    // Keep tail messages as-is
    for msg in &messages[tail_start..] {
        result.push(msg.clone());
    }

    // If still over budget, drop oldest middle messages
    let mut current_estimate = estimate_total_tokens(&result);
    while current_estimate > DEFAULT_TRUNCATION_THRESHOLD && result.len() > keep_tail + 1 {
        // Remove the second message (index 1, after the kept first message)
        result.remove(1);
        current_estimate = estimate_total_tokens(&result);
    }

    result
}

/// Replace large tool result contents with a truncation notice.
fn truncate_tool_results(msg: &Message) -> Message {
    match &msg.content {
        MessageContent::Blocks(blocks) => {
            let truncated: Vec<ContentBlock> = blocks
                .iter()
                .map(|b| match b {
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } if content.len() > 500 => ContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: format!("[result truncated — {} chars]", content.len()),
                        is_error: *is_error,
                    },
                    other => other.clone(),
                })
                .collect();
            Message {
                role: msg.role.clone(),
                content: MessageContent::Blocks(truncated),
            }
        }
        _ => msg.clone(),
    }
}
