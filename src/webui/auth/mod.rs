pub mod credentials;
pub mod dto;
pub mod handlers;
pub mod pkce;

use axum::{
    routing::{delete, get, post},
    Router,
};

pub fn routes() -> Router {
    Router::new()
        .route("/api/auth/status", get(handlers::auth_status))
        .route("/api/auth/setup", post(handlers::setup_api_key))
        .route("/api/auth/oauth/authorize", get(handlers::oauth_authorize))
        .route("/api/auth/oauth/callback", post(handlers::oauth_callback))
        .route("/api/auth/logout", delete(handlers::logout))
        .route("/api/models", get(handlers::list_models))
        .route("/api/commands", get(handlers::list_commands))
}
