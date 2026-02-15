use axum::{extract::Path, Json};
use serde::{Deserialize, Serialize};

use crate::webui::error::{ApiError, ApiResult};

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct PlanSummary {
    pub id: String,
    pub title: String,
    pub tldr: Option<String>,
    pub task_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct PlanDetail {
    pub id: String,
    pub content: String,
    pub title: String,
    pub tldr: Option<String>,
    pub tasks: Vec<PlanTask>,
}

#[derive(Serialize)]
pub struct PlanTask {
    pub number: usize,
    pub title: String,
    pub task_type: String,
    pub model: Option<String>,
    pub files: Vec<String>,
    pub depends_on: Vec<usize>,
}

#[derive(Deserialize)]
pub struct DispatchRequest {
    #[serde(alias = "droneName")]
    pub drone_name: String,
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_model() -> String {
    "sonnet".to_string()
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn plans_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(".hive/plans")
}

fn extract_title(content: &str) -> String {
    content
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").trim().to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn extract_tldr(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.iter().position(|l| {
        let t = l.trim().to_lowercase();
        t == "## tl;dr" || t == "## tldr"
    })?;

    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| l.trim().starts_with("## "))
        .map(|(i, _)| i)
        .unwrap_or(lines.len());

    let tldr: String = lines[start + 1..end]
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if tldr.is_empty() {
        None
    } else {
        Some(tldr)
    }
}

fn file_timestamps(path: &std::path::Path) -> (String, String) {
    let meta = std::fs::metadata(path).ok();
    let created = meta
        .as_ref()
        .and_then(|m| m.created().ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
        .unwrap_or_default();
    let updated = meta
        .and_then(|m| m.modified().ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
        .unwrap_or_default();
    (created, updated)
}

// ── Handlers ───────────────────────────────────────────────────────────────

/// GET /api/plans — list all plans
pub async fn list_plans() -> ApiResult<Json<Vec<PlanSummary>>> {
    let dir = plans_dir();
    if !dir.is_dir() {
        return Ok(Json(vec![]));
    }

    let mut plans = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Cannot read plans dir: {e}")))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md") {
            let id = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            let tasks = crate::plan_parser::parse_tasks(&content);
            let (created_at, updated_at) = file_timestamps(&path);

            plans.push(PlanSummary {
                id,
                title: extract_title(&content),
                tldr: extract_tldr(&content),
                task_count: tasks.len(),
                created_at,
                updated_at,
            });
        }
    }

    // Sort by updated_at descending (newest first)
    plans.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(Json(plans))
}

/// GET /api/plans/{id} — read a specific plan
pub async fn get_plan(Path(id): Path<String>) -> ApiResult<Json<PlanDetail>> {
    let path = plans_dir().join(format!("{id}.md"));
    if !path.is_file() {
        return Err(ApiError::NotFound(format!("Plan '{id}' not found")));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Cannot read plan: {e}")))?;

    let parsed_tasks = crate::plan_parser::parse_tasks(&content);
    let tasks: Vec<PlanTask> = parsed_tasks
        .into_iter()
        .map(|t| PlanTask {
            number: t.number,
            title: t.title,
            task_type: t.task_type.to_string(),
            model: t.model,
            files: t.files,
            depends_on: t.depends_on,
        })
        .collect();

    Ok(Json(PlanDetail {
        id,
        title: extract_title(&content),
        tldr: extract_tldr(&content),
        content,
        tasks,
    }))
}

/// DELETE /api/plans/{id} — delete a plan
pub async fn delete_plan(Path(id): Path<String>) -> ApiResult<Json<serde_json::Value>> {
    let path = plans_dir().join(format!("{id}.md"));
    if !path.is_file() {
        return Err(ApiError::NotFound(format!("Plan '{id}' not found")));
    }

    std::fs::remove_file(&path)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Cannot delete plan: {e}")))?;

    Ok(Json(serde_json::json!({"ok": true})))
}

/// POST /api/plans/{id}/dispatch — dispatch a plan to a drone
///
/// Spawns `start::run()` on a background thread because the native team
/// coordinator blocks with its own tokio runtime (`rt.block_on`). Running it
/// directly inside the Axum handler would create a nested runtime (panic) and
/// block the HTTP response indefinitely.
pub async fn dispatch_plan(
    Path(id): Path<String>,
    Json(body): Json<DispatchRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let path = plans_dir().join(format!("{id}.md"));
    if !path.is_file() {
        return Err(ApiError::NotFound(format!("Plan '{id}' not found")));
    }

    let drone_name = body.drone_name;
    let model = body.model;

    // Spawn on a dedicated OS thread — `start::run` creates its own tokio
    // runtime internally, which cannot nest inside the Axum runtime.
    let name_for_thread = drone_name.clone();
    let model_for_thread = model.clone();
    std::thread::spawn(move || {
        if let Err(e) =
            crate::commands::start::run(name_for_thread.clone(), false, model_for_thread, 3, false)
        {
            eprintln!("[hive] Dispatch failed for '{}': {:#}", name_for_thread, e);
        }
    });

    Ok(Json(serde_json::json!({
        "ok": true,
        "droneName": drone_name,
    })))
}
