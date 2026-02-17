use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AuthStatusResponse {
    pub configured: bool,
    #[serde(rename = "type")]
    pub auth_type: Option<String>,
    pub expired: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SetupApiKeyRequest {
    #[garde(length(min = 10, max = 256))]
    pub api_key: String,
}

#[derive(Debug, Serialize)]
pub struct OAuthAuthorizeResponse {
    pub authorize_url: String,
    pub state: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct OAuthCallbackRequest {
    #[garde(length(min = 1))]
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomCommand {
    pub name: String,
    pub description: String,
    pub source: String,
}

// ── Profile DTOs ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub provider: String,
    pub is_active: bool,
    pub has_credentials: bool,
}

#[derive(Debug, Serialize)]
pub struct ActiveProfileResponse {
    pub name: String,
    pub provider: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProfileRequest {
    #[garde(length(min = 1, max = 64))]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub description: Option<String>,
    #[garde(skip)]
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub secret_access_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub session_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub aws_profile: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ActivateProfileRequest {
    #[garde(length(min = 1, max = 64))]
    pub name: String,
}
