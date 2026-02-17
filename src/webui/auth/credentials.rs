use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::commands::profile;
use crate::commands::provider::Provider;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Credentials {
    #[serde(rename = "api_key")]
    ApiKey { api_key: String },
    #[serde(rename = "oauth")]
    OAuth {
        access_token: String,
        refresh_token: String,
        expires_at: i64,
    },
    #[serde(rename = "bedrock")]
    Bedrock {
        region: String,
        access_key_id: String,
        secret_access_key: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        session_token: Option<String>,
    },
    #[serde(rename = "bedrock_profile")]
    BedrockProfile { region: String, aws_profile: String },
}

pub fn credentials_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hive")
        .join("credentials.json")
}

/// Path to per-profile credentials file.
pub fn profile_credentials_path(profile_name: &str) -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hive")
        .join("profiles")
        .join(format!("{profile_name}.credentials.json"))
}

/// Load credentials for a specific profile. Falls back to global file.
pub fn load_credentials_for_profile(profile_name: &str) -> Result<Option<Credentials>> {
    // Try per-profile credentials first
    let profile_path = profile_credentials_path(profile_name);
    if profile_path.exists() {
        let data = std::fs::read_to_string(&profile_path).context("Reading profile credentials")?;
        let creds: Credentials = serde_json::from_str(&data).context("Parsing credentials")?;
        return Ok(Some(creds));
    }
    // Fall back to global credentials
    load_credentials()
}

/// Load credentials from the global file (backward-compatible).
pub fn load_credentials() -> Result<Option<Credentials>> {
    let path = credentials_path();
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path).context("Reading credentials file")?;
    let creds: Credentials = serde_json::from_str(&data).context("Parsing credentials")?;
    Ok(Some(creds))
}

/// Save credentials for a specific profile.
pub fn save_credentials_for_profile(profile_name: &str, creds: &Credentials) -> Result<()> {
    let path = profile_credentials_path(profile_name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Creating profiles directory")?;
    }
    let json = serde_json::to_string_pretty(creds)?;
    std::fs::write(&path, json).context("Writing profile credentials")?;
    Ok(())
}

/// Save credentials to the global file (backward-compatible).
pub fn save_credentials(creds: &Credentials) -> Result<()> {
    let path = credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Creating credentials directory")?;
    }
    let json = serde_json::to_string_pretty(creds)?;
    std::fs::write(&path, json).context("Writing credentials file")?;
    Ok(())
}

/// Check if a specific profile has Anthropic credentials stored.
pub fn has_profile_credentials(profile_name: &str) -> bool {
    profile_credentials_path(profile_name).exists() || credentials_path().exists()
}

pub fn is_token_expired(expires_at: i64) -> bool {
    let now = chrono::Utc::now().timestamp();
    now >= expires_at - 60
}

pub async fn refresh_oauth_token(refresh_token: &str) -> Result<Credentials> {
    let client = reqwest::Client::new();
    let res = client
        .post("https://console.anthropic.com/v1/oauth/token")
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
        }))
        .send()
        .await
        .context("Sending refresh token request")?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        anyhow::bail!("OAuth token refresh failed ({status}): {body}");
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: String,
        expires_in: i64,
    }

    let token: TokenResponse = res.json().await.context("Parsing token response")?;
    let expires_at = chrono::Utc::now().timestamp() + token.expires_in;

    let creds = Credentials::OAuth {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at,
    };

    // Save to per-profile if active profile exists, otherwise global
    let active_name = profile::get_active_profile().unwrap_or_default();
    if profile_credentials_path(&active_name).exists() {
        save_credentials_for_profile(&active_name, &creds)?;
    } else {
        save_credentials(&creds)?;
    }
    Ok(creds)
}

/// Returns the auth header name and value. Refreshes OAuth tokens if expired.
/// Not applicable for Bedrock credentials (which use SigV4 signing instead).
pub async fn get_auth_header(creds: &Credentials) -> Result<(&'static str, String)> {
    match creds {
        Credentials::ApiKey { api_key } => Ok(("x-api-key", api_key.clone())),
        Credentials::OAuth {
            access_token,
            refresh_token,
            expires_at,
        } => {
            if is_token_expired(*expires_at) {
                let refreshed = refresh_oauth_token(refresh_token).await?;
                match &refreshed {
                    Credentials::OAuth { access_token, .. } => {
                        Ok(("Authorization", format!("Bearer {access_token}")))
                    }
                    _ => unreachable!(),
                }
            } else {
                Ok(("Authorization", format!("Bearer {access_token}")))
            }
        }
        Credentials::Bedrock { .. } | Credentials::BedrockProfile { .. } => {
            anyhow::bail!("Bedrock credentials use SigV4 signing, not auth headers")
        }
    }
}

/// Resolve credentials from the active profile, falling back to global file.
///
/// Priority:
/// 1. Active profile's bedrock config → `Credentials::Bedrock`
/// 2. Per-profile credentials file (`~/.config/hive/profiles/{name}.credentials.json`)
/// 3. Global credentials file (`~/.config/hive/credentials.json`)
pub fn resolve_credentials() -> Result<Option<Credentials>> {
    let active_name = profile::get_active_profile().unwrap_or_default();
    if let Ok(active) = profile::load_active_profile() {
        if active.provider == Provider::Bedrock {
            if let Some(ref bc) = active.bedrock {
                // AWS Profile mode: resolve credentials at request time
                if let Some(ref profile_name) = bc.aws_profile {
                    return Ok(Some(Credentials::BedrockProfile {
                        region: bc.region.clone(),
                        aws_profile: profile_name.clone(),
                    }));
                }
                // Static keys mode (backward-compatible)
                if let (Some(key_id), Some(secret)) = (&bc.access_key_id, &bc.secret_access_key) {
                    return Ok(Some(Credentials::Bedrock {
                        region: bc.region.clone(),
                        access_key_id: key_id.clone(),
                        secret_access_key: secret.clone(),
                        session_token: bc.session_token.clone(),
                    }));
                }
            }
        }
    }
    // Anthropic: try per-profile credentials, then global
    load_credentials_for_profile(&active_name)
}

/// Resolve the active provider from the active profile.
pub fn resolve_provider() -> Provider {
    profile::load_active_profile()
        .map(|p| p.provider)
        .unwrap_or_default()
}
