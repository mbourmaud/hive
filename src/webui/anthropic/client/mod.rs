mod request;
mod sse_parser;

use anyhow::Result;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::broadcast;

use super::types::{Message, MessagesRequest, UsageStats};
use crate::webui::auth::credentials::Credentials;

use request::build_request;
use sse_parser::parse_sse_stream;

/// Tracks an in-flight tool_use content block during SSE streaming.
pub(crate) struct ToolUseAccumulator {
    pub id: String,
    pub name: String,
    pub input_json: String,
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
