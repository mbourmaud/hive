pub mod dto;
pub mod handlers;
pub mod liveness;
pub mod polling;

use axum::{routing::get, Router};
use std::sync::Arc;

pub use handlers::MonitorState;

pub fn routes(state: Arc<MonitorState>) -> Router {
    Router::new()
        .route("/api/projects", get(handlers::api_projects))
        .route("/api/drones", get(handlers::api_drones))
        .route("/api/drones/{name}", get(handlers::api_drone_detail))
        .route("/api/events", get(handlers::api_events_sse))
        .with_state(state)
}

pub fn spawn_poller(state: Arc<MonitorState>) {
    handlers::spawn_poller(state);
}
