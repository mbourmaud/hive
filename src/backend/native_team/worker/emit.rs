use crate::webui::anthropic::types::{ContentBlock, Message, MessageContent};

use super::super::events::EventEmitter;
use super::WorkerConfig;

/// Emit accumulated cost from the session store to cost.ndjson.
pub async fn emit_cost_from_store(
    store: &crate::webui::chat::session::SessionStore,
    session_id: &str,
    emitter: &EventEmitter,
) {
    let sessions = store.lock().await;
    if let Some(s) = sessions.get(session_id) {
        let usage = crate::webui::anthropic::types::UsageStats {
            input_tokens: s.total_input_tokens,
            output_tokens: s.total_output_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        emitter.emit_cost(&usage);
    }
}

/// Emit ToolDone events for each tool_use in the messages.
pub fn emit_tool_events(emitter: &EventEmitter, messages: &[Message]) {
    for msg in messages {
        if msg.role != "assistant" {
            continue;
        }
        if let MessageContent::Blocks(blocks) = &msg.content {
            for block in blocks {
                if let ContentBlock::ToolUse { id, name, .. } = block {
                    emitter.emit_tool_done(name, Some(id));
                }
            }
        }
    }
}

/// Build a minimal SpawnConfig reference for prompt building.
pub fn spawn_config_ref(config: &WorkerConfig) -> crate::backend::SpawnConfig {
    crate::backend::SpawnConfig {
        drone_name: config.team_name.clone(),
        prd_path: config.prd_path.clone(),
        model: config.model.clone(),
        worktree_path: config.cwd.clone(),
        status_file: std::path::PathBuf::new(),
        working_dir: config.cwd.clone(),
        wait: false,
        team_name: config.team_name.clone(),
        max_agents: 0,
        claude_binary: String::new(),
        environment: None,
        structured_tasks: vec![],
        remote_url: String::new(),
        mode: String::new(),
        project_languages: config.project_languages.clone(),
    }
}
