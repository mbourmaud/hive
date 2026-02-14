use serde::{Deserialize, Serialize};

// ── Response DTOs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProfile {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub git: Option<GitContext>,
    pub runtimes: Vec<RuntimeInfo>,
    pub key_files: Vec<String>,
    pub open_pr: Option<PrInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitContext {
    pub branch: String,
    pub remote_url: String,
    pub platform: String,
    pub ahead: u32,
    pub behind: u32,
    pub dirty_count: u32,
    pub last_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub name: String,
    pub version: Option<String>,
    pub marker_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    pub is_draft: bool,
}

// ── SSE Detection Events ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DetectionEvent {
    StepStarted {
        step: String,
        label: String,
    },
    StepCompleted {
        step: String,
        label: String,
        result: serde_json::Value,
    },
    StepFailed {
        step: String,
        label: String,
        error: String,
    },
    AllComplete {
        context: ProjectContext,
    },
}

// ── Request DTOs ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, garde::Validate)]
pub struct CreateProjectRequest {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(length(min = 1, max = 500))]
    pub path: String,
    #[garde(skip)]
    pub color_theme: Option<String>,
}

#[derive(Debug, Deserialize, garde::Validate)]
pub struct UpdateProjectRequest {
    #[garde(length(min = 1, max = 100))]
    pub name: Option<String>,
    #[garde(skip)]
    pub color_theme: Option<String>,
}
