use serde::{Deserialize, Serialize};

/// Supported API providers for chat and agentic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    #[default]
    Anthropic,
    Bedrock,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anthropic => write!(f, "anthropic"),
            Self::Bedrock => write!(f, "bedrock"),
        }
    }
}

/// AWS Bedrock connection configuration stored in a profile.
///
/// Two auth modes:
/// - **AWS Profile** (`aws_profile = Some(...)`): resolve credentials via `aws-config`
///   (SSO, env vars, credential chain). Static key fields are ignored.
/// - **Static keys** (`aws_profile = None`): uses `access_key_id` + `secret_access_key`
///   directly (backward-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BedrockConfig {
    pub region: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
}
