use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::webui::error::{ApiError, ApiResult};

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
