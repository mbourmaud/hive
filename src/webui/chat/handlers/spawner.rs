use std::sync::Arc;
use tokio::sync::broadcast;

use crate::webui::anthropic;
use crate::webui::auth::credentials;
use crate::webui::mcp_client::pool::McpPool;

use super::super::persistence::{append_event, save_messages, update_meta_status};
use super::super::session::{Effort, SessionStatus, SessionStore};
use super::agentic::{run_agentic_loop, AgenticLoopParams};

use anthropic::types::Message;

pub(super) struct AgenticTaskParams {
    pub creds: credentials::Credentials,
    pub model_resolved: String,
    pub messages_snapshot: Vec<Message>,
    pub system_prompt: Option<String>,
    pub tools_opt: Option<Vec<anthropic::types::ToolDefinition>>,
    pub session_cwd: std::path::PathBuf,
    pub tx: broadcast::Sender<String>,
    pub abort_flag: Arc<std::sync::atomic::AtomicBool>,
    pub session_id: String,
    pub store_bg: SessionStore,
    pub effort: Effort,
    pub max_turns: Option<usize>,
    pub mcp_pool: Option<Arc<tokio::sync::Mutex<McpPool>>>,
}

pub(super) fn spawn_agentic_task(params: AgenticTaskParams) {
    let AgenticTaskParams {
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
    } = params;

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
}
