//! Chat engine — shared logic for both web UI and TUI.
//!
//! This module holds all Axum-independent chat functionality:
//! session management, the agentic loop, persistence, context
//! truncation, and system prompt construction.

pub mod agentic;
pub mod compressor;
pub mod context;
pub mod persistence;
pub mod project_context;
pub mod session;
pub mod session_mgmt;
pub mod spawner;
pub mod system_prompt;
pub mod tool_executor;
pub mod tool_tier;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::webui::anthropic;
use crate::webui::auth::credentials;
use crate::webui::provider;

use session::{SessionStatus, SessionStore};

/// Options for creating a new chat session.
pub struct CreateSessionOpts {
    pub cwd: PathBuf,
    pub model: String,
    pub system_prompt: Option<String>,
    pub agent: Option<String>,
    pub max_turns: Option<usize>,
}

/// The chat engine — creates and manages sessions without HTTP.
pub struct ChatEngine {
    pub store: SessionStore,
}

impl Default for ChatEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatEngine {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new chat session and return its ID.
    pub async fn create_session(&self, opts: CreateSessionOpts) -> anyhow::Result<String> {
        session_mgmt::create_session(&self.store, opts).await
    }

    /// Restore a session from disk persistence.
    pub async fn restore_session(&self, id: &str) -> Option<()> {
        session_mgmt::restore_session(&self.store, id).await
    }

    /// Send a message to an existing session (non-HTTP path).
    pub async fn send_message(&self, session_id: &str, text: &str) -> anyhow::Result<()> {
        let creds = credentials::resolve_credentials()?
            .ok_or_else(|| anyhow::anyhow!("No credentials configured. Run the web UI first to set up authentication, or place credentials in ~/.config/hive/credentials.json"))?;

        let mut sessions = self.store.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session '{session_id}' not found"))?;

        if session.status == SessionStatus::Busy {
            anyhow::bail!("Session is busy");
        }

        let resolved_text = system_prompt::resolve_slash_command(text, &session.cwd);

        // Set title from first user message
        if session.title.is_none() {
            let title = persistence::extract_title(text);
            session.title = Some(title.clone());
            if let Some(mut meta) = persistence::read_meta(session_id) {
                meta.title = title;
                meta.updated_at = chrono::Utc::now().to_rfc3339();
                meta.status = "busy".to_string();
                persistence::write_meta(&meta);
            }
        } else {
            persistence::update_meta_status(session_id, "busy");
        }

        session.status = SessionStatus::Busy;
        session
            .abort_flag
            .store(false, std::sync::atomic::Ordering::Relaxed);

        let user_message = anthropic::types::Message {
            role: "user".to_string(),
            content: anthropic::types::MessageContent::Text(resolved_text),
        };
        session.messages.push(user_message);

        let sys_prompt = session.system_prompt.clone().or_else(|| {
            Some(system_prompt::build_mode_system_prompt(
                session.chat_mode,
                &session.cwd,
            ))
        });

        let model_resolved = provider::resolve_model(&session.model, &creds);
        let mut session_tools = session.tools.clone();

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

        let params = spawner::AgenticTaskParams {
            creds,
            model_resolved,
            messages_snapshot: session.messages.clone(),
            system_prompt: sys_prompt,
            tools_opt,
            session_cwd: session.cwd.clone(),
            tx: session.tx.clone(),
            abort_flag: session.abort_flag.clone(),
            session_id: session_id.to_string(),
            store_bg: self.store.clone(),
            effort: session.effort,
            chat_mode: session.chat_mode,
            max_turns: session.max_turns,
            mcp_pool: session.mcp_pool.clone(),
            deferred_tools_active: session.deferred_tools_active,
        };

        drop(sessions);

        // Persist user event
        let user_event = serde_json::json!({
            "type": "user",
            "message": {
                "content": [{"type": "text", "text": text}]
            }
        });
        persistence::append_event(session_id, &user_event.to_string());

        spawner::spawn_agentic_task(params);

        Ok(())
    }

    /// Find the most recent session ID from persisted sessions.
    pub fn find_last_session_id(&self) -> Option<String> {
        let mut sessions = persistence::list_persisted_sessions();
        sessions.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
        sessions.first().map(|(id, _)| id.clone())
    }

    /// Check if credentials are available.
    pub fn has_credentials() -> bool {
        credentials::resolve_credentials().ok().flatten().is_some()
    }

    /// Abort the current streaming response.
    pub async fn abort_session(&self, session_id: &str) {
        let mut sessions = self.store.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session
                .abort_flag
                .store(true, std::sync::atomic::Ordering::Relaxed);
            session.status = SessionStatus::Idle;
        }
    }
}
