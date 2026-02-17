//! Resolve AWS credentials via the `aws-config` credential chain (SSO, env vars, profiles, etc.).

use anyhow::{Context, Result};
use aws_credential_types::provider::ProvideCredentials;

/// Temporary AWS credentials resolved from the credential chain.
pub struct ResolvedAwsCreds {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Resolve AWS credentials using the given profile name and region.
///
/// Handles SSO, static credentials, env vars, and any other source in the
/// AWS credential provider chain. If SSO tokens are expired, returns a
/// user-friendly error with `aws sso login` instructions.
pub async fn resolve_from_profile(profile_name: &str, region: &str) -> Result<ResolvedAwsCreds> {
    let aws_region = aws_config::Region::new(region.to_owned());

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .profile_name(profile_name)
        .region(aws_region)
        .load()
        .await;

    let provider = config
        .credentials_provider()
        .context("No credentials provider found in AWS config")?;

    let creds = provider.provide_credentials().await.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("expired") || msg.contains("SSO") || msg.contains("sso") {
            anyhow::anyhow!(
                "AWS SSO token expired for profile '{profile_name}'. \
                 Run: aws sso login --profile {profile_name}"
            )
        } else {
            anyhow::anyhow!("Failed to resolve AWS credentials for profile '{profile_name}': {msg}")
        }
    })?;

    Ok(ResolvedAwsCreds {
        access_key_id: creds.access_key_id().to_string(),
        secret_access_key: creds.secret_access_key().to_string(),
        session_token: creds.session_token().map(|s| s.to_string()),
    })
}
