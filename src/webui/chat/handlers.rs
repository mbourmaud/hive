use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::webui::anthropic::{
    self,
    types::{ContentBlock, Message, MessageContent, MessagesRequest},
};
use crate::webui::auth::credentials;
use crate::webui::error::{ApiError, ApiResult};
use crate::webui::extractors::ValidJson;
use crate::webui::tools;

use super::dto::{
    CreateSessionRequest, SendMessageRequest, SessionListItem, SessionResponse,
    UpdateSessionRequest,
};
use super::persistence::{
    append_event, extract_title, list_persisted_sessions, load_messages, read_meta, save_messages,
    session_dir, update_meta_status, write_meta, SessionMeta,
};
use super::session::{ChatSession, SessionStatus, SessionStore};

/// POST /api/chat/sessions
pub async fn create_session(
    State(store): State<SessionStore>,
    ValidJson(body): ValidJson<CreateSessionRequest>,
) -> ApiResult<impl IntoResponse> {
    let cwd = std::path::PathBuf::from(&body.cwd);
    if !cwd.is_dir() {
        return Err(ApiError::BadRequest(
            "cwd is not a valid directory".to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let (tx, _rx) = broadcast::channel::<String>(512);
    let now = chrono::Utc::now();

    let meta = SessionMeta {
        id: id.clone(),
        cwd: cwd.to_string_lossy().to_string(),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        status: "idle".to_string(),
        title: "New session".to_string(),
        model: body.model.clone(),
        system_prompt: body.system_prompt.clone(),
    };
    write_meta(&meta);

    // Populate built-in tools
    let builtin_tools = tools::definitions::builtin_tool_definitions();

    // Load MCP tools for this session's cwd
    let mcp_tools = crate::webui::mcp_client::discover_tools_for_cwd(&cwd).await;

    let mut all_tools = builtin_tools;
    all_tools.extend(mcp_tools);

    let session = ChatSession {
        id: id.clone(),
        cwd: cwd.clone(),
        created_at: now,
        status: SessionStatus::Idle,
        tx,
        title: None,
        messages: Vec::new(),
        model: body.model.clone(),
        system_prompt: body.system_prompt,
        abort_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        tools: all_tools,
    };

    store.lock().await.insert(id.clone(), session);

    let resp = SessionResponse {
        id,
        status: SessionStatus::Idle,
        cwd: cwd.to_string_lossy().to_string(),
        created_at: now.to_rfc3339(),
        model: body.model,
    };

    Ok((StatusCode::CREATED, Json(serde_json::json!(resp))))
}

/// GET /api/chat/sessions/{id}/stream
pub async fn stream_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
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

/// Resolve slash commands: if user message starts with `/commandname`,
/// look up the command file and expand it.
fn resolve_slash_command(text: &str, cwd: &std::path::Path) -> String {
    if !text.starts_with('/') {
        return text.to_string();
    }

    let parts: Vec<&str> = text.splitn(2, char::is_whitespace).collect();
    let command_name = parts[0].trim_start_matches('/');
    let arguments = parts.get(1).unwrap_or(&"").to_string();

    if command_name.is_empty() {
        return text.to_string();
    }

    // Search for command file in standard locations
    let search_dirs = [
        cwd.join(".claude").join("commands"),
        dirs::home_dir()
            .unwrap_or_default()
            .join(".claude")
            .join("commands"),
    ];

    for dir in &search_dirs {
        let md_path = dir.join(format!("{command_name}.md"));
        if md_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&md_path) {
                let expanded = content.replace("$ARGUMENTS", &arguments);
                return expanded;
            }
        }
    }

    // No command found — return original text
    text.to_string()
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

    let system_prompt = session.system_prompt.clone().or_else(|| {
        Some(format!(
            "You are Claude, a helpful AI assistant. The user's working directory is: {}",
            session.cwd.display()
        ))
    });

    let model_resolved = anthropic::model::resolve_model(&session.model);
    let session_tools = if session.tools.is_empty() {
        None
    } else {
        Some(session.tools.clone())
    };
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

    // Spawn background task to run the agentic loop
    tokio::spawn(async move {
        let mut rx = tx.subscribe();
        let persist_id = session_id.clone();
        let persist_handle = tokio::spawn(async move {
            while let Ok(line) = rx.recv().await {
                append_event(&persist_id, &line);
            }
        });

        let loop_result = run_agentic_loop(AgenticLoopParams {
            creds: &creds,
            model: model_resolved,
            messages: messages_snapshot,
            system_prompt,
            tools: session_tools,
            cwd: &session_cwd,
            tx: &tx,
            session_id: &session_id,
            abort_flag: &abort_flag,
            store: store_bg.clone(),
        })
        .await;

        let completion = serde_json::json!({"type": "session.completed"}).to_string();
        let _ = tx.send(completion);

        let mut sessions = store_bg.lock().await;
        if let Some(s) = sessions.get_mut(&session_id) {
            match loop_result {
                Ok(final_messages) => {
                    s.messages = final_messages;
                    save_messages(&session_id, &s.messages);
                }
                Err(e) => {
                    eprintln!("Agentic loop error: {e:#}");
                }
            }
            s.status = SessionStatus::Idle;
        }
        drop(sessions);

        update_meta_status(&session_id, "idle");
        persist_handle.abort();
    });

    Ok(Json(serde_json::json!({"ok": true})))
}

/// Parameters for the agentic loop, grouped to avoid too-many-arguments.
struct AgenticLoopParams<'a> {
    creds: &'a credentials::Credentials,
    model: &'a str,
    messages: Vec<Message>,
    system_prompt: Option<String>,
    tools: Option<Vec<anthropic::types::ToolDefinition>>,
    cwd: &'a std::path::Path,
    tx: &'a broadcast::Sender<String>,
    session_id: &'a str,
    abort_flag: &'a Arc<std::sync::atomic::AtomicBool>,
    store: SessionStore,
}

/// The agentic loop: stream API response, execute tools, repeat until end_turn.
async fn run_agentic_loop(params: AgenticLoopParams<'_>) -> anyhow::Result<Vec<Message>> {
    let AgenticLoopParams {
        creds,
        model,
        mut messages,
        system_prompt,
        tools: session_tools,
        cwd,
        tx,
        session_id,
        abort_flag,
        store,
    } = params;
    const MAX_TOOL_TURNS: usize = 25;

    for _turn in 0..MAX_TOOL_TURNS {
        if abort_flag.load(Ordering::Relaxed) {
            break;
        }

        let request = MessagesRequest {
            model: model.to_string(),
            max_tokens: 16384,
            messages: messages.clone(),
            system: system_prompt.clone(),
            stream: true,
            metadata: None,
            tools: session_tools.clone(),
            tool_choice: None,
        };

        let (assistant_msg, _usage, stop_reason) =
            anthropic::client::stream_messages(creds, &request, tx, session_id, abort_flag).await?;

        messages.push(assistant_msg.clone());

        // Update messages in the store after each assistant response
        {
            let mut sessions = store.lock().await;
            if let Some(s) = sessions.get_mut(session_id) {
                s.messages = messages.clone();
            }
        }

        if stop_reason != "tool_use" {
            break;
        }

        if abort_flag.load(Ordering::Relaxed) {
            break;
        }

        // Extract tool_use blocks from the assistant message
        let tool_uses = extract_tool_uses(&assistant_msg);
        if tool_uses.is_empty() {
            break;
        }

        // Execute each tool and collect results
        let mut tool_result_blocks: Vec<ContentBlock> = Vec::new();

        for (tool_id, tool_name, tool_input) in &tool_uses {
            if abort_flag.load(Ordering::Relaxed) {
                break;
            }

            // Check if this is an MCP tool (contains __ separator)
            let result = if tool_name.contains("__") {
                // MCP tool — route through MCP client
                match crate::webui::mcp_client::call_mcp_tool(tool_name, tool_input, cwd).await {
                    Ok(content) => tools::ToolExecutionResult {
                        content,
                        is_error: false,
                    },
                    Err(e) => tools::ToolExecutionResult {
                        content: format!("{e:#}"),
                        is_error: true,
                    },
                }
            } else {
                // Built-in tool
                match tools::execute_tool(tool_name, tool_input, cwd).await {
                    Some(r) => r,
                    None => tools::ToolExecutionResult {
                        content: format!("Unknown tool: {tool_name}"),
                        is_error: true,
                    },
                }
            };

            // Broadcast tool result to frontend
            let tool_result_event = serde_json::json!({
                "type": "user",
                "message": {
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_id,
                        "content": result.content,
                        "is_error": result.is_error
                    }]
                }
            });
            let _ = tx.send(tool_result_event.to_string());

            tool_result_blocks.push(ContentBlock::ToolResult {
                tool_use_id: tool_id.clone(),
                content: result.content,
                is_error: Some(result.is_error),
            });
        }

        // Add tool results as a user message for the next API call
        let tool_result_message = Message {
            role: "user".to_string(),
            content: MessageContent::Blocks(tool_result_blocks),
        };
        messages.push(tool_result_message);
    }

    Ok(messages)
}

/// Extract (id, name, input) tuples from tool_use blocks in an assistant message.
fn extract_tool_uses(msg: &Message) -> Vec<(String, String, serde_json::Value)> {
    match &msg.content {
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.clone(), name.clone(), input.clone()))
                }
                _ => None,
            })
            .collect(),
        MessageContent::Text(_) => Vec::new(),
    }
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

/// DELETE /api/chat/sessions/{id}
pub async fn delete_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut sessions = store.lock().await;
    let dir = session_dir(&id);

    if let Some(session) = sessions.remove(&id) {
        session.abort_flag.store(true, Ordering::Relaxed);
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Ok(Json(serde_json::json!({"ok": true})));
    }
    drop(sessions);

    if dir.exists() {
        let _ = tokio::fs::remove_dir_all(&dir).await;
        Ok(Json(serde_json::json!({"ok": true})))
    } else {
        Err(ApiError::NotFound(format!("Session '{id}' not found")))
    }
}

/// GET /api/chat/sessions
pub async fn list_sessions(
    State(store): State<SessionStore>,
) -> ApiResult<Json<Vec<SessionListItem>>> {
    let sessions = store.lock().await;
    let mut items: Vec<SessionListItem> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for s in sessions.values() {
        let title = s.title.clone().unwrap_or_else(|| "New session".to_string());
        let status = match &s.status {
            SessionStatus::Idle => "idle",
            SessionStatus::Busy => "busy",
            SessionStatus::Error(_) => "error",
        };
        items.push(SessionListItem {
            id: s.id.clone(),
            status: status.to_string(),
            cwd: s.cwd.to_string_lossy().to_string(),
            created_at: s.created_at.to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            title,
            model: s.model.clone(),
        });
        seen_ids.insert(s.id.clone());
    }

    for (id, meta) in list_persisted_sessions() {
        if seen_ids.contains(&id) {
            continue;
        }
        items.push(SessionListItem {
            id: meta.id,
            status: meta.status,
            cwd: meta.cwd,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            title: meta.title,
            model: meta.model,
        });
    }

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(Json(items))
}

/// GET /api/chat/sessions/{id}/history
pub async fn session_history(Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    let events_path = session_dir(&id).join("events.ndjson");
    let events: Vec<serde_json::Value> = match tokio::fs::read_to_string(&events_path).await {
        Ok(contents) => contents
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect(),
        Err(_) => Vec::new(),
    };

    let messages = load_messages(&id);

    if events.is_empty() && messages.is_empty() {
        return Err(ApiError::NotFound(format!(
            "No history found for session '{id}'"
        )));
    }

    Ok(Json(serde_json::json!({
        "events": events,
        "messages": messages
    })))
}

/// PATCH /api/chat/sessions/{id}
pub async fn update_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
    ValidJson(body): ValidJson<UpdateSessionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut sessions = store.lock().await;
    if let Some(session) = sessions.get_mut(&id) {
        if let Some(ref title) = body.title {
            session.title = Some(title.clone());
        }
        if let Some(ref system_prompt) = body.system_prompt {
            session.system_prompt = Some(system_prompt.clone());
        }
    }
    drop(sessions);

    let mut meta =
        read_meta(&id).ok_or_else(|| ApiError::NotFound(format!("Session '{id}' not found")))?;

    if let Some(ref title) = body.title {
        meta.title = title.clone();
    }
    if let Some(ref system_prompt) = body.system_prompt {
        meta.system_prompt = Some(system_prompt.clone());
    }
    meta.updated_at = chrono::Utc::now().to_rfc3339();
    write_meta(&meta);

    Ok(Json(serde_json::json!({"ok": true})))
}
