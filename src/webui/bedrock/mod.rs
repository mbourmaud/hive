//! AWS Bedrock provider module.
//!
//! Implements `stream_messages()` with the same signature as the Anthropic
//! client, using SigV4-signed requests to the Bedrock InvokeModelWithResponseStream API.

pub mod aws_resolve;
pub mod model;
mod request;
mod stream_parser;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;

use crate::webui::anthropic::types::{Message, MessagesRequest, UsageStats};
use crate::webui::auth::credentials::Credentials;

use request::build_bedrock_request;
use stream_parser::parse_event_stream;

/// Maximum retries for transient API errors (429, 500, 503, 529).
const MAX_API_RETRIES: usize = 3;
/// Base delay between retries (exponential backoff: 2s, 4s, 8s).
const RETRY_BASE_DELAY_MS: u64 = 2000;

/// Stream a Messages API request via Bedrock, translating EventStream events
/// to the frontend format and broadcasting via `tx`. Returns the full assistant
/// message, usage statistics, and the stop reason.
pub async fn stream_messages(
    creds: &Credentials,
    request: &MessagesRequest,
    tx: &broadcast::Sender<String>,
    session_id: &str,
    abort_flag: &Arc<AtomicBool>,
) -> Result<(Message, UsageStats, String)> {
    let mut last_error = String::new();

    for attempt in 0..=MAX_API_RETRIES {
        if abort_flag.load(Ordering::Relaxed) {
            anyhow::bail!("Aborted");
        }

        let response = match build_bedrock_request(creds, request).await {
            Ok(r) => r,
            Err(e) => {
                if attempt < MAX_API_RETRIES {
                    let delay = RETRY_BASE_DELAY_MS * (1 << attempt);
                    eprintln!(
                        "[hive] Bedrock request failed (attempt {}/{}): {e:#}, retrying in {delay}ms",
                        attempt + 1,
                        MAX_API_RETRIES + 1
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }
                return Err(e);
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            last_error = format!("Bedrock API error ({status}): {body}");

            let is_retryable = status.as_u16() == 429
                || status.as_u16() == 500
                || status.as_u16() == 529
                || status.as_u16() == 503;

            if is_retryable && attempt < MAX_API_RETRIES {
                let delay = RETRY_BASE_DELAY_MS * (1 << attempt);
                eprintln!(
                    "[hive] Bedrock error {status} (attempt {}/{}), retrying in {delay}ms",
                    attempt + 1,
                    MAX_API_RETRIES + 1
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }

            let error_event = serde_json::json!({
                "type": "result",
                "subtype": "error",
                "result": &last_error,
                "is_error": true
            });
            let _ = tx.send(error_event.to_string());
            anyhow::bail!("{last_error}");
        }

        // Success — send init event and parse EventStream
        let init_event = serde_json::json!({
            "type": "system",
            "subtype": "init",
            "session_id": session_id
        });
        let _ = tx.send(init_event.to_string());

        return parse_event_stream(response, tx, abort_flag).await;
    }

    anyhow::bail!("{last_error}")
}
