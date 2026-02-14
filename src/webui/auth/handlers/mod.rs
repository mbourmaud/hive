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

pub async fn import_claude_code_credentials() -> ApiResult<Json<serde_json::Value>> {
    let raw = read_keychain_credentials()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to read keychain: {e}")))?;
    let raw = match raw {
        Some(r) => r,
        None => {
            return Err(ApiError::NotFound(
                "No Claude Code credentials found in keychain. Run 'claude' CLI first to authenticate."
                    .to_string(),
            ));
        }
    };

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct KeychainOauth {
        access_token: String,
        refresh_token: String,
        expires_at: i64,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct KeychainCreds {
        claude_ai_oauth: Option<KeychainOauth>,
    }

    let parsed: KeychainCreds = serde_json::from_str(raw.trim())
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to parse keychain data: {e}")))?;

    let oauth = match parsed.claude_ai_oauth {
        Some(o) if !o.access_token.is_empty() => o,
        _ => {
            return Err(ApiError::NotFound(
                "Claude Code is not authenticated with OAuth. Run 'claude' CLI to log in first."
                    .to_string(),
            ));
        }
    };

    // Normalize millisecond timestamps to seconds
    let expires_at = if oauth.expires_at > 1_000_000_000_000 {
        oauth.expires_at / 1000
    } else {
        oauth.expires_at
    };

    let creds = Credentials::OAuth {
        access_token: oauth.access_token,
        refresh_token: oauth.refresh_token,
        expires_at,
    };

    credentials::save_credentials(&creds)
        .map_err(|e| ApiError::Internal(e.context("Failed to save credentials")))?;

    Ok(Json(serde_json::json!({"ok": true, "type": "oauth"})))
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
