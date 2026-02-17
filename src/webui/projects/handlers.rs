use std::convert::Infallible;
use std::path::Path;
use std::time::Duration;

use axum::extract::{Multipart, Path as AxumPath};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures_util::stream::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::config;
use crate::webui::error::{ApiError, ApiResult};
use crate::webui::extractors::ValidJson;

use super::detection;
use super::types::{CreateProjectRequest, ProjectProfile, UpdateProjectRequest};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn entry_to_profile(entry: &config::ProjectEntry) -> ProjectProfile {
    let id = entry.id.clone().unwrap_or_default();
    let image_url = entry
        .image_path
        .as_ref()
        .filter(|p| Path::new(p).exists())
        .map(|_| format!("/api/registry/projects/{id}/image"));

    ProjectProfile {
        id,
        name: entry.name.clone(),
        path: entry.path.clone(),
        color_theme: entry.color_theme.clone(),
        image_url,
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

pub async fn list_projects() -> ApiResult<Json<Vec<ProjectProfile>>> {
    let mut registry = config::load_projects_registry()
        .map_err(|e| ApiError::Internal(e.context("Failed to load registry")))?;

    let mut changed = config::ensure_project_ids(&mut registry);

    // Prune entries whose path no longer exists (e.g. temp test directories)
    let before = registry.projects.len();
    registry.projects.retain(|p| Path::new(&p.path).exists());
    if registry.projects.len() != before {
        changed = true;
    }

    if changed {
        let _ = config::save_projects_registry(&registry);
    }

    let profiles: Vec<ProjectProfile> = registry.projects.iter().map(entry_to_profile).collect();
    Ok(Json(profiles))
}

pub async fn create_project(
    ValidJson(body): ValidJson<CreateProjectRequest>,
) -> ApiResult<Json<ProjectProfile>> {
    let path = Path::new(&body.path);
    if !path.exists() {
        return Err(ApiError::BadRequest(format!(
            "Path '{}' does not exist",
            body.path
        )));
    }

    let abs_path = tokio::fs::canonicalize(path)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to resolve path: {e}")))?;
    let abs_path_str = abs_path.to_string_lossy().to_string();

    let mut registry = config::load_projects_registry()
        .map_err(|e| ApiError::Internal(e.context("Failed to load registry")))?;

    // Check for duplicate path
    if registry.projects.iter().any(|p| p.path == abs_path_str) {
        return Err(ApiError::Conflict(format!(
            "Project at '{}' is already registered",
            abs_path_str
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let entry = config::ProjectEntry {
        path: abs_path_str,
        name: body.name,
        id: Some(id),
        color_theme: body.color_theme,
        image_path: None,
    };

    registry.projects.push(entry.clone());
    config::save_projects_registry(&registry)
        .map_err(|e| ApiError::Internal(e.context("Failed to save registry")))?;

    Ok(Json(entry_to_profile(&entry)))
}

pub async fn get_project(AxumPath(id): AxumPath<String>) -> ApiResult<Json<ProjectProfile>> {
    let entry = config::find_project_by_id(&id)
        .map_err(|e| ApiError::Internal(e.context("Failed to load registry")))?
        .ok_or_else(|| ApiError::NotFound(format!("Project '{id}' not found")))?;

    Ok(Json(entry_to_profile(&entry)))
}

pub async fn update_project(
    AxumPath(id): AxumPath<String>,
    ValidJson(body): ValidJson<UpdateProjectRequest>,
) -> ApiResult<Json<ProjectProfile>> {
    let mut entry = config::find_project_by_id(&id)
        .map_err(|e| ApiError::Internal(e.context("Failed to load registry")))?
        .ok_or_else(|| ApiError::NotFound(format!("Project '{id}' not found")))?;

    if let Some(name) = body.name {
        entry.name = name;
    }
    if let Some(color_theme) = body.color_theme {
        entry.color_theme = Some(color_theme);
    }

    config::update_project(&entry)
        .map_err(|e| ApiError::Internal(e.context("Failed to update project")))?;

    Ok(Json(entry_to_profile(&entry)))
}

pub async fn delete_project(AxumPath(id): AxumPath<String>) -> ApiResult<Json<serde_json::Value>> {
    config::remove_project(&id)
        .map_err(|e| ApiError::Internal(e.context("Failed to delete project")))?;

    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn detect_project_context(
    AxumPath(id): AxumPath<String>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let entry = config::find_project_by_id(&id)
        .map_err(|e| ApiError::Internal(e.context("Failed to load registry")))?
        .ok_or_else(|| ApiError::NotFound(format!("Project '{id}' not found")))?;

    let path = std::path::PathBuf::from(&entry.path);
    if !path.exists() {
        return Err(ApiError::BadRequest(format!(
            "Project path '{}' does not exist",
            entry.path
        )));
    }

    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(async move {
        detection::detect_with_events(&path, tx).await;
    });

    let stream = ReceiverStream::new(rx).map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_default();
        Ok(Event::default().data(json))
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    ))
}

pub async fn upload_image(
    AxumPath(id): AxumPath<String>,
    mut multipart: Multipart,
) -> ApiResult<Json<ProjectProfile>> {
    let mut entry = config::find_project_by_id(&id)
        .map_err(|e| ApiError::Internal(e.context("Failed to load registry")))?
        .ok_or_else(|| ApiError::NotFound(format!("Project '{id}' not found")))?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name != "image" {
            continue;
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {e}")))?;

        // Max 2MB
        if data.len() > 2 * 1024 * 1024 {
            return Err(ApiError::BadRequest("Image must be under 2MB".to_string()));
        }

        let images_dir = config::images_dir()
            .map_err(|e| ApiError::Internal(e.context("Failed to get images directory")))?;
        tokio::fs::create_dir_all(&images_dir).await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Failed to create images directory: {e}"))
        })?;

        let image_path = images_dir.join(format!("{id}.png"));
        tokio::fs::write(&image_path, &data)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to write image: {e}")))?;

        entry.image_path = Some(image_path.to_string_lossy().to_string());
        config::update_project(&entry)
            .map_err(|e| ApiError::Internal(e.context("Failed to update project")))?;

        return Ok(Json(entry_to_profile(&entry)));
    }

    Err(ApiError::BadRequest(
        "No 'image' field found in upload".to_string(),
    ))
}

pub async fn serve_image(AxumPath(id): AxumPath<String>) -> ApiResult<impl IntoResponse> {
    let entry = config::find_project_by_id(&id)
        .map_err(|e| ApiError::Internal(e.context("Failed to load registry")))?
        .ok_or_else(|| ApiError::NotFound(format!("Project '{id}' not found")))?;

    let image_path = entry
        .image_path
        .ok_or_else(|| ApiError::NotFound("No image set for this project".to_string()))?;

    let data = tokio::fs::read(&image_path)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to read image: {e}")))?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "image/png"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        data,
    ))
}

/// Open a native folder picker dialog and return the selected path.
///
/// The dialog runs on a blocking thread via `spawn_blocking` since `rfd`
/// uses the OS file dialog which blocks the calling thread.
pub async fn pick_folder() -> ApiResult<Json<serde_json::Value>> {
    let path = tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Select project folder")
            .pick_folder()
    })
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("Dialog thread panicked: {e}")))?;

    match path {
        Some(p) => Ok(Json(
            serde_json::json!({ "path": p.to_string_lossy(), "name": p.file_name().map(|n| n.to_string_lossy().to_string()) }),
        )),
        None => Ok(Json(serde_json::json!({ "path": null, "name": null }))),
    }
}
