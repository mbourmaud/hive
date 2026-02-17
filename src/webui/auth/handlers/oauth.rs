use axum::Json;
use serde::Deserialize;

use crate::webui::error::{ApiError, ApiResult};
use crate::webui::extractors::ValidJson;

use super::super::credentials::{self, Credentials};
use super::super::dto::{OAuthAuthorizeResponse, OAuthCallbackRequest, SetupApiKeyRequest};
use super::super::pkce::pkce_store;

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
    // Save per-profile if possible, otherwise global
    let active_name = crate::commands::profile::get_active_profile().unwrap_or_default();
    credentials::save_credentials_for_profile(&active_name, &creds)
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
            // Save per-profile if possible, otherwise global
            let active_name = crate::commands::profile::get_active_profile().unwrap_or_default();
            credentials::save_credentials_for_profile(&active_name, &creds)
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
