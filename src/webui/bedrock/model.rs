//! Bedrock model resolution and discovery.

use anyhow::Result;
use serde::Deserialize;

use crate::webui::auth::credentials::Credentials;

use super::request::{resolve_aws_creds, sign_aws_request};

/// Resolve a short model alias to a Bedrock model ID.
///
/// Bedrock uses its own model ID format: `anthropic.{model}-v{version}:0`.
/// Pass-through IDs that already start with `anthropic.` unchanged.
pub fn resolve_bedrock_model(short: &str) -> &str {
    match short.to_lowercase().as_str() {
        "sonnet" | "claude-sonnet" | "sonnet-4.5" => "anthropic.claude-sonnet-4-5-20250929-v1:0",
        "opus" | "claude-opus" | "opus-4" => "anthropic.claude-opus-4-20250514-v1:0",
        "opus-4.6" | "claude-opus-4.6" => "anthropic.claude-opus-4-6-20260213-v1:0",
        "haiku" | "claude-haiku" | "haiku-4.5" => "anthropic.claude-haiku-4-5-20251001-v1:0",
        // Full Anthropic model IDs → wrap for Bedrock
        "claude-sonnet-4-5-20250929" => "anthropic.claude-sonnet-4-5-20250929-v1:0",
        "claude-opus-4-20250514" => "anthropic.claude-opus-4-20250514-v1:0",
        "claude-opus-4-6-20260213" => "anthropic.claude-opus-4-6-20260213-v1:0",
        "claude-haiku-4-5-20251001" => "anthropic.claude-haiku-4-5-20251001-v1:0",
        // Already a Bedrock ID — pass through
        other if other.starts_with("anthropic.") => short,
        // Default
        _ => "anthropic.claude-sonnet-4-5-20250929-v1:0",
    }
}

/// Known Bedrock Claude models for the model listing API (fallback).
pub fn bedrock_model_list() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "anthropic.claude-opus-4-6-20260213-v1:0",
            "Claude Opus 4.6 (Bedrock)",
        ),
        (
            "anthropic.claude-sonnet-4-5-20250929-v1:0",
            "Claude Sonnet 4.5 (Bedrock)",
        ),
        (
            "anthropic.claude-opus-4-20250514-v1:0",
            "Claude Opus 4 (Bedrock)",
        ),
        (
            "anthropic.claude-haiku-4-5-20251001-v1:0",
            "Claude Haiku 4.5 (Bedrock)",
        ),
    ]
}

/// Discovered model from Bedrock's ListFoundationModels API.
pub struct DiscoveredModel {
    pub model_id: String,
    pub display_name: String,
}

/// Discover available Claude models via Bedrock's ListFoundationModels API.
///
/// Calls `GET /foundation-models?byProvider=Anthropic` on the `bedrock` service
/// and filters for Claude models. Falls back to the hardcoded list on any error.
pub async fn discover_bedrock_models(creds: &Credentials) -> Vec<DiscoveredModel> {
    match try_discover(creds).await {
        Ok(models) if !models.is_empty() => models,
        _ => bedrock_model_list()
            .into_iter()
            .map(|(id, name)| DiscoveredModel {
                model_id: id.to_string(),
                display_name: name.to_string(),
            })
            .collect(),
    }
}

async fn try_discover(creds: &Credentials) -> Result<Vec<DiscoveredModel>> {
    let aws = resolve_aws_creds(creds)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let url = format!(
        "https://bedrock.{}.amazonaws.com/foundation-models?byProvider=Anthropic",
        aws.region
    );

    let signed_headers = sign_aws_request("GET", &url, &[], &aws, "bedrock")?;

    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    for (name, value) in &signed_headers {
        req = req.header(name.as_str(), value.as_str());
    }

    let res = req.send().await?;
    if !res.status().is_success() {
        anyhow::bail!("ListFoundationModels returned {}", res.status());
    }

    let body: ListModelsResponse = res.json().await?;
    let mut models: Vec<DiscoveredModel> = body
        .model_summaries
        .into_iter()
        .filter(|m| m.model_id.contains("claude"))
        .map(|m| {
            let display = format!("{} (Bedrock)", m.model_name);
            DiscoveredModel {
                model_id: m.model_id,
                display_name: display,
            }
        })
        .collect();

    // Sort: newest first (reverse alphabetical by ID works since IDs contain dates)
    models.sort_by(|a, b| b.model_id.cmp(&a.model_id));
    Ok(models)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListModelsResponse {
    model_summaries: Vec<ModelSummary>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelSummary {
    model_id: String,
    model_name: String,
}
