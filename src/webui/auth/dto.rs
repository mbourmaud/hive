use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AuthStatusResponse {
    pub configured: bool,
    #[serde(rename = "type")]
    pub auth_type: Option<String>,
    pub expired: bool,
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
