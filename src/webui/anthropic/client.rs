use std::collections::HashMap;

use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

use super::types::{ContentBlock, Message, MessageContent, MessagesRequest, UsageStats};
use crate::webui::auth::credentials::{self, Credentials};

/// Read user metadata from ~/.claude.json for OAuth requests.
fn read_claude_metadata() -> Option<(String, String)> {
    let home = dirs::home_dir()?;
    let path = home.join(".claude.json");
    let data = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&data).ok()?;
    let user_id = json.get("userID")?.as_str()?.to_string();
    let account_uuid = json
        .get("oauthAccount")
        .and_then(|o| o.get("accountUuid"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Some((user_id, account_uuid))
}

/// Tracks an in-flight tool_use content block during SSE streaming.
struct ToolUseAccumulator {
    id: String,
    name: String,
    input_json: String,
}

/// Stream a Messages API request, translating Anthropic SSE events to the
/// frontend event format and broadcasting them via `tx`. Returns the full
/// assistant message, usage statistics, and the stop reason ("end_turn",
/// "tool_use", or "max_tokens").
pub async fn stream_messages(
    creds: &Credentials,
    request: &MessagesRequest,
    tx: &broadcast::Sender<String>,
    session_id: &str,
    abort_flag: &Arc<AtomicBool>,
) -> Result<(Message, UsageStats, String)> {
    let is_oauth = matches!(creds, Credentials::OAuth { .. });
    let (auth_header_name, auth_header_value) = credentials::get_auth_header(creds).await?;

    // Build the request body — inject metadata for OAuth
    let mut body = serde_json::to_value(request).context("Serializing request")?;
    if is_oauth {
        if let Some((user_id, account_uuid)) = read_claude_metadata() {
            let meta_user_id = if account_uuid.is_empty() {
                user_id
            } else {
                format!("user_{user_id}_account_{account_uuid}")
            };
            body["metadata"] = serde_json::json!({ "user_id": meta_user_id });
        }
        if let Some(o) = body.as_object_mut() {
            o.remove("temperature");
            o.remove("tool_choice");
        }
    }

    let url = if is_oauth {
        "https://api.anthropic.com/v1/messages?beta=true"
    } else {
        "https://api.anthropic.com/v1/messages"
    };

    let client = reqwest::Client::new();
    let mut req_builder = client
        .post(url)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .header(auth_header_name, &auth_header_value);

    if is_oauth {
        req_builder = req_builder
            .header(
                "anthropic-beta",
                "oauth-2025-04-20,interleaved-thinking-2025-05-14",
            )
            .header("user-agent", "claude-cli/2.1.7 (external, cli)")
            .header("anthropic-dangerous-direct-browser-access", "true")
            .header("x-app", "cli")
            .header("x-stainless-arch", "x64")
            .header("x-stainless-lang", "js")
            .header("x-stainless-os", "Darwin")
            .header("x-stainless-package-version", "0.70.0")
            .header("x-stainless-runtime", "node")
            .header("x-stainless-runtime-version", "v24.3.0")
            .header("x-stainless-retry-count", "0")
            .header("x-stainless-timeout", "600")
            .header("x-stainless-helper-method", "stream")
            .header("accept", "application/json");
    }

    let response = req_builder
        .json(&body)
        .send()
        .await
        .context("Sending Anthropic API request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let error_event = serde_json::json!({
            "type": "result",
            "subtype": "error",
            "result": format!("Anthropic API error ({status}): {body}"),
            "is_error": true
        });
        let _ = tx.send(error_event.to_string());
        anyhow::bail!("Anthropic API error ({status}): {body}");
    }

    // Send init event
    let init_event = serde_json::json!({
        "type": "system",
        "subtype": "init",
        "session_id": session_id
    });
    let _ = tx.send(init_event.to_string());

    // Parse SSE stream
    let mut accumulated_text = String::new();
    let mut accumulated_thinking = String::new();
    let mut usage = UsageStats::default();
    let mut stop_reason = String::from("end_turn");
    let mut buffer = String::new();

    // Track in-flight tool_use blocks by content block index
    let mut tool_accumulators: HashMap<u64, ToolUseAccumulator> = HashMap::new();
    // Completed tool_use blocks for building the final message
    let mut tool_use_blocks: Vec<ContentBlock> = Vec::new();

    use futures_util::StreamExt;
    let mut byte_stream = response.bytes_stream();

    while let Some(chunk) = byte_stream.next().await {
        if abort_flag.load(Ordering::Relaxed) {
            break;
        }

        let chunk = chunk.context("Reading SSE chunk")?;
        let chunk_str = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_str);

        while let Some(event_end) = buffer.find("\n\n") {
            let event_block = buffer[..event_end].to_string();
            buffer = buffer[event_end + 2..].to_string();

            if abort_flag.load(Ordering::Relaxed) {
                break;
            }

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

            match event_type.as_str() {
                "message_start" => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&event_data) {
                        if let Some(u) = val.pointer("/message/usage") {
                            if let Some(n) = u.get("input_tokens").and_then(|v| v.as_u64()) {
                                usage.input_tokens = n;
                            }
                        }
                    }
                }
                "content_block_start" => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&event_data) {
                        let index = val.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                        if let Some(cb) = val.get("content_block") {
                            let block_type = cb.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            if block_type == "tool_use" {
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
                        }
                    }
                }
                "content_block_delta" => {
                    if let Ok(delta) = serde_json::from_str::<serde_json::Value>(&event_data) {
                        let index = delta.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                        if let Some(d) = delta.get("delta") {
                            let delta_type = d.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            match delta_type {
                                "text_delta" => {
                                    if let Some(text) = d.get("text").and_then(|t| t.as_str()) {
                                        accumulated_text.push_str(text);
                                        let assistant_event = serde_json::json!({
                                            "type": "assistant",
                                            "message": {
                                                "content": [{
                                                    "type": "text",
                                                    "text": text
                                                }]
                                            }
                                        });
                                        let _ = tx.send(assistant_event.to_string());
                                    }
                                }
                                "thinking_delta" => {
                                    if let Some(thinking) =
                                        d.get("thinking").and_then(|t| t.as_str())
                                    {
                                        accumulated_thinking.push_str(thinking);
                                        let thinking_event = serde_json::json!({
                                            "type": "assistant",
                                            "message": {
                                                "content": [{
                                                    "type": "thinking",
                                                    "thinking": thinking
                                                }]
                                            }
                                        });
                                        let _ = tx.send(thinking_event.to_string());
                                    }
                                }
                                "input_json_delta" => {
                                    if let Some(json_fragment) =
                                        d.get("partial_json").and_then(|v| v.as_str())
                                    {
                                        if let Some(acc) = tool_accumulators.get_mut(&index) {
                                            acc.input_json.push_str(json_fragment);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                "content_block_stop" => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&event_data) {
                        let index = val.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                        if let Some(acc) = tool_accumulators.remove(&index) {
                            let input: serde_json::Value = serde_json::from_str(&acc.input_json)
                                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                            // Broadcast tool_use event to frontend
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
                "message_delta" => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&event_data) {
                        if let Some(u) = val.get("usage") {
                            if let Some(n) = u.get("output_tokens").and_then(|v| v.as_u64()) {
                                usage.output_tokens = n;
                            }
                        }
                        if let Some(d) = val.get("delta") {
                            if let Some(sr) = d.get("stop_reason").and_then(|v| v.as_str()) {
                                stop_reason = sr.to_string();
                            }
                        }
                    }
                }
                "message_stop" => {
                    // Only send result event when we're truly done (end_turn),
                    // not on tool_use (the agentic loop will continue)
                    if stop_reason != "tool_use" {
                        let result_event = serde_json::json!({
                            "type": "result",
                            "subtype": "success",
                            "result": "",
                            "is_error": false,
                            "usage": {
                                "input_tokens": usage.input_tokens,
                                "output_tokens": usage.output_tokens
                            }
                        });
                        let _ = tx.send(result_event.to_string());
                    }
                }
                "error" => {
                    if let Ok(err_val) = serde_json::from_str::<serde_json::Value>(&event_data) {
                        let err_msg = err_val
                            .get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown API error");
                        let error_event = serde_json::json!({
                            "type": "result",
                            "subtype": "error",
                            "result": err_msg,
                            "is_error": true
                        });
                        let _ = tx.send(error_event.to_string());
                    }
                }
                _ => {}
            }
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
