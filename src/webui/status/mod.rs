use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;

use crate::webui::auth::credentials;
use crate::webui::chat::SessionStore;
use crate::webui::mcp_client::config::load_mcp_configs;
use crate::webui::monitor::polling::poll_all_projects;
use crate::webui::monitor::MonitorState;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    pub auth: AuthStatusSummary,
    pub session: SessionSummary,
    pub mcp_servers: Vec<McpServerInfo>,
    pub drones: Vec<DroneStatusBrief>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthStatusSummary {
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<String>,
    pub expired: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummary {
    pub active_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpServerInfo {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DroneStatusBrief {
    pub name: String,
    pub liveness: String,
    pub progress: (usize, usize),
    pub elapsed: String,
    pub cost_usd: f64,
    pub is_stuck: bool,
}

// ── Shared state ─────────────────────────────────────────────────────────────

pub struct StatusState {
    pub sessions: SessionStore,
    pub monitor: Arc<MonitorState>,
}

// ── Routes ───────────────────────────────────────────────────────────────────

pub fn routes(sessions: SessionStore, monitor: Arc<MonitorState>) -> Router {
    let state = Arc::new(StatusState { sessions, monitor });
    Router::new()
        .route("/api/status", get(api_status))
        .with_state(state)
}

// ── Handler ──────────────────────────────────────────────────────────────────

async fn api_status(State(state): State<Arc<StatusState>>) -> Json<SystemStatus> {
    let auth = build_auth_summary();
    let session = build_session_summary(&state.sessions).await;
    let mcp_servers = build_mcp_summary();
    let drones = build_drone_summary(&state.monitor);

    Json(SystemStatus {
        auth,
        session,
        mcp_servers,
        drones,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// ── Builders ─────────────────────────────────────────────────────────────────

fn build_auth_summary() -> AuthStatusSummary {
    match credentials::load_credentials() {
        Ok(Some(creds)) => match &creds {
            credentials::Credentials::ApiKey { .. } => AuthStatusSummary {
                configured: true,
                auth_type: Some("api_key".to_string()),
                expired: false,
            },
            credentials::Credentials::OAuth { expires_at, .. } => AuthStatusSummary {
                configured: true,
                auth_type: Some("oauth".to_string()),
                expired: credentials::is_token_expired(*expires_at),
            },
        },
        _ => AuthStatusSummary {
            configured: false,
            auth_type: None,
            expired: false,
        },
    }
}

async fn build_session_summary(sessions: &SessionStore) -> SessionSummary {
    let store = sessions.lock().await;
    let total_count = store.len();
    let active_count = store
        .values()
        .filter(|s| matches!(s.status, crate::webui::chat::session::SessionStatus::Busy))
        .count();
    SessionSummary {
        active_count,
        total_count,
    }
}

fn build_mcp_summary() -> Vec<McpServerInfo> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let configs = load_mcp_configs(&cwd);
    configs
        .into_iter()
        .map(|(name, cfg)| McpServerInfo {
            name,
            command: cfg.command,
            args: cfg.args,
        })
        .collect()
}

fn build_drone_summary(monitor: &MonitorState) -> Vec<DroneStatusBrief> {
    let projects = poll_all_projects(&monitor.snapshot_stores);
    let mut drones = Vec::new();

    for project in projects {
        for drone in project.drones {
            let is_stuck = drone.liveness == "working" && is_updated_stale(&drone.updated);
            drones.push(DroneStatusBrief {
                name: drone.name,
                liveness: drone.liveness,
                progress: drone.progress,
                elapsed: drone.elapsed,
                cost_usd: drone.cost.total_usd,
                is_stuck,
            });
        }
    }

    drones
}

/// A drone is "stuck" if its `updated` timestamp is older than 5 minutes.
fn is_updated_stale(updated: &str) -> bool {
    let Ok(updated_dt) = chrono::DateTime::parse_from_rfc3339(updated) else {
        // Try the alternative format used by some status files: "2024-01-15 12:30:45"
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(updated, "%Y-%m-%d %H:%M:%S") {
            let dt = naive.and_utc();
            let elapsed = chrono::Utc::now().signed_duration_since(dt);
            return elapsed.num_minutes() >= 5;
        }
        return false;
    };
    let elapsed = chrono::Utc::now().signed_duration_since(updated_dt);
    elapsed.num_minutes() >= 5
}
