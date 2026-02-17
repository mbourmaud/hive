//! AWS profile discovery and SSO login handlers.

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::webui::bedrock::aws_resolve;
use crate::webui::error::{ApiError, ApiResult};

#[derive(Debug, Clone, Serialize)]
pub struct AwsProfileInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sso_start_url: Option<String>,
}

/// Parse `~/.aws/config` and return profile entries with region + SSO hints.
pub async fn list_aws_profiles() -> ApiResult<Json<Vec<AwsProfileInfo>>> {
    let config_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".aws")
        .join("config");

    let content = match tokio::fs::read_to_string(&config_path).await {
        Ok(c) => c,
        Err(_) => return Ok(Json(Vec::new())),
    };

    Ok(Json(parse_aws_config(&content)))
}

/// Line-by-line parser for AWS config INI files.
fn parse_aws_config(content: &str) -> Vec<AwsProfileInfo> {
    let mut profiles = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_region: Option<String> = None;
    let mut current_sso_url: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Flush previous profile
            if let Some(name) = current_name.take() {
                profiles.push(AwsProfileInfo {
                    name,
                    region: current_region.take(),
                    sso_start_url: current_sso_url.take(),
                });
            }
            current_region = None;
            current_sso_url = None;

            let section = &trimmed[1..trimmed.len() - 1].trim();
            current_name = if let Some(stripped) = section.strip_prefix("profile ") {
                Some(stripped.trim().to_string())
            } else if *section == "default" {
                Some("default".to_string())
            } else {
                None
            };
            continue;
        }

        if current_name.is_none() {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "region" => current_region = Some(value.to_string()),
                "sso_start_url" => current_sso_url = Some(value.to_string()),
                _ => {}
            }
        }
    }

    // Flush last profile
    if let Some(name) = current_name {
        profiles.push(AwsProfileInfo {
            name,
            region: current_region,
            sso_start_url: current_sso_url,
        });
    }

    profiles
}

#[derive(Deserialize)]
pub struct SsoLoginRequest {
    pub profile: String,
}

/// `POST /api/aws/sso-login` — run `aws sso login --profile <name>`.
pub async fn aws_sso_login(
    Json(body): Json<SsoLoginRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    aws_resolve::run_sso_login(&body.profile)
        .await
        .map_err(ApiError::Internal)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_aws_config() {
        let content = r#"
[default]
region = us-east-1

[profile bedrock]
region = us-west-2
sso_start_url = https://my-org.awsapps.com/start
sso_account_id = 123456789012
sso_role_name = PowerUser
sso_region = us-east-1

[profile dev]
region = eu-west-1
"#;
        let profiles = parse_aws_config(content);
        assert_eq!(profiles.len(), 3);

        assert_eq!(profiles[0].name, "default");
        assert_eq!(profiles[0].region.as_deref(), Some("us-east-1"));
        assert!(profiles[0].sso_start_url.is_none());

        assert_eq!(profiles[1].name, "bedrock");
        assert_eq!(profiles[1].region.as_deref(), Some("us-west-2"));
        assert_eq!(
            profiles[1].sso_start_url.as_deref(),
            Some("https://my-org.awsapps.com/start")
        );

        assert_eq!(profiles[2].name, "dev");
        assert_eq!(profiles[2].region.as_deref(), Some("eu-west-1"));
        assert!(profiles[2].sso_start_url.is_none());
    }

    #[test]
    fn test_parse_empty_config() {
        assert!(parse_aws_config("").is_empty());
    }
}
