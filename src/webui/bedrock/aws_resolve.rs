//! Resolve AWS credentials via the `aws-config` credential chain (SSO, env vars, profiles, etc.).

use anyhow::{Context, Result};
use aws_credential_types::provider::ProvideCredentials;

/// Temporary AWS credentials resolved from the credential chain.
pub struct ResolvedAwsCreds {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Error type for credential resolution — distinguishes auth failures from transient errors.
#[derive(Debug)]
pub enum AwsCredentialError {
    /// SSO token expired or missing — requires `aws sso login`.
    SsoLoginRequired { profile: String },
    /// Other credential resolution failure.
    Other(anyhow::Error),
}

impl std::fmt::Display for AwsCredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SsoLoginRequired { profile } => {
                write!(f, "AWS SSO session expired for profile '{profile}'")
            }
            Self::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AwsCredentialError {}

/// Check if an error message indicates an SSO login is required.
fn is_sso_error(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("expired")
        || lower.contains("sso")
        || lower.contains("token")
        || lower.contains("no credentials")
        || lower.contains("while loading credentials")
}

/// Resolve AWS credentials using the given profile name and region.
///
/// Returns `SsoLoginRequired` for auth failures so callers can skip retries
/// and trigger re-authentication instead.
pub async fn resolve_from_profile(
    profile_name: &str,
    region: &str,
) -> std::result::Result<ResolvedAwsCreds, AwsCredentialError> {
    let aws_region = aws_config::Region::new(region.to_owned());

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .profile_name(profile_name)
        .region(aws_region)
        .load()
        .await;

    let provider = config.credentials_provider().ok_or_else(|| {
        AwsCredentialError::Other(anyhow::anyhow!(
            "No credentials provider found in AWS config for profile '{profile_name}'"
        ))
    })?;

    let creds = provider.provide_credentials().await.map_err(|e| {
        let msg = e.to_string();
        if is_sso_error(&msg) {
            AwsCredentialError::SsoLoginRequired {
                profile: profile_name.to_string(),
            }
        } else {
            AwsCredentialError::Other(anyhow::anyhow!(
                "Failed to resolve AWS credentials for profile '{profile_name}': {msg}"
            ))
        }
    })?;

    Ok(ResolvedAwsCreds {
        access_key_id: creds.access_key_id().to_string(),
        secret_access_key: creds.secret_access_key().to_string(),
        session_token: creds.session_token().map(|s| s.to_string()),
    })
}

/// Run `aws sso login --profile <name>` and return success/failure.
pub async fn run_sso_login(profile_name: &str) -> Result<()> {
    let output = tokio::process::Command::new("aws")
        .args(["sso", "login", "--profile", profile_name])
        .output()
        .await
        .context("Failed to run `aws sso login`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("aws sso login failed: {stderr}");
    }

    Ok(())
}
