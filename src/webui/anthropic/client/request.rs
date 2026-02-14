use anyhow::{Context, Result};

use super::super::types::MessagesRequest;
use crate::webui::auth::credentials::{self, Credentials};

/// Read user metadata from ~/.claude.json for OAuth requests.
fn read_claude_metadata() -> Option<(String, String)> {
    let home = dirs::home_dir()?;
    let path = home.join(".claude.json");
    let data = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&data).ok()?;
    let user_id = json.get("userID")?.as_str()?.to_string();
    let account_uuid = json
        .get("oauthAccount")
        .and_then(|o| o.get("accountUuid"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Some((user_id, account_uuid))
}

/// Build and send the HTTP request to the Anthropic Messages API.
pub(super) async fn build_request(
    creds: &Credentials,
    request: &MessagesRequest,
) -> Result<reqwest::Response> {
    let is_oauth = matches!(creds, Credentials::OAuth { .. });
    let (auth_header_name, auth_header_value) = credentials::get_auth_header(creds).await?;

    let thinking_enabled = request
        .thinking
        .as_ref()
        .is_some_and(|t| t.thinking_type == "enabled");

    let mut body = serde_json::to_value(request).context("Serializing request")?;

    // When thinking is enabled, the API requires no temperature
    if thinking_enabled {
        if let Some(o) = body.as_object_mut() {
            o.remove("temperature");
        }
    }

    if is_oauth {
        if let Some((user_id, account_uuid)) = read_claude_metadata() {
            let meta_user_id = if account_uuid.is_empty() {
                user_id
            } else {
                format!("user_{user_id}_account_{account_uuid}")
            };
            body["metadata"] = serde_json::json!({ "user_id": meta_user_id });
        }
        if let Some(o) = body.as_object_mut() {
            o.remove("temperature");
            o.remove("tool_choice");
        }
    }

    let url = if is_oauth {
        "https://api.anthropic.com/v1/messages?beta=true"
    } else {
        "https://api.anthropic.com/v1/messages"
    };

    let client = reqwest::Client::new();
    let mut req_builder = client
        .post(url)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .header(auth_header_name, &auth_header_value);

    // Build anthropic-beta header
    let mut betas: Vec<&str> = Vec::new();
    if thinking_enabled {
        betas.push("interleaved-thinking-2025-05-14");
    }
    if is_oauth {
        betas.push("oauth-2025-04-20");
    }
    if !betas.is_empty() {
        req_builder = req_builder.header("anthropic-beta", betas.join(","));
    }

    if is_oauth {
        req_builder = req_builder
            .header("user-agent", "claude-cli/2.1.7 (external, cli)")
            .header("anthropic-dangerous-direct-browser-access", "true")
            .header("x-app", "cli")
            .header("x-stainless-arch", "x64")
            .header("x-stainless-lang", "js")
            .header("x-stainless-os", "Darwin")
            .header("x-stainless-package-version", "0.70.0")
            .header("x-stainless-runtime", "node")
            .header("x-stainless-runtime-version", "v24.3.0")
            .header("x-stainless-retry-count", "0")
            .header("x-stainless-timeout", "600")
            .header("x-stainless-helper-method", "stream")
            .header("accept", "application/json");
    }

    req_builder
        .json(&body)
        .send()
        .await
        .context("Sending Anthropic API request")
}
