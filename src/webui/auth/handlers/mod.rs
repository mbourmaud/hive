mod commands;
mod models;
mod oauth;

use axum::Json;

use crate::webui::error::{ApiError, ApiResult};

use super::credentials::{self, Credentials};
use super::dto::AuthStatusResponse;

pub use commands::list_commands;
pub use models::list_models;
pub use oauth::{oauth_authorize, oauth_callback, setup_api_key};

pub async fn auth_status() -> ApiResult<Json<AuthStatusResponse>> {
    match credentials::load_credentials() {
        Ok(Some(creds)) => {
            let (auth_type, expired) = match &creds {
                Credentials::ApiKey { .. } => ("api_key".to_string(), false),
                Credentials::OAuth { expires_at, .. } => (
                    "oauth".to_string(),
                    credentials::is_token_expired(*expires_at),
                ),
            };
            Ok(Json(AuthStatusResponse {
                configured: true,
                auth_type: Some(auth_type),
                expired,
            }))
        }
        _ => Ok(Json(AuthStatusResponse {
            configured: false,
            auth_type: None,
            expired: false,
        })),
    }
}

pub async fn logout() -> ApiResult<Json<serde_json::Value>> {
    let path = credentials::credentials_path();
    if path.exists() {
        tokio::fs::remove_file(&path).await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Failed to remove credentials: {e}"))
        })?;
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

/// Read the raw Claude Code keychain entry. Returns `Ok(None)` when no entry exists.
pub(crate) async fn read_keychain_credentials() -> Result<Option<String>, std::io::Error> {
    let output = tokio::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}
