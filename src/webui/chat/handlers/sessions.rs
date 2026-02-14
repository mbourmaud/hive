use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::webui::error::{ApiError, ApiResult};
use crate::webui::extractors::ValidJson;
use crate::webui::mcp_client::pool::McpPool;
use crate::webui::tools;

use super::super::agents;
use super::super::dto::{
    CreateSessionRequest, SessionListItem, SessionResponse, UpdateSessionRequest,
};
use super::super::persistence::{
    list_persisted_sessions, load_messages, read_meta, session_dir, write_meta, SessionMeta,
};
use super::super::session::{ChatMode, ChatSession, Effort, SessionStatus, SessionStore};

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

    // Load agent profile if specified
    let agent_profile = body.agent.as_ref().and_then(|agent_slug| {
        agents::discover_agents(&cwd)
            .into_iter()
            .find(|a| a.slug == *agent_slug)
    });

    // Determine model and system prompt from agent or request
    let model = agent_profile
        .as_ref()
        .and_then(|a| a.model.clone())
        .unwrap_or(body.model.clone());
    let system_prompt = if let Some(ref profile) = agent_profile {
        if profile.system_prompt.is_empty() {
            body.system_prompt.clone()
        } else {
            Some(profile.system_prompt.clone())
        }
    } else {
        body.system_prompt.clone()
    };

    let meta = SessionMeta {
        id: id.clone(),
        cwd: cwd.to_string_lossy().to_string(),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        status: "idle".to_string(),
        title: "New session".to_string(),
        model: model.clone(),
        system_prompt: system_prompt.clone(),
        total_input_tokens: 0,
        total_output_tokens: 0,
    };
    write_meta(&meta);

    // Populate built-in tools
    let builtin_tools = tools::definitions::builtin_tool_definitions();

    // Load MCP tools for this session's cwd
    let mcp_tools = crate::webui::mcp_client::discover_tools_for_cwd(&cwd).await;

    let mut all_tools = builtin_tools;
    all_tools.extend(mcp_tools);

    // Filter tools based on agent profile allowed_tools
    let allowed_tools = agent_profile
        .as_ref()
        .filter(|a| !a.allowed_tools.is_empty())
        .map(|a| a.allowed_tools.clone());

    // Create MCP connection pool for this session
    let mcp_pool = Arc::new(tokio::sync::Mutex::new(McpPool::new(cwd.clone())));

    let session = ChatSession {
        id: id.clone(),
        cwd: cwd.clone(),
        created_at: now,
        status: SessionStatus::Idle,
        tx,
        title: None,
        messages: Vec::new(),
        model: model.clone(),
        system_prompt,
        abort_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        tools: all_tools,
        effort: Effort::Medium,
        chat_mode: ChatMode::Code,
        total_input_tokens: 0,
        total_output_tokens: 0,
        allowed_tools,
        disallowed_tools: None,
        max_turns: body.max_turns,
        mcp_pool: Some(mcp_pool),
        agent: body.agent.clone(),
    };

    store.lock().await.insert(id.clone(), session);

    let resp = SessionResponse {
        id,
        status: SessionStatus::Idle,
        cwd: cwd.to_string_lossy().to_string(),
        created_at: now.to_rfc3339(),
        model,
    };

    Ok((StatusCode::CREATED, Json(serde_json::json!(resp))))
}

/// Restore a persisted session into the in-memory store.
pub(super) async fn restore_session_from_disk(store: &SessionStore, id: &str) -> Option<()> {
    let meta = read_meta(id)?;
    let messages = load_messages(id);

    let cwd = std::path::PathBuf::from(&meta.cwd);
    let (tx, _rx) = broadcast::channel::<String>(512);

    let created_at = chrono::DateTime::parse_from_rfc3339(&meta.created_at)
        .ok()?
        .with_timezone(&chrono::Utc);

    // Load tools for this session's cwd
    let builtin_tools = tools::definitions::builtin_tool_definitions();
    let mcp_tools = crate::webui::mcp_client::discover_tools_for_cwd(&cwd).await;
    let mut all_tools = builtin_tools;
    all_tools.extend(mcp_tools);

    let mcp_pool = Arc::new(tokio::sync::Mutex::new(McpPool::new(cwd.clone())));

    let session = ChatSession {
        id: id.to_string(),
        cwd,
        created_at,
        status: SessionStatus::Idle,
        tx,
        title: Some(meta.title),
        messages,
        model: meta.model,
        system_prompt: meta.system_prompt,
        abort_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        tools: all_tools,
        effort: Effort::Medium,
        chat_mode: ChatMode::Code,
        total_input_tokens: meta.total_input_tokens,
        total_output_tokens: meta.total_output_tokens,
        allowed_tools: None,
        disallowed_tools: None,
        max_turns: None,
        mcp_pool: Some(mcp_pool),
        agent: None,
    };

    store.lock().await.insert(id.to_string(), session);
    Some(())
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
    let meta = read_meta(&id);

    if events.is_empty() && messages.is_empty() && meta.is_none() {
        return Err(ApiError::NotFound(format!(
            "No history found for session '{id}'"
        )));
    }

    let (total_input, total_output) = meta
        .map(|m| (m.total_input_tokens, m.total_output_tokens))
        .unwrap_or((0, 0));

    Ok(Json(serde_json::json!({
        "events": events,
        "messages": messages,
        "total_input_tokens": total_input,
        "total_output_tokens": total_output
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

/// DELETE /api/chat/sessions/{id}
pub async fn delete_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut sessions = store.lock().await;
    let dir = session_dir(&id);

    if let Some(session) = sessions.remove(&id) {
        session
            .abort_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(pool) = &session.mcp_pool {
            let mut pool = pool.lock().await;
            pool.shutdown_all().await;
        }
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
