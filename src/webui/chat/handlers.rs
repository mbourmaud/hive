use axum::{
    extract::{Path, Query, State},
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
    types::{ContentBlock, Message, MessageContent, MessagesRequest, ThinkingConfig},
};
use crate::webui::auth::credentials;
use crate::webui::error::{ApiError, ApiResult};
use crate::webui::extractors::ValidJson;
use crate::webui::mcp_client::pool::McpPool;
use crate::webui::tools;

use super::agents;
use super::context;
use super::dto::{
    CreateSessionRequest, SendMessageRequest, SessionListItem, SessionResponse,
    UpdateSessionRequest,
};
use super::persistence::{
    append_event, extract_title, list_persisted_sessions, load_messages, read_meta, save_messages,
    session_dir, update_meta_status, write_meta, SessionMeta,
};
use super::session::{ChatSession, Effort, SessionStatus, SessionStore};

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

/// Build a system prompt that instructs Claude to use the available tools.
fn build_default_system_prompt(cwd: &std::path::Path) -> String {
    let is_git = cwd.join(".git").is_dir();
    let platform = std::env::consts::OS;
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Load CLAUDE.md or opencode.md if present
    let mut context_files = String::new();
    for name in &["CLAUDE.md", "CLAUDE.local.md", "opencode.md", "OpenCode.md"] {
        let path = cwd.join(name);
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                context_files.push_str(&format!(
                    "\n<context_file path=\"{name}\">\n{content}\n</context_file>\n"
                ));
            }
        }
    }

    format!(
        r#"You are Hive, an interactive AI coding assistant with access to tools for reading, writing, and searching code.

You help users with software engineering tasks including reading files, writing code, debugging, searching codebases, and executing commands.

# Tools

You have access to these tools — use them to accomplish tasks:

- **Read**: Read files from the filesystem. Always read a file before modifying it.
- **Write**: Create or overwrite files.
- **Edit**: Make precise string replacements in files. Preferred over Write for modifying existing files.
- **Bash**: Execute shell commands. Use for git, build tools, tests, and other CLI operations.
- **Grep**: Search file contents using regex patterns (powered by ripgrep).
- **Glob**: Find files by name patterns.

# Guidelines

- When asked about files or code, use the Read tool to examine them — never guess at file contents.
- When asked to modify code, read the file first, then use Edit for precise changes.
- For searching, use Grep for content search and Glob for finding files by name.
- Use Bash for running tests, git operations, build commands, and other shell tasks.
- Be concise. Minimize output tokens. Answer with fewer than 4 lines unless the user asks for detail.
- Follow existing code conventions and patterns in the project.
- When multiple independent tool calls are needed, make them all at once for efficiency.

<env>
Working directory: {cwd}
Is git repo: {is_git}
Platform: {platform}
Date: {date}
</env>{context_files}"#,
        cwd = cwd.display(),
        is_git = is_git,
        platform = platform,
        date = date,
        context_files = context_files,
    )
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

/// GET /api/chat/agents?cwd=...
pub async fn list_agents(Query(params): Query<AgentsQuery>) -> Json<Vec<agents::AgentProfile>> {
    let cwd = params
        .cwd
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let profiles = agents::discover_agents(&cwd);
    Json(profiles)
}

#[derive(Debug, serde::Deserialize)]
pub struct AgentsQuery {
    cwd: Option<String>,
}

/// Restore a persisted session into the in-memory store.
async fn restore_session_from_disk(store: &SessionStore, id: &str) -> Option<()> {
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

    let session = super::session::ChatSession {
        id: id.to_string(),
        cwd,
        created_at,
        status: super::session::SessionStatus::Idle,
        tx,
        title: Some(meta.title),
        messages,
        model: meta.model,
        system_prompt: meta.system_prompt,
        abort_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        tools: all_tools,
        effort: Effort::Medium,
        total_input_tokens: 0,
        total_output_tokens: 0,
        allowed_tools: None,
        disallowed_tools: None,
        max_turns: None,
        mcp_pool: Some(mcp_pool),
        agent: None,
    };

    store.lock().await.insert(id.to_string(), session);
    Some(())
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

    // Apply tool permission filters (Feature 5)
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
            model: &model_resolved,
            messages: messages_snapshot,
            system_prompt,
            tools: tools_opt,
            cwd: &session_cwd,
            tx: &tx,
            session_id: &session_id,
            abort_flag: &abort_flag,
            store: store_bg.clone(),
            effort,
            max_turns,
            mcp_pool,
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
    effort: Effort,
    max_turns: Option<usize>,
    mcp_pool: Option<Arc<tokio::sync::Mutex<McpPool>>>,
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
        effort,
        max_turns,
        mcp_pool,
    } = params;
    let max_tool_turns = max_turns.unwrap_or(25);

    // Build thinking config from effort level
    let thinking = if effort.thinking_enabled() {
        Some(ThinkingConfig {
            thinking_type: "enabled".to_string(),
            budget_tokens: effort.thinking_budget(),
        })
    } else {
        None
    };

    // When thinking is enabled, max_tokens must be > budget
    let base_max_tokens: u32 = if effort.thinking_enabled() {
        effort.thinking_budget() + 16384
    } else {
        16384
    };

    for _turn in 0..max_tool_turns {
        if abort_flag.load(Ordering::Relaxed) {
            break;
        }

        // Context window management: truncate if needed
        let estimated = context::estimate_total_tokens(&messages);
        let api_messages = if effort.thinking_enabled() {
            // Keep thinking blocks in history when thinking is enabled
            // (API requires signature for echoed thinking blocks)
            context::truncate_messages(&messages, estimated)
        } else {
            // Strip thinking blocks when thinking is disabled
            let stripped = strip_thinking_from_history(&messages);
            let stripped_estimated = context::estimate_total_tokens(&stripped);
            context::truncate_messages(&stripped, stripped_estimated)
        };

        let request = MessagesRequest {
            model: model.to_string(),
            max_tokens: base_max_tokens,
            messages: api_messages,
            system: system_prompt.clone(),
            stream: true,
            metadata: None,
            tools: session_tools.clone(),
            tool_choice: None,
            thinking: thinking.clone(),
            temperature: if effort.thinking_enabled() {
                None
            } else {
                Some(1.0)
            },
        };

        let (assistant_msg, usage, stop_reason) =
            anthropic::client::stream_messages(creds, &request, tx, session_id, abort_flag).await?;

        messages.push(assistant_msg.clone());

        // Update messages and token counters in the store
        {
            let mut sessions = store.lock().await;
            if let Some(s) = sessions.get_mut(session_id) {
                s.messages = messages.clone();
                s.total_input_tokens += usage.input_tokens;
                s.total_output_tokens += usage.output_tokens;
            }
        }

        // Broadcast cumulative usage event to frontend
        {
            let sessions = store.lock().await;
            if let Some(s) = sessions.get(session_id) {
                let usage_event = serde_json::json!({
                    "type": "usage",
                    "input_tokens": usage.input_tokens,
                    "output_tokens": usage.output_tokens,
                    "total_input": s.total_input_tokens,
                    "total_output": s.total_output_tokens,
                    "cache_creation_input_tokens": usage.cache_creation_input_tokens,
                    "cache_read_input_tokens": usage.cache_read_input_tokens
                });
                let _ = tx.send(usage_event.to_string());
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
                // MCP tool — use connection pool if available, otherwise fall back
                let mcp_result = if let Some(ref pool) = mcp_pool {
                    let mut pool = pool.lock().await;
                    pool.call_tool(tool_name, tool_input).await
                } else {
                    crate::webui::mcp_client::call_mcp_tool(tool_name, tool_input, cwd).await
                };
                match mcp_result {
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

/// Remove thinking blocks from conversation history before sending to the API.
/// The Anthropic API requires valid signatures on thinking blocks when echoed
/// back. Like OpenCode, we simply drop them — they were already shown to the
/// user during streaming.
fn strip_thinking_from_history(messages: &[Message]) -> Vec<Message> {
    messages
        .iter()
        .map(|msg| match &msg.content {
            MessageContent::Blocks(blocks) => {
                let filtered: Vec<ContentBlock> = blocks
                    .iter()
                    .filter(|b| !matches!(b, ContentBlock::Thinking { .. }))
                    .cloned()
                    .collect();
                if filtered.is_empty() {
                    // Keep at least an empty text block so the message isn't empty
                    Message {
                        role: msg.role.clone(),
                        content: MessageContent::Blocks(vec![ContentBlock::Text {
                            text: String::new(),
                        }]),
                    }
                } else {
                    Message {
                        role: msg.role.clone(),
                        content: MessageContent::Blocks(filtered),
                    }
                }
            }
            _ => msg.clone(),
        })
        .collect()
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
        // Shutdown MCP pool connections
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
