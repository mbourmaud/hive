use axum::{
    extract::{Path, State},
    Json,
};

use crate::webui::anthropic::{self, types::*};
use crate::webui::auth::credentials;
use crate::webui::error::{ApiError, ApiResult};

use super::super::persistence::{self, save_messages, update_meta_tokens};
use super::super::session::{SessionStatus, SessionStore};
use super::sessions::restore_session_from_disk;

const COMPACT_PROMPT: &str = "\
Summarize this conversation for a continuation prompt. Include:

## Goal — what the user is trying to accomplish

## Accomplished — what work has been completed

## In Progress — what's still being worked on

## Relevant Files — files read, edited, or created

## Key Decisions — important choices made

Keep it concise but preserve all actionable context.";

/// POST /api/chat/sessions/{id}/compact
pub async fn compact_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let creds = match credentials::load_credentials() {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Err(ApiError::Unauthorized(
                "No credentials configured".to_string(),
            ));
        }
        Err(e) => {
            return Err(ApiError::Internal(e.context("Failed to load credentials")));
        }
    };

    // Restore from disk if not in memory
    {
        let sessions = store.lock().await;
        if !sessions.contains_key(&id) {
            drop(sessions);
            if restore_session_from_disk(&store, &id).await.is_none() {
                return Err(ApiError::NotFound(format!("Session '{id}' not found")));
            }
        }
    }

    // Extract messages and model, validate session is idle
    let (messages, model_short) = {
        let sessions = store.lock().await;
        let session = sessions
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Session '{id}' not found")))?;

        if session.status == SessionStatus::Busy {
            return Err(ApiError::Conflict(
                "Session is busy, cannot compact".to_string(),
            ));
        }
        if session.messages.len() < 2 {
            return Err(ApiError::BadRequest(
                "Not enough messages to compact".to_string(),
            ));
        }

        (session.messages.clone(), session.model.clone())
    };

    // Build a summarization request from the full conversation
    let model_resolved = anthropic::model::resolve_model(&model_short).to_string();
    let mut summary_messages = messages.clone();
    summary_messages.push(Message {
        role: "user".to_string(),
        content: MessageContent::Text(COMPACT_PROMPT.to_string()),
    });

    let request = MessagesRequest {
        model: model_resolved,
        max_tokens: 4096,
        messages: summary_messages,
        system: None,
        stream: false,
        metadata: None,
        tools: None,
        tool_choice: None,
        thinking: None,
        temperature: Some(0.5),
    };

    let (assistant_msg, usage) = anthropic::client::call_messages(&creds, &request)
        .await
        .map_err(|e| ApiError::Internal(e.context("Compact API call failed")))?;

    // Extract summary text from assistant response
    let summary = match &assistant_msg.content {
        MessageContent::Text(t) => t.clone(),
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
    };

    // Replace session messages with compacted pair
    let compacted_messages = vec![
        Message {
            role: "user".to_string(),
            content: MessageContent::Text("[conversation compacted]".to_string()),
        },
        Message {
            role: "assistant".to_string(),
            content: MessageContent::Text(summary.clone()),
        },
    ];

    // Estimate new token counts from the compacted messages
    let new_input = (summary.len() as u64 + 30) / 4;
    let new_output = usage.output_tokens;

    // Update in-memory session
    {
        let mut sessions = store.lock().await;
        if let Some(session) = sessions.get_mut(&id) {
            session.messages = compacted_messages.clone();
            session.total_input_tokens = new_input;
            session.total_output_tokens = new_output;
        }
    }

    // Persist
    save_messages(&id, &compacted_messages);
    update_meta_tokens(&id, new_input, new_output);

    // Broadcast compact event to frontend via SSE
    {
        let sessions = store.lock().await;
        if let Some(session) = sessions.get(&id) {
            let compact_event = serde_json::json!({
                "type": "compact.completed",
                "summary": summary,
                "total_input": new_input,
                "total_output": new_output,
            });
            let _ = session.tx.send(compact_event.to_string());
        }
    }

    // Persist a compact event to events.ndjson for replay
    let compact_replay = serde_json::json!({
        "type": "compact.completed",
        "summary": summary,
        "total_input": new_input,
        "total_output": new_output,
    });
    persistence::append_event(&id, &compact_replay.to_string());

    Ok(Json(serde_json::json!({
        "ok": true,
        "summary": summary,
        "usage": {
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
        }
    })))
}
