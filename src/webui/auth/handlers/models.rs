use axum::Json;
use serde::Deserialize;

use crate::commands::provider::Provider;
use crate::webui::bedrock::model::{bedrock_model_list, discover_bedrock_models};
use crate::webui::error::ApiResult;

use super::super::credentials::{self, Credentials};
use super::super::dto::ModelInfo;
use super::read_keychain_credentials;

pub async fn list_models() -> ApiResult<Json<Vec<ModelInfo>>> {
    // Bedrock provider → discover models dynamically (falls back to hardcoded list)
    if credentials::resolve_provider() == Provider::Bedrock {
        if let Ok(Some(creds)) = credentials::resolve_credentials() {
            let discovered = discover_bedrock_models(&creds).await;
            let models = discovered
                .into_iter()
                .map(|m| ModelInfo {
                    id: m.model_id,
                    name: m.display_name,
                    description: String::new(),
                })
                .collect();
            return Ok(Json(models));
        }
        let fallback: Vec<ModelInfo> = bedrock_model_list()
            .into_iter()
            .map(|(id, name)| ModelInfo {
                id: id.to_string(),
                name: name.to_string(),
                description: String::new(),
            })
            .collect();
        return Ok(Json(fallback));
    }

    if let Some(models) = try_fetch_models().await {
        return Ok(Json(models));
    }
    Ok(Json(Vec::new()))
}

async fn try_fetch_models() -> Option<Vec<ModelInfo>> {
    if let Ok(Some(creds)) = credentials::resolve_credentials() {
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
