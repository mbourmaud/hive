use std::collections::HashSet;
use std::path::Path;

use axum::Json;
use serde::Deserialize;

use crate::webui::error::{ApiError, ApiResult};
use crate::webui::extractors::ValidJson;

use super::credentials::{self, Credentials};
use super::dto::{
    AuthStatusResponse, CustomCommand, ModelInfo, OAuthAuthorizeResponse, OAuthCallbackRequest,
    SetupApiKeyRequest,
};
use super::pkce::pkce_store;

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

pub async fn setup_api_key(
    ValidJson(body): ValidJson<SetupApiKeyRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let api_key = body.api_key.trim().to_string();

    // Validate the API key by calling GET /v1/models
    let client = reqwest::Client::new();
    let validate_res = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await;

    match validate_res {
        Ok(res) if res.status().is_success() => {}
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(ApiError::Unauthorized(format!(
                "Invalid API key ({status}): {body}"
            )));
        }
        Err(e) => {
            return Err(ApiError::Internal(anyhow::anyhow!(
                "Could not reach Anthropic API: {e}"
            )));
        }
    }

    let creds = Credentials::ApiKey { api_key };
    credentials::save_credentials(&creds)
        .map_err(|e| ApiError::Internal(e.context("Failed to save credentials")))?;

    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn oauth_authorize() -> ApiResult<Json<OAuthAuthorizeResponse>> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use sha2::Digest;

    let (code_verifier, code_challenge, state) = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let verifier_bytes: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();
        let code_verifier = URL_SAFE_NO_PAD.encode(&verifier_bytes);
        let challenge_hash = sha2::Sha256::digest(code_verifier.as_bytes());
        let code_challenge = URL_SAFE_NO_PAD.encode(challenge_hash);
        let state_bytes: Vec<u8> = (0..48).map(|_| rng.gen::<u8>()).collect();
        let state = URL_SAFE_NO_PAD.encode(&state_bytes);
        (code_verifier, code_challenge, state)
    };

    pkce_store()
        .lock()
        .await
        .insert(state.clone(), code_verifier);

    let client_id = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
    let redirect_uri = "https://console.anthropic.com/oauth/code/callback";
    let scopes = "org:create_api_key user:profile user:inference";

    let scope_encoded = scopes.replace(' ', "+").replace(':', "%3A");

    let authorize_url = format!(
        "https://claude.ai/oauth/authorize?code=true&response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        scope_encoded,
        urlencoding::encode(&state),
        urlencoding::encode(&code_challenge),
    );

    Ok(Json(OAuthAuthorizeResponse {
        authorize_url,
        state,
    }))
}

pub async fn oauth_callback(
    ValidJson(body): ValidJson<OAuthCallbackRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let store = pkce_store();

    let (code, state) = if let Some(idx) = body.code.find('#') {
        (
            body.code[..idx].to_string(),
            body.code[idx + 1..].to_string(),
        )
    } else {
        let guard = store.lock().await;
        if let Some((state, _)) = guard.iter().next() {
            (body.code.clone(), state.clone())
        } else {
            return Err(ApiError::BadRequest(
                "No pending OAuth flow found. Please start the flow again.".to_string(),
            ));
        }
    };

    let code_verifier = {
        let mut guard = store.lock().await;
        guard
            .remove(&state)
            .ok_or_else(|| ApiError::BadRequest("Invalid or expired OAuth state".to_string()))?
    };

    let client = reqwest::Client::new();
    let token_res = client
        .post("https://console.anthropic.com/v1/oauth/token")
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
            "state": state,
            "redirect_uri": "https://console.anthropic.com/oauth/code/callback",
            "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
            "code_verifier": code_verifier,
        }))
        .send()
        .await;

    match token_res {
        Ok(res) if res.status().is_success() => {
            #[derive(Deserialize)]
            struct TokenResponse {
                access_token: String,
                refresh_token: String,
                expires_in: i64,
            }

            let token: TokenResponse = res.json().await.map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to parse token response: {e}"))
            })?;

            let expires_at = chrono::Utc::now().timestamp() + token.expires_in;
            let creds = Credentials::OAuth {
                access_token: token.access_token,
                refresh_token: token.refresh_token,
                expires_at,
            };
            credentials::save_credentials(&creds)
                .map_err(|e| ApiError::Internal(e.context("Failed to save credentials")))?;

            Ok(Json(serde_json::json!({"ok": true})))
        }
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            Err(ApiError::Internal(anyhow::anyhow!(
                "Token exchange failed ({status}): {body}"
            )))
        }
        Err(e) => Err(ApiError::Internal(anyhow::anyhow!(
            "Could not reach token endpoint: {e}"
        ))),
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

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct KeychainOauth {
        access_token: String,
        refresh_token: String,
        expires_at: i64,
    }

    #[derive(Deserialize)]
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
async fn read_keychain_credentials() -> Result<Option<String>, std::io::Error> {
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

pub async fn list_models() -> ApiResult<Json<Vec<ModelInfo>>> {
    if let Some(models) = try_fetch_models().await {
        return Ok(Json(models));
    }
    Ok(Json(Vec::new()))
}

async fn try_fetch_models() -> Option<Vec<ModelInfo>> {
    if let Ok(Some(creds)) = credentials::load_credentials() {
        if let Ok((header_name, header_value)) = credentials::get_auth_header(&creds).await {
            let is_oauth = matches!(creds, Credentials::OAuth { .. });
            if let Ok(models) = fetch_models_from_api(header_name, &header_value, is_oauth).await {
                return Some(models);
            }
        }
    }

    if let Some(token) = read_claude_code_access_token().await {
        if let Ok(models) =
            fetch_models_from_api("Authorization", &format!("Bearer {token}"), true).await
        {
            return Some(models);
        }
    }

    None
}

async fn read_claude_code_access_token() -> Option<String> {
    let raw = read_keychain_credentials().await.ok()??;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct KeychainCreds {
        claude_ai_oauth: Option<KeychainOauth>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct KeychainOauth {
        access_token: String,
    }

    let parsed: KeychainCreds = serde_json::from_str(raw.trim()).ok()?;
    let oauth = parsed.claude_ai_oauth?;
    if oauth.access_token.is_empty() {
        return None;
    }
    Some(oauth.access_token)
}

async fn fetch_models_from_api(
    header_name: &str,
    header_value: &str,
    is_oauth: bool,
) -> Result<Vec<ModelInfo>, ()> {
    let client = reqwest::Client::new();
    let mut req = client
        .get("https://api.anthropic.com/v1/models?limit=100")
        .header(header_name, header_value)
        .header("anthropic-version", "2023-06-01");

    if is_oauth {
        req = req.header("anthropic-beta", "oauth-2025-04-20");
    }

    let res = req.send().await.map_err(|_| ())?;

    if !res.status().is_success() {
        return Err(());
    }

    let body: serde_json::Value = res.json().await.map_err(|_| ())?;
    let data = body.get("data").and_then(|d| d.as_array()).ok_or(())?;

    let mut models: Vec<ModelInfo> = Vec::new();
    for item in data {
        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let display_name = item
            .get("display_name")
            .and_then(|v| v.as_str())
            .unwrap_or(id);

        if !id.starts_with("claude-") {
            continue;
        }

        models.push(ModelInfo {
            id: id.to_string(),
            name: display_name.to_string(),
            description: String::new(),
        });
    }

    if models.is_empty() {
        return Err(());
    }

    Ok(models)
}

pub async fn list_commands() -> ApiResult<Json<Vec<CustomCommand>>> {
    let mut commands: Vec<CustomCommand> = Vec::new();
    let mut seen = HashSet::new();

    if let Ok(cwd) = std::env::current_dir() {
        let project_dir = cwd.join(".claude").join("commands");
        scan_commands_dir(&project_dir, "project", &mut commands, &mut seen);
    }

    if let Some(home) = dirs::home_dir() {
        let user_dir = home.join(".claude").join("commands");
        scan_commands_dir(&user_dir, "user", &mut commands, &mut seen);

        let tools_dir = home.join(".claude").join("commands").join("tools");
        scan_commands_dir(&tools_dir, "tools", &mut commands, &mut seen);
    }

    Ok(Json(commands))
}

fn scan_commands_dir(
    dir: &Path,
    source: &str,
    commands: &mut Vec<CustomCommand>,
    seen: &mut HashSet<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if path.is_dir() {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if name.is_empty() || seen.contains(&name) {
            continue;
        }

        let description = std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                    .map(|l| {
                        let trimmed = l.trim();
                        let truncated: String = trimmed.chars().take(80).collect();
                        if truncated.len() < trimmed.len() {
                            format!("{truncated}…")
                        } else {
                            truncated
                        }
                    })
            })
            .unwrap_or_default();

        seen.insert(name.clone());
        commands.push(CustomCommand {
            name,
            description,
            source: source.to_string(),
        });
    }
}
