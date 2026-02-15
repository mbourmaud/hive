pub mod handlers;
mod helpers;
pub mod types;

use axum::{routing::get, Router};

pub fn routes() -> Router {
    Router::new()
        .route("/api/git/status", get(handlers::git_status))
        .route("/api/git/diff", get(handlers::git_diff))
}
