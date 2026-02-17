use crate::webui::anthropic::types::{ContentBlock, Message, MessageContent};

use super::compressor;

/// Default max input tokens (80% of 200K context window).
const DEFAULT_TRUNCATION_THRESHOLD: u64 = 160_000;

/// At 60% of context window, start proactively compressing middle tool results.
const PROACTIVE_THRESHOLD: u64 = 120_000;

/// Tool results above this char count get replaced in middle messages.
const TOOL_RESULT_TRUNCATION_CHARS: usize = 200;

/// At this token estimate, apply compressor to tail messages too.
const TAIL_COMPRESSION_THRESHOLD: u64 = 140_000;

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
                ContentBlock::Image { .. } => 1000,
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
/// 4. At proactive threshold (120K), aggressively compress middle results.
/// 5. At tail threshold (140K), apply compressor to tail messages.
/// 6. If still over budget, drop oldest message pairs.
pub fn truncate_messages(messages: &[Message], estimated_tokens: u64) -> Vec<Message> {
    if estimated_tokens < PROACTIVE_THRESHOLD || messages.len() <= 6 {
        return messages.to_vec();
    }

    let keep_tail = 6.min(messages.len());
    let tail_start = messages.len() - keep_tail;
    let proactive = estimated_tokens >= PROACTIVE_THRESHOLD;

    let mut result = Vec::with_capacity(messages.len());

    // Always keep first message
    if let Some(first) = messages.first() {
        result.push(first.clone());
    }

    // Process middle messages: truncate/compress tool results
    let middle_start = 1.min(tail_start);
    for msg in &messages[middle_start..tail_start] {
        if proactive {
            result.push(compress_middle_tool_results(msg));
        } else {
            result.push(truncate_tool_results(msg));
        }
    }

    // Keep tail messages, with optional compression at high pressure
    for msg in &messages[tail_start..] {
        if estimated_tokens >= TAIL_COMPRESSION_THRESHOLD {
            result.push(compress_tail_tool_results(msg));
        } else {
            result.push(msg.clone());
        }
    }

    // If still over budget, drop oldest middle messages
    let mut current_estimate = estimate_total_tokens(&result);
    while current_estimate > DEFAULT_TRUNCATION_THRESHOLD && result.len() > keep_tail + 1 {
        result.remove(1);
        current_estimate = estimate_total_tokens(&result);
    }

    result
}

/// Replace large tool result contents with a truncation notice (standard).
fn truncate_tool_results(msg: &Message) -> Message {
    replace_tool_results(msg, |content| {
        if content.len() > TOOL_RESULT_TRUNCATION_CHARS {
            format!("[result truncated - {} chars]", content.len())
        } else {
            content.to_string()
        }
    })
}

/// Aggressively compress middle tool results at proactive threshold.
/// Replace all tool results with a minimal summary.
fn compress_middle_tool_results(msg: &Message) -> Message {
    replace_tool_results(msg, |content| {
        if content.len() > TOOL_RESULT_TRUNCATION_CHARS {
            format!("[output: {} chars]", content.len())
        } else {
            content.to_string()
        }
    })
}

/// Apply the compressor to tail tool results under high context pressure.
fn compress_tail_tool_results(msg: &Message) -> Message {
    replace_tool_results(msg, |content| {
        compressor::compress_tool_output(content, false)
    })
}

/// Helper: apply a transform function to all tool result blocks in a message.
fn replace_tool_results(msg: &Message, transform: impl Fn(&str) -> String) -> Message {
    match &msg.content {
        MessageContent::Blocks(blocks) => {
            let transformed: Vec<ContentBlock> = blocks
                .iter()
                .map(|b| match b {
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } => ContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: transform(content),
                        is_error: *is_error,
                    },
                    other => other.clone(),
                })
                .collect();
            Message {
                role: msg.role.clone(),
                content: MessageContent::Blocks(transformed),
            }
        }
        _ => msg.clone(),
    }
}
