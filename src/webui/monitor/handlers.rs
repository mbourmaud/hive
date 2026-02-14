use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use garde::Validate;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::webui::error::{ApiError, ApiResult};
use crate::webui::extractors::ValidJson;

use super::dto::{DroneInfo, ProjectInfo};
use super::polling::{poll_all_projects, SnapshotStores};

pub struct MonitorState {
    pub snapshot_stores: SnapshotStores,
    pub tx: broadcast::Sender<String>,
}

pub async fn api_projects(
    State(state): State<Arc<MonitorState>>,
) -> ApiResult<Json<Vec<ProjectInfo>>> {
    let projects = poll_all_projects(&state.snapshot_stores);
    Ok(Json(projects))
}

pub async fn api_drones(State(state): State<Arc<MonitorState>>) -> ApiResult<Json<Vec<DroneInfo>>> {
    let projects = poll_all_projects(&state.snapshot_stores);
    let all_drones: Vec<DroneInfo> = projects.into_iter().flat_map(|p| p.drones).collect();
    Ok(Json(all_drones))
}

pub async fn api_drone_detail(
    State(state): State<Arc<MonitorState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<DroneInfo>> {
    let projects = poll_all_projects(&state.snapshot_stores);
    let all_drones: Vec<DroneInfo> = projects.into_iter().flat_map(|p| p.drones).collect();
    all_drones
        .into_iter()
        .find(|d| d.name == name)
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("Drone '{name}' not found")))
}

pub async fn api_events_sse(
    State(state): State<Arc<MonitorState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(data) => Some(Ok(Event::default().data(data))),
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Spawn the background poller that pushes SSE updates every 2 seconds.
pub fn spawn_poller(state: Arc<MonitorState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let projects = poll_all_projects(&state.snapshot_stores);
            if let Ok(json) = serde_json::to_string(&projects) {
                let _ = state.tx.send(json);
            }
        }
    });
}

// ── Drone launch ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct LaunchDroneRequest {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(length(min = 1))]
    pub prompt: String,
    #[serde(default = "default_model")]
    #[garde(length(max = 100))]
    pub model: String,
    #[serde(default = "default_mode")]
    #[garde(length(max = 20))]
    pub mode: String,
}

fn default_model() -> String {
    "sonnet".to_string()
}
fn default_mode() -> String {
    "agent-team".to_string()
}

/// POST /api/drones/launch — spawn a new drone from the web UI.
///
/// This creates a temporary plan from the prompt, sets up the worktree and hooks,
/// then launches the Claude CLI subprocess. The drone will appear in the monitor
/// panel within a few seconds.
pub async fn launch_drone(
    State(_state): State<Arc<MonitorState>>,
    ValidJson(body): ValidJson<LaunchDroneRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let name = body.name.clone();
    let prompt = body.prompt.clone();
    let model = body.model.clone();

    if name.is_empty() {
        return Err(ApiError::BadRequest("Drone name is required".to_string()));
    }

    // Create a plan file from the prompt
    let plan_content = format!(
        "# {name}\n\n{prompt}\n\n## Tasks\n\n### 1. Execute\n- **type**: work\n\n{prompt}\n"
    );

    let plans_dir = std::path::PathBuf::from(".hive/plans");
    if !plans_dir.exists() {
        tokio::fs::create_dir_all(&plans_dir).await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Cannot create plans directory: {e}"))
        })?;
    }

    let plan_path = plans_dir.join(format!("{name}.md"));
    tokio::fs::write(&plan_path, &plan_content)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Cannot write plan: {e}")))?;

    // Spawn `hive start` in a blocking task (reuses all existing CLI logic)
    let result = tokio::task::spawn_blocking(move || {
        crate::commands::start::run(name.clone(), false, model, 3, false)
    })
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("Spawn failed: {e}")))?;

    match result {
        Ok(()) => Ok(Json(serde_json::json!({
            "ok": true,
            "name": body.name,
            "message": format!("Drone '{}' launched", body.name)
        }))),
        Err(e) => Err(ApiError::Internal(anyhow::anyhow!(
            "Failed to launch drone '{}': {e:#}",
            body.name
        ))),
    }
}

/// POST /api/drones/{name}/stop — stop a running drone.
pub async fn stop_drone(
    State(_state): State<Arc<MonitorState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = tokio::task::spawn_blocking(move || crate::commands::kill_clean::kill_quiet(name))
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Stop failed: {e}")))?;

    match result {
        Ok(()) => Ok(Json(serde_json::json!({"ok": true}))),
        Err(e) => Err(ApiError::Internal(anyhow::anyhow!("Failed to stop: {e:#}"))),
    }
}
