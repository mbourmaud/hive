pub mod agents;
pub mod context;
pub mod dto;
pub mod handlers;
pub mod persistence;
pub mod session;

use axum::{
    routing::{delete, get, post},
    Router,
};

pub use session::SessionStore;

pub fn routes(sessions: SessionStore) -> Router {
    Router::new()
        .route("/api/chat/sessions", post(handlers::create_session))
        .route("/api/chat/sessions", get(handlers::list_sessions))
        .route("/api/chat/agents", get(handlers::list_agents))
        .route(
            "/api/chat/sessions/{id}/stream",
            get(handlers::stream_session),
        )
        .route(
            "/api/chat/sessions/{id}/message",
            post(handlers::send_message),
        )
        .route(
            "/api/chat/sessions/{id}/abort",
            post(handlers::abort_session),
        )
        .route(
            "/api/chat/sessions/{id}",
            delete(handlers::delete_session).patch(handlers::update_session),
        )
        .route(
            "/api/chat/sessions/{id}/history",
            get(handlers::session_history),
        )
        .with_state(sessions)
}
