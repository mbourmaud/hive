pub mod aws_profiles;
mod commands;
mod models;
mod oauth;
pub mod profiles;

use axum::Json;

use crate::webui::error::{ApiError, ApiResult};

use super::credentials::{self, Credentials};
use super::dto::AuthStatusResponse;

pub use aws_profiles::{aws_sso_login, list_aws_profiles};
pub use commands::list_commands;
pub use models::list_models;
pub use oauth::{oauth_authorize, oauth_callback, setup_api_key};
pub use profiles::{
    activate_profile, active_profile, create_profile, delete_profile, list_profiles,
};

pub async fn auth_status() -> ApiResult<Json<AuthStatusResponse>> {
    let provider = credentials::resolve_provider();
    let profile_name = crate::commands::profile::get_active_profile().unwrap_or_default();

    match credentials::resolve_credentials() {
        Ok(Some(creds)) => {
            let (auth_type, expired) = match &creds {
                Credentials::ApiKey { .. } => ("api_key".to_string(), false),
                Credentials::OAuth { expires_at, .. } => (
                    "oauth".to_string(),
                    credentials::is_token_expired(*expires_at),
                ),
                Credentials::Bedrock { .. } | Credentials::BedrockProfile { .. } => {
                    ("bedrock".to_string(), false)
                }
            };
            Ok(Json(AuthStatusResponse {
                configured: true,
                auth_type: Some(auth_type),
                expired,
                profile: Some(profile_name),
                provider: Some(provider.to_string()),
            }))
        }
        _ => Ok(Json(AuthStatusResponse {
            configured: false,
            auth_type: None,
            expired: false,
            profile: Some(profile_name),
            provider: Some(provider.to_string()),
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
