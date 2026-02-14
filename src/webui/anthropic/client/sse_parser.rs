use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::broadcast;

use super::super::types::{ContentBlock, Message, MessageContent, UsageStats};
use super::ToolUseAccumulator;

/// Parse the SSE byte stream, broadcasting events and returning the final message.
pub(super) async fn parse_sse_stream(
    response: reqwest::Response,
    tx: &broadcast::Sender<String>,
    abort_flag: &Arc<AtomicBool>,
) -> Result<(Message, UsageStats, String)> {
    let mut accumulated_text = String::new();
    let mut accumulated_thinking = String::new();
    let mut thinking_signature = String::new();
    let mut usage = UsageStats::default();
    let mut stop_reason = String::from("end_turn");
    let mut buffer = String::new();

    let mut tool_accumulators: HashMap<u64, ToolUseAccumulator> = HashMap::new();
    let mut tool_use_blocks: Vec<ContentBlock> = Vec::new();

    use futures_util::StreamExt;
    let mut byte_stream = response.bytes_stream();

    loop {
        let chunk = tokio::select! {
            biased;
            _ = abort_notified(abort_flag) => break,
            next = byte_stream.next() => match next {
                Some(c) => c.context("Reading SSE chunk")?,
                None => break,
            },
        };
        let chunk_str = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_str);

        while let Some(event_end) = buffer.find("\n\n") {
            let event_block = buffer[..event_end].to_string();
            buffer = buffer[event_end + 2..].to_string();

            let mut event_type = String::new();
            let mut event_data = String::new();

            for line in event_block.lines() {
                if let Some(t) = line.strip_prefix("event: ") {
                    event_type = t.to_string();
                } else if let Some(d) = line.strip_prefix("data: ") {
                    event_data = d.to_string();
                }
            }

            if event_data.is_empty() {
                continue;
            }

            process_sse_event(
                &event_type,
                &event_data,
                tx,
                &mut accumulated_text,
                &mut accumulated_thinking,
                &mut thinking_signature,
                &mut usage,
                &mut stop_reason,
                &mut tool_accumulators,
                &mut tool_use_blocks,
            );
        }
    }

    if abort_flag.load(Ordering::Relaxed) {
        let abort_event = serde_json::json!({
            "type": "result",
            "subtype": "error",
            "result": "Aborted by user",
            "is_error": true
        });
        let _ = tx.send(abort_event.to_string());
    }

    // Build the final message with all content blocks in order
    let mut content_blocks = Vec::new();
    if !accumulated_thinking.is_empty() {
        content_blocks.push(ContentBlock::Thinking {
            thinking: accumulated_thinking,
            signature: thinking_signature,
        });
    }
    if !accumulated_text.is_empty() {
        content_blocks.push(ContentBlock::Text {
            text: accumulated_text,
        });
    }
    content_blocks.append(&mut tool_use_blocks);

    if content_blocks.is_empty() {
        content_blocks.push(ContentBlock::Text {
            text: String::new(),
        });
    }

    Ok((
        Message {
            role: "assistant".to_string(),
            content: MessageContent::Blocks(content_blocks),
        },
        usage,
        stop_reason,
    ))
}

#[allow(clippy::too_many_arguments)]
fn process_sse_event(
    event_type: &str,
    event_data: &str,
    tx: &broadcast::Sender<String>,
    accumulated_text: &mut String,
    accumulated_thinking: &mut String,
    thinking_signature: &mut String,
    usage: &mut UsageStats,
    stop_reason: &mut String,
    tool_accumulators: &mut HashMap<u64, ToolUseAccumulator>,
    tool_use_blocks: &mut Vec<ContentBlock>,
) {
    match event_type {
        "message_start" => handle_message_start(event_data, usage),
        "content_block_start" => handle_block_start(event_data, tool_accumulators),
        "content_block_delta" => handle_block_delta(
            event_data,
            tx,
            accumulated_text,
            accumulated_thinking,
            thinking_signature,
            tool_accumulators,
        ),
        "content_block_stop" => {
            handle_block_stop(event_data, tx, tool_accumulators, tool_use_blocks)
        }
        "message_delta" => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(event_data) {
                if let Some(n) = val.pointer("/usage/output_tokens").and_then(|v| v.as_u64()) {
                    usage.output_tokens = n;
                }
                if let Some(sr) = val.pointer("/delta/stop_reason").and_then(|v| v.as_str()) {
                    *stop_reason = sr.to_string();
                }
            }
        }
        "message_stop" => {
            if stop_reason != "tool_use" {
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

/// Poll the abort flag at 50ms intervals, returning when it becomes `true`.
async fn abort_notified(flag: &AtomicBool) {
    loop {
        if flag.load(Ordering::Relaxed) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
