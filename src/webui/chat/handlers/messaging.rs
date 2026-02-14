use axum::{
    extract::{Path, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use std::sync::atomic::Ordering;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::webui::anthropic::{
    self,
    types::{Message, MessageContent},
};
use crate::webui::auth::credentials;
use crate::webui::error::{ApiError, ApiResult};
use crate::webui::extractors::ValidJson;

use super::super::dto::SendMessageRequest;
use super::super::persistence::{
    append_event, extract_title, read_meta, update_meta_status, write_meta,
};
use super::super::session::{Effort, SessionStatus, SessionStore};
use super::sessions::restore_session_from_disk;
use super::spawner::{spawn_agentic_task, AgenticTaskParams};
use super::system_prompt::{build_default_system_prompt, resolve_slash_command};

/// GET /api/chat/sessions/{id}/stream
pub async fn stream_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Try to restore from disk if not in memory
    {
        let sessions = store.lock().await;
        if !sessions.contains_key(&id) {
            drop(sessions);
            let _ = restore_session_from_disk(&store, &id).await;
        }
    }

    let sessions = store.lock().await;
    let session = match sessions.get(&id) {
        Some(s) => s,
        None => {
            let stream = async_stream::stream! {
                yield Ok::<_, std::convert::Infallible>(
                    Event::default().event("error").data("session not found")
                );
            };
            return Sse::new(stream)
                .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(30)))
                .into_response();
        }
    };

    let rx = session.tx.subscribe();
    drop(sessions);

    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(data) => Some(Ok::<_, std::convert::Infallible>(
            Event::default().data(data),
        )),
        Err(_) => None,
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(30)))
        .into_response()
}

/// POST /api/chat/sessions/{id}/message
pub async fn send_message(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
    ValidJson(body): ValidJson<SendMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    let creds = match credentials::load_credentials() {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Err(ApiError::Unauthorized(
                "No credentials configured. Please set up an API key or OAuth connection."
                    .to_string(),
            ));
        }
        Err(e) => {
            return Err(ApiError::Internal(e.context("Failed to load credentials")));
        }
    };

    // Try to restore from disk if not in memory
    {
        let sessions = store.lock().await;
        if !sessions.contains_key(&id) {
            drop(sessions);
            if restore_session_from_disk(&store, &id).await.is_none() {
                return Err(ApiError::NotFound(format!("Session '{id}' not found")));
            }
        }
    }

    let mut sessions = store.lock().await;
    let session = sessions
        .get_mut(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Session '{id}' not found")))?;

    if session.status == SessionStatus::Busy {
        return Err(ApiError::Conflict(
            "Session is busy, wait for current turn to complete".to_string(),
        ));
    }

    // Resolve slash commands
    let resolved_text = resolve_slash_command(&body.text, &session.cwd);

    // Set title from first user message
    if session.title.is_none() {
        let title = extract_title(&body.text);
        session.title = Some(title.clone());
        if let Some(mut meta) = read_meta(&id) {
            meta.title = title;
            meta.updated_at = chrono::Utc::now().to_rfc3339();
            meta.status = "busy".to_string();
            write_meta(&meta);
        }
    } else {
        update_meta_status(&id, "busy");
    }

    // Update model if provided in request
    if let Some(ref model) = body.model {
        session.model = model.clone();
    }

    // Update effort level if provided
    if let Some(ref effort_str) = body.effort {
        if let Some(effort) = Effort::from_str_opt(effort_str) {
            session.effort = effort;
        }
    }

    session.status = SessionStatus::Busy;
    session.abort_flag.store(false, Ordering::Relaxed);

    // Add user message to history (with optional images)
    let user_content = if body.images.is_empty() {
        MessageContent::Text(resolved_text.clone())
    } else {
        let mut blocks: Vec<anthropic::types::ContentBlock> = body
            .images
            .iter()
            .map(|img| anthropic::types::ContentBlock::Image {
                source: anthropic::types::ImageSource {
                    source_type: "base64".to_string(),
                    media_type: img.media_type.clone(),
                    data: img.data.clone(),
                },
            })
            .collect();
        blocks.push(anthropic::types::ContentBlock::Text {
            text: resolved_text.clone(),
        });
        MessageContent::Blocks(blocks)
    };
    let user_message = Message {
        role: "user".to_string(),
        content: user_content,
    };
    session.messages.push(user_message);

    let system_prompt = session
        .system_prompt
        .clone()
        .or_else(|| Some(build_default_system_prompt(&session.cwd)));

    let model_resolved = anthropic::model::resolve_model(&session.model).to_string();
    let mut session_tools: Vec<anthropic::types::ToolDefinition> = session.tools.clone();

    // Apply tool permission filters
    if let Some(ref allowed) = session.allowed_tools {
        session_tools.retain(|t| allowed.iter().any(|a| t.name.contains(a)));
    }
    if let Some(ref disallowed) = session.disallowed_tools {
        session_tools.retain(|t| !disallowed.iter().any(|d| t.name.contains(d)));
    }

    let tools_opt = if session_tools.is_empty() {
        None
    } else {
        Some(session_tools)
    };

    let effort = session.effort;
    let max_turns = session.max_turns;
    let mcp_pool = session.mcp_pool.clone();
    let messages_snapshot = session.messages.clone();
    let session_cwd = session.cwd.clone();
    let tx = session.tx.clone();
    let abort_flag = session.abort_flag.clone();
    let session_id = id.clone();
    let store_bg = store.clone();

    drop(sessions);

    // Persist user message event for replay
    let user_event = serde_json::json!({
        "type": "user",
        "message": {
            "content": [{"type": "text", "text": body.text}]
        }
    });
    append_event(&session_id, &user_event.to_string());

    spawn_agentic_task(AgenticTaskParams {
        creds,
        model_resolved,
        messages_snapshot,
        system_prompt,
        tools_opt,
        session_cwd,
        tx,
        abort_flag,
        session_id,
        store_bg,
        effort,
        max_turns,
        mcp_pool,
    });

    Ok(Json(serde_json::json!({"ok": true})))
}

/// POST /api/chat/sessions/{id}/abort
pub async fn abort_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let sessions = store.lock().await;
    let session = sessions
        .get(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Session '{id}' not found")))?;

    session.abort_flag.store(true, Ordering::Relaxed);

    Ok(Json(serde_json::json!({"ok": true})))
}
