mod request;
mod sse_parser;

use anyhow::{Context, Result};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::broadcast;

use super::types::{ContentBlock, Message, MessageContent, MessagesRequest, UsageStats};
use crate::webui::auth::credentials::Credentials;

use request::build_request;
use sse_parser::parse_sse_stream;

/// Tracks an in-flight tool_use content block during SSE streaming.
pub(crate) struct ToolUseAccumulator {
    pub id: String,
    pub name: String,
    pub input_json: String,
}

/// Make a non-streaming Messages API call and return the assistant message
/// and usage statistics.
pub async fn call_messages(
    creds: &Credentials,
    request: &MessagesRequest,
) -> Result<(Message, UsageStats)> {
    let response = build_request(creds, request).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic API error ({status}): {body}");
    }

    let body: serde_json::Value = response
        .json()
        .await
        .context("Parsing Anthropic API response")?;

    let usage = UsageStats {
        input_tokens: body["usage"]["input_tokens"].as_u64().unwrap_or(0),
        output_tokens: body["usage"]["output_tokens"].as_u64().unwrap_or(0),
        cache_creation_input_tokens: body["usage"]["cache_creation_input_tokens"]
            .as_u64()
            .unwrap_or(0),
        cache_read_input_tokens: body["usage"]["cache_read_input_tokens"]
            .as_u64()
            .unwrap_or(0),
    };

    let content_blocks: Vec<ContentBlock> = body["content"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|b| {
                    if b["type"].as_str() == Some("text") {
                        Some(ContentBlock::Text {
                            text: b["text"].as_str().unwrap_or("").to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let message = Message {
        role: "assistant".to_string(),
        content: MessageContent::Blocks(content_blocks),
    };

    Ok((message, usage))
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
    let response = build_request(creds, request).await?;

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

    parse_sse_stream(response, tx, abort_flag).await
}
