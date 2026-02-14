use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

pub fn credentials_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hive")
        .join("credentials.json")
}

pub fn load_credentials() -> Result<Option<Credentials>> {
    let path = credentials_path();
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path).context("Reading credentials file")?;
    let creds: Credentials = serde_json::from_str(&data).context("Parsing credentials")?;
    Ok(Some(creds))
}

pub fn save_credentials(creds: &Credentials) -> Result<()> {
    let path = credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Creating credentials directory")?;
    }
    let json = serde_json::to_string_pretty(creds)?;
    std::fs::write(&path, json).context("Writing credentials file")?;
    Ok(())
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

    save_credentials(&creds)?;
    Ok(creds)
}

/// Returns the auth header name and value. Refreshes OAuth tokens if expired.
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
    }
}
