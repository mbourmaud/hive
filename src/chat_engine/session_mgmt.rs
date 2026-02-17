//! Session creation, restoration, and lifecycle management.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::broadcast;

use crate::webui::mcp_client::pool::McpPool;
use crate::webui::tools;

use super::persistence;
use super::session::{ChatMode, ChatSession, Effort, SessionStatus, SessionStore};
use super::CreateSessionOpts;

/// Create a new chat session and insert it into the store.
pub async fn create_session(
    store: &SessionStore,
    opts: CreateSessionOpts,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let (tx, _rx) = broadcast::channel::<String>(512);
    let now = chrono::Utc::now();

    let agent_profile = opts.agent.as_ref().and_then(|agent_slug| {
        crate::webui::chat::agents::discover_agents(&opts.cwd)
            .into_iter()
            .find(|a| a.slug == *agent_slug)
    });

    let model = agent_profile
        .as_ref()
        .and_then(|a| a.model.clone())
        .unwrap_or(opts.model);

    let sys_prompt = if let Some(ref profile) = agent_profile {
        if profile.system_prompt.is_empty() {
            opts.system_prompt
        } else {
            Some(profile.system_prompt.clone())
        }
    } else {
        opts.system_prompt
    };

    let meta = persistence::SessionMeta {
        id: id.clone(),
        cwd: opts.cwd.to_string_lossy().to_string(),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        status: "idle".to_string(),
        title: "New session".to_string(),
        model: model.clone(),
        system_prompt: sys_prompt.clone(),
        total_input_tokens: 0,
        total_output_tokens: 0,
    };
    persistence::write_meta(&meta);

    let builtin_tools = tools::definitions::builtin_tool_definitions();
    let allowed_tools = agent_profile
        .as_ref()
        .filter(|a| !a.allowed_tools.is_empty())
        .map(|a| a.allowed_tools.clone());

    let mcp_pool = Arc::new(tokio::sync::Mutex::new(McpPool::new(opts.cwd.clone())));

    let session = ChatSession {
        id: id.clone(),
        cwd: opts.cwd.clone(),
        created_at: now,
        status: SessionStatus::Idle,
        tx,
        title: None,
        messages: Vec::new(),
        model,
        system_prompt: sys_prompt,
        abort_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        tools: builtin_tools,
        effort: Effort::Medium,
        chat_mode: ChatMode::Code,
        total_input_tokens: 0,
        total_output_tokens: 0,
        allowed_tools,
        disallowed_tools: None,
        max_turns: opts.max_turns,
        mcp_pool: Some(mcp_pool),
        agent: opts.agent,
    };

    store.lock().await.insert(id.clone(), session);

    // Discover MCP tools in background
    let bg_store = store.clone();
    let bg_id = id.clone();
    let bg_cwd = opts.cwd;
    tokio::spawn(async move {
        let mcp_tools = crate::webui::mcp_client::discover_tools_for_cwd(&bg_cwd).await;
        if !mcp_tools.is_empty() {
            let mut sessions = bg_store.lock().await;
            if let Some(s) = sessions.get_mut(&bg_id) {
                s.tools.extend(mcp_tools);
            }
        }
    });

    Ok(id)
}

/// Restore a session from disk persistence into the store.
pub async fn restore_session(store: &SessionStore, id: &str) -> Option<()> {
    let meta = persistence::read_meta(id)?;
    let messages = persistence::load_messages(id);

    let cwd = PathBuf::from(&meta.cwd);
    let (tx, _rx) = broadcast::channel::<String>(512);

    let created_at = chrono::DateTime::parse_from_rfc3339(&meta.created_at)
        .ok()?
        .with_timezone(&chrono::Utc);

    let builtin_tools = tools::definitions::builtin_tool_definitions();
    let mcp_pool = Arc::new(tokio::sync::Mutex::new(McpPool::new(cwd.clone())));

    let session = ChatSession {
        id: id.to_string(),
        cwd: cwd.clone(),
        created_at,
        status: SessionStatus::Idle,
        tx,
        title: Some(meta.title),
        messages,
        model: meta.model,
        system_prompt: meta.system_prompt,
        abort_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        tools: builtin_tools,
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

    let id_owned = id.to_string();
    store.lock().await.insert(id_owned.clone(), session);

    // Discover MCP tools in background
    let bg_store = store.clone();
    tokio::spawn(async move {
        let mcp_tools = crate::webui::mcp_client::discover_tools_for_cwd(&cwd).await;
        if !mcp_tools.is_empty() {
            let mut sessions = bg_store.lock().await;
            if let Some(s) = sessions.get_mut(&id_owned) {
                s.tools.extend(mcp_tools);
            }
        }
    });

    Some(())
}
