pub mod detection;
pub mod handlers;
pub mod types;

use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router {
    Router::new()
        .route(
            "/api/registry/projects",
            get(handlers::list_projects).post(handlers::create_project),
        )
        .route(
            "/api/registry/projects/{id}",
            get(handlers::get_project)
                .put(handlers::update_project)
                .delete(handlers::delete_project),
        )
        .route(
            "/api/registry/projects/{id}/detect",
            get(handlers::detect_project_context),
        )
        .route(
            "/api/registry/projects/{id}/image",
            post(handlers::upload_image).get(handlers::serve_image),
        )
}
