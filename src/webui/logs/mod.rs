pub mod formatter;
pub mod handlers;

use axum::{routing::get, Router};

pub fn routes() -> Router {
    Router::new()
        .route("/api/logs/{name}", get(handlers::api_logs_sse))
        .route(
            "/api/logs/{project_path}/{name}",
            get(handlers::api_logs_project_sse),
        )
}
