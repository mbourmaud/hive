//! Bedrock model resolution and discovery.

use anyhow::Result;
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::webui::auth::credentials::Credentials;

use super::request::{resolve_aws_creds, sign_aws_request};

/// Resolve a short model alias to a Bedrock inference profile ID.
///
/// Newer Claude models (4.x) require cross-region inference profile IDs
/// (`us.anthropic.{model}`) instead of plain model IDs (`anthropic.{model}`).
/// See: <https://docs.aws.amazon.com/bedrock/latest/userguide/inference-profiles.html>
pub fn resolve_bedrock_model(short: &str) -> &str {
    match short.to_lowercase().as_str() {
        "sonnet" | "claude-sonnet" | "sonnet-4.5" => "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "opus" | "claude-opus" | "opus-4" => "us.anthropic.claude-opus-4-20250514-v1:0",
        "opus-4.6" | "claude-opus-4.6" => "us.anthropic.claude-opus-4-6-20260213-v1:0",
        "haiku" | "claude-haiku" | "haiku-4.5" => "us.anthropic.claude-haiku-4-5-20251001-v1:0",
        // Full Anthropic model IDs → wrap for Bedrock inference profile
        "claude-sonnet-4-5-20250929" => "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "claude-opus-4-20250514" => "us.anthropic.claude-opus-4-20250514-v1:0",
        "claude-opus-4-6-20260213" => "us.anthropic.claude-opus-4-6-20260213-v1:0",
        "claude-haiku-4-5-20251001" => "us.anthropic.claude-haiku-4-5-20251001-v1:0",
        // Already a Bedrock inference profile ID — pass through
        other if other.starts_with("us.anthropic.") || other.starts_with("eu.anthropic.") => short,
        // Legacy plain Bedrock ID → add us. prefix for inference profile
        other if other.starts_with("anthropic.") => {
            // Can't return owned string from &str fn, so common cases covered above.
            // Unknown anthropic.* IDs passed through as-is (may fail at API level).
            short
        }
        // Default
        _ => "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
    }
}

/// Known Bedrock Claude models for the model listing API (fallback).
/// Uses cross-region inference profile IDs required for Claude 4.x models.
pub fn bedrock_model_list() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "us.anthropic.claude-opus-4-6-20260213-v1:0",
            "Claude Opus 4.6 (Bedrock)",
        ),
        (
            "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
            "Claude Sonnet 4.5 (Bedrock)",
        ),
        (
            "us.anthropic.claude-opus-4-20250514-v1:0",
            "Claude Opus 4 (Bedrock)",
        ),
        (
            "us.anthropic.claude-haiku-4-5-20251001-v1:0",
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
        Ok(models) if !models.is_empty() => {
            info!(count = models.len(), "Discovered Bedrock models via API");
            for m in &models {
                debug!(id = %m.model_id, name = %m.display_name, "Bedrock model");
            }
            models
        }
        Ok(_) => {
            warn!("Bedrock ListFoundationModels returned no Claude models, using fallback");
            fallback_models()
        }
        Err(e) => {
            warn!(error = %e, "Bedrock model discovery failed, using fallback list");
            fallback_models()
        }
    }
}

fn fallback_models() -> Vec<DiscoveredModel> {
    bedrock_model_list()
        .into_iter()
        .map(|(id, name)| DiscoveredModel {
            model_id: id.to_string(),
            display_name: name.to_string(),
        })
        .collect()
}

async fn try_discover(creds: &Credentials) -> Result<Vec<DiscoveredModel>> {
    let aws = resolve_aws_creds(creds)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let url = format!(
        "https://bedrock.{}.amazonaws.com/foundation-models?byProvider=Anthropic",
        aws.region
    );
    debug!(%url, "Discovering Bedrock models");

    let signed_headers = sign_aws_request("GET", &url, &[], &aws, "bedrock")?;

    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    for (name, value) in &signed_headers {
        req = req.header(name.as_str(), value.as_str());
    }

    let res = req.send().await?;
    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        warn!(%status, %body, "ListFoundationModels failed");
        anyhow::bail!("ListFoundationModels returned {status}: {body}");
    }

    let body: ListModelsResponse = res.json().await?;
    debug!(
        raw_count = body.model_summaries.len(),
        "ListFoundationModels raw response"
    );
    let mut models: Vec<DiscoveredModel> = body
        .model_summaries
        .into_iter()
        .filter(|m| m.model_id.contains("claude"))
        .map(|m| {
            // ListFoundationModels returns base model IDs (anthropic.claude-*).
            // Claude 4.x requires cross-region inference profile IDs (us.anthropic.claude-*).
            let inference_id = if m.model_id.starts_with("anthropic.") {
                format!("us.{}", m.model_id)
            } else {
                m.model_id
            };
            let display = format!("{} (Bedrock)", m.model_name);
            DiscoveredModel {
                model_id: inference_id,
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
