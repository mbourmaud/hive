//! Shared event processing logic for Anthropic-format SSE/EventStream payloads.
//!
//! Both the Anthropic SSE parser and the Bedrock EventStream parser delegate
//! to this module for event dispatch, accumulation, and final message building.

use std::collections::HashMap;

use tokio::sync::broadcast;

use super::super::types::{ContentBlock, Message, MessageContent, UsageStats};
use super::ToolUseAccumulator;

/// Accumulated state during streaming.
pub(crate) struct EventAccumulator {
    pub text: String,
    pub thinking: String,
    pub thinking_signature: String,
    pub usage: UsageStats,
    pub stop_reason: String,
    pub tool_accumulators: HashMap<u64, ToolUseAccumulator>,
    pub tool_use_blocks: Vec<ContentBlock>,
}

impl EventAccumulator {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            thinking: String::new(),
            thinking_signature: String::new(),
            usage: UsageStats::default(),
            stop_reason: String::from("end_turn"),
            tool_accumulators: HashMap::new(),
            tool_use_blocks: Vec::new(),
        }
    }

    /// Build the final assistant message from accumulated state.
    pub fn into_result(self) -> (Message, UsageStats, String) {
        let mut content_blocks = Vec::new();
        if !self.thinking.is_empty() {
            content_blocks.push(ContentBlock::Thinking {
                thinking: self.thinking,
                signature: self.thinking_signature,
            });
        }
        if !self.text.is_empty() {
            content_blocks.push(ContentBlock::Text { text: self.text });
        }
        let mut tool_blocks = self.tool_use_blocks;
        content_blocks.append(&mut tool_blocks);

        if content_blocks.is_empty() {
            content_blocks.push(ContentBlock::Text {
                text: ".".to_string(),
            });
        }

        (
            Message {
                role: "assistant".to_string(),
                content: MessageContent::Blocks(content_blocks),
            },
            self.usage,
            self.stop_reason,
        )
    }
}

/// Process a single Anthropic-format event (works for both SSE and Bedrock).
pub(crate) fn process_event(
    event_type: &str,
    event_data: &str,
    tx: &broadcast::Sender<String>,
    acc: &mut EventAccumulator,
) {
    match event_type {
        "message_start" => handle_message_start(event_data, &mut acc.usage),
        "content_block_start" => handle_block_start(event_data, &mut acc.tool_accumulators),
        "content_block_delta" => handle_block_delta(
            event_data,
            tx,
            &mut acc.text,
            &mut acc.thinking,
            &mut acc.thinking_signature,
            &mut acc.tool_accumulators,
        ),
        "content_block_stop" => handle_block_stop(
            event_data,
            tx,
            &mut acc.tool_accumulators,
            &mut acc.tool_use_blocks,
        ),
        "message_delta" => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(event_data) {
                if let Some(n) = val.pointer("/usage/output_tokens").and_then(|v| v.as_u64()) {
                    acc.usage.output_tokens = n;
                }
                if let Some(sr) = val.pointer("/delta/stop_reason").and_then(|v| v.as_str()) {
                    acc.stop_reason = sr.to_string();
                }
            }
        }
        "message_stop" => {
            if acc.stop_reason != "tool_use" {
                let ev = serde_json::json!({"type":"result","subtype":"success","result":"","is_error":false});
                let _ = tx.send(ev.to_string());
            }
        }
        "error" => {
            if let Ok(err_val) = serde_json::from_str::<serde_json::Value>(event_data) {
                let err_msg = err_val
                    .pointer("/error/message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown API error");
                let ev = serde_json::json!({"type":"result","subtype":"error","result":err_msg,"is_error":true});
                let _ = tx.send(ev.to_string());
            }
        }
        _ => {}
    }
}

fn handle_message_start(event_data: &str, usage: &mut UsageStats) {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(event_data) else {
        return;
    };
    let Some(u) = val.pointer("/message/usage") else {
        return;
    };
    if let Some(n) = u.get("input_tokens").and_then(|v| v.as_u64()) {
        usage.input_tokens = n;
    }
    if let Some(n) = u
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_u64())
    {
        usage.cache_creation_input_tokens = n;
    }
    if let Some(n) = u.get("cache_read_input_tokens").and_then(|v| v.as_u64()) {
        usage.cache_read_input_tokens = n;
    }
}

fn handle_block_start(event_data: &str, tool_accumulators: &mut HashMap<u64, ToolUseAccumulator>) {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(event_data) else {
        return;
    };
    let index = val.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
    let Some(cb) = val.get("content_block") else {
        return;
    };
    if cb.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
        return;
    }
    let id = cb
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = cb
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    tool_accumulators.insert(
        index,
        ToolUseAccumulator {
            id,
            name,
            input_json: String::new(),
        },
    );
}

fn handle_block_delta(
    event_data: &str,
    tx: &broadcast::Sender<String>,
    accumulated_text: &mut String,
    accumulated_thinking: &mut String,
    thinking_signature: &mut String,
    tool_accumulators: &mut HashMap<u64, ToolUseAccumulator>,
) {
    let Ok(delta) = serde_json::from_str::<serde_json::Value>(event_data) else {
        return;
    };
    let index = delta.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
    let Some(d) = delta.get("delta") else { return };
    let delta_type = d.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match delta_type {
        "text_delta" => {
            if let Some(text) = d.get("text").and_then(|t| t.as_str()) {
                accumulated_text.push_str(text);
                let event = serde_json::json!({
                    "type": "assistant",
                    "message": { "content": [{"type": "text", "text": text}] }
                });
                let _ = tx.send(event.to_string());
            }
        }
        "thinking_delta" => {
            if let Some(thinking) = d.get("thinking").and_then(|t| t.as_str()) {
                accumulated_thinking.push_str(thinking);
                let event = serde_json::json!({
                    "type": "assistant",
                    "message": { "content": [{"type": "thinking", "thinking": thinking}] }
                });
                let _ = tx.send(event.to_string());
            }
        }
        "signature_delta" => {
            if let Some(sig) = d.get("signature").and_then(|v| v.as_str()) {
                thinking_signature.push_str(sig);
            }
        }
        "input_json_delta" => {
            if let Some(json_fragment) = d.get("partial_json").and_then(|v| v.as_str()) {
                if let Some(acc) = tool_accumulators.get_mut(&index) {
                    acc.input_json.push_str(json_fragment);
                }
            }
        }
        _ => {}
    }
}

fn handle_block_stop(
    event_data: &str,
    tx: &broadcast::Sender<String>,
    tool_accumulators: &mut HashMap<u64, ToolUseAccumulator>,
    tool_use_blocks: &mut Vec<ContentBlock>,
) {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(event_data) {
        let index = val.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
        if let Some(acc) = tool_accumulators.remove(&index) {
            let input: serde_json::Value = serde_json::from_str(&acc.input_json)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

            let tool_event = serde_json::json!({
                "type": "assistant",
                "message": {
                    "content": [{
                        "type": "tool_use",
                        "id": acc.id,
                        "name": acc.name,
                        "input": input
                    }]
                }
            });
            let _ = tx.send(tool_event.to_string());

            tool_use_blocks.push(ContentBlock::ToolUse {
                id: acc.id,
                name: acc.name,
                input,
            });
        }
    }
}
