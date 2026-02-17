//! Provider dispatch — routes API calls to the correct backend (Anthropic or Bedrock).

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;

use super::anthropic::types::{Message, MessagesRequest, UsageStats};
use crate::webui::auth::credentials::Credentials;

/// Stream a Messages API request through the appropriate provider.
pub async fn stream_messages(
    creds: &Credentials,
    request: &MessagesRequest,
    tx: &broadcast::Sender<String>,
    session_id: &str,
    abort_flag: &Arc<AtomicBool>,
) -> Result<(Message, UsageStats, String)> {
    match creds {
        Credentials::Bedrock { .. } | Credentials::BedrockProfile { .. } => {
            super::bedrock::stream_messages(creds, request, tx, session_id, abort_flag).await
        }
        _ => {
            super::anthropic::client::stream_messages(creds, request, tx, session_id, abort_flag)
                .await
        }
    }
}

/// Resolve a short model alias to a full model ID based on the provider.
pub fn resolve_model(short: &str, creds: &Credentials) -> String {
    match creds {
        Credentials::Bedrock { .. } | Credentials::BedrockProfile { .. } => {
            super::bedrock::model::resolve_bedrock_model(short).to_string()
        }
        _ => super::anthropic::model::resolve_model(short).to_string(),
    }
}
