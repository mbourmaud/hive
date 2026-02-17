//! Profile management REST API handlers.

use axum::extract::Path;
use axum::Json;

use crate::commands::profile;
use crate::commands::provider::{BedrockConfig, Provider};
use crate::webui::auth::credentials;
use crate::webui::auth::dto::{
    ActivateProfileRequest, ActiveProfileResponse, CreateProfileRequest, ProfileResponse,
};
use crate::webui::error::{ApiError, ApiResult};
use crate::webui::extractors::ValidJson;

/// GET /api/profiles — list all profiles with active indicator.
pub async fn list_profiles() -> ApiResult<Json<Vec<ProfileResponse>>> {
    let profiles = profile::list_profiles()
        .map_err(|e| ApiError::Internal(e.context("Failed to list profiles")))?;
    let active = profile::get_active_profile().unwrap_or_default();

    let responses: Vec<ProfileResponse> = profiles
        .into_iter()
        .map(|p| {
            let has_creds = match p.provider {
                Provider::Bedrock => p.bedrock.as_ref().is_some_and(|bc| {
                    bc.aws_profile.is_some()
                        || (bc.access_key_id.is_some() && bc.secret_access_key.is_some())
                }),
                Provider::Anthropic => credentials::has_profile_credentials(&p.name),
            };
            ProfileResponse {
                name: p.name.clone(),
                description: p.description,
                provider: p.provider.to_string(),
                is_active: p.name == active,
                has_credentials: has_creds,
            }
        })
        .collect();

    Ok(Json(responses))
}

/// GET /api/profiles/active — active profile name + provider.
pub async fn active_profile() -> ApiResult<Json<ActiveProfileResponse>> {
    let active_name = profile::get_active_profile().unwrap_or_default();
    let provider = credentials::resolve_provider();

    Ok(Json(ActiveProfileResponse {
        name: active_name,
        provider: provider.to_string(),
    }))
}

/// POST /api/profiles — create a new profile.
pub async fn create_profile(
    ValidJson(body): ValidJson<CreateProfileRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let profiles_exist = profile::list_profiles()
        .ok()
        .map(|ps| ps.iter().any(|p| p.name == body.name))
        .unwrap_or(false);

    if profiles_exist {
        return Err(ApiError::Conflict(format!(
            "Profile '{}' already exists",
            body.name
        )));
    }

    let provider = match body.provider.as_str() {
        "bedrock" => Provider::Bedrock,
        _ => Provider::Anthropic,
    };

    let bedrock = if provider == Provider::Bedrock {
        let region = body.region.clone().ok_or_else(|| {
            ApiError::BadRequest("Region is required for Bedrock profiles".to_string())
        })?;

        if let Some(ref aws_profile) = body.aws_profile {
            // AWS Profile mode — no static keys needed
            Some(BedrockConfig {
                region,
                aws_profile: Some(aws_profile.clone()),
                access_key_id: None,
                secret_access_key: None,
                session_token: None,
            })
        } else {
            // Static keys mode
            let access_key_id = body.access_key_id.clone().ok_or_else(|| {
                ApiError::BadRequest(
                    "Access key ID is required for Bedrock static key profiles".to_string(),
                )
            })?;
            let secret_access_key = body.secret_access_key.clone().ok_or_else(|| {
                ApiError::BadRequest(
                    "Secret access key is required for Bedrock static key profiles".to_string(),
                )
            })?;
            Some(BedrockConfig {
                region,
                aws_profile: None,
                access_key_id: Some(access_key_id),
                secret_access_key: Some(secret_access_key),
                session_token: body.session_token.clone(),
            })
        }
    } else {
        None
    };

    // If Anthropic profile with API key, save per-profile
    if provider == Provider::Anthropic {
        if let Some(ref key) = body.api_key {
            credentials::save_credentials_for_profile(
                &body.name,
                &credentials::Credentials::ApiKey {
                    api_key: key.clone(),
                },
            )
            .map_err(|e| ApiError::Internal(e.context("Failed to save API key")))?;
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let new_profile = profile::Profile {
        name: body.name.clone(),
        description: body.description,
        claude_wrapper: "claude".to_string(),
        environment: None,
        provider,
        bedrock,
        created: now.clone(),
        updated: now,
    };

    profile::save_profile(&new_profile)
        .map_err(|e| ApiError::Internal(e.context("Failed to save profile")))?;

    Ok(Json(serde_json::json!({ "ok": true, "name": body.name })))
}

/// POST /api/profiles/activate — set the active profile.
pub async fn activate_profile(
    ValidJson(body): ValidJson<ActivateProfileRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Verify profile exists
    profile::load_profile(&body.name)
        .map_err(|_| ApiError::NotFound(format!("Profile '{}' not found", body.name)))?;

    profile::use_profile(body.name.clone())
        .map_err(|e| ApiError::Internal(e.context("Failed to activate profile")))?;

    Ok(Json(serde_json::json!({ "ok": true, "active": body.name })))
}

/// DELETE /api/profiles/{name} — delete a profile.
pub async fn delete_profile(Path(name): Path<String>) -> ApiResult<Json<serde_json::Value>> {
    if name == "default" {
        return Err(ApiError::BadRequest(
            "Cannot delete the default profile".to_string(),
        ));
    }

    // Verify it exists
    profile::load_profile(&name)
        .map_err(|_| ApiError::NotFound(format!("Profile '{name}' not found")))?;

    // If active, clear active profile
    let active = profile::get_active_profile().unwrap_or_default();
    if active == name {
        let _ = profile::use_profile("default".to_string());
    }

    // Clean up per-profile credentials file
    let creds_path = credentials::profile_credentials_path(&name);
    if creds_path.exists() {
        let _ = std::fs::remove_file(&creds_path);
    }

    profile::delete(name.clone())
        .map_err(|e| ApiError::Internal(e.context("Failed to delete profile")))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
