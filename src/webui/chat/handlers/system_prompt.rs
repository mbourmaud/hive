use axum::{extract::Query, Json};

use super::super::agents;

// Re-export shared logic from chat_engine
pub(super) use crate::chat_engine::system_prompt::{
    build_mode_system_prompt, resolve_slash_command,
};

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
