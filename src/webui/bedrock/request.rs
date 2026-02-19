//! Build and sign HTTP requests for the Bedrock InvokeModelWithResponseStream API.

use anyhow::{Context, Result};
use aws_credential_types::Credentials as AwsCredentials;
use aws_sigv4::http_request::{
    sign, SignableBody, SignableRequest, SigningParams, SigningSettings,
};
use aws_sigv4::sign::v4;
use std::time::SystemTime;
use tracing::{debug, info, warn};

use crate::webui::anthropic::types::MessagesRequest;
use crate::webui::auth::credentials::Credentials;

use super::aws_resolve;
use super::model::resolve_bedrock_model;

/// Resolved AWS credentials (static keys or dynamically resolved from profile).
pub(super) struct AwsCreds {
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Extract or resolve AWS credentials from any Bedrock credential variant.
///
/// Propagates `AwsCredentialError` for SSO issues so callers can skip retries.
pub(super) async fn resolve_aws_creds(
    creds: &Credentials,
) -> std::result::Result<AwsCreds, aws_resolve::AwsCredentialError> {
    match creds {
        Credentials::Bedrock {
            region,
            access_key_id,
            secret_access_key,
            session_token,
        } => {
            debug!(%region, has_session_token = session_token.is_some(), "Using static Bedrock credentials");
            Ok(AwsCreds {
                region: region.clone(),
                access_key_id: access_key_id.clone(),
                secret_access_key: secret_access_key.clone(),
                session_token: session_token.clone(),
            })
        }
        Credentials::BedrockProfile {
            region,
            aws_profile,
        } => {
            debug!(%region, %aws_profile, "Resolving Bedrock credentials from AWS profile");
            let resolved = aws_resolve::resolve_from_profile(aws_profile, region).await?;
            info!(%region, %aws_profile, "AWS profile credentials resolved successfully");
            Ok(AwsCreds {
                region: region.clone(),
                access_key_id: resolved.access_key_id,
                secret_access_key: resolved.secret_access_key,
                session_token: resolved.session_token,
            })
        }
        _ => {
            warn!("resolve_aws_creds called with non-Bedrock credentials");
            Err(aws_resolve::AwsCredentialError::Other(anyhow::anyhow!(
                "Expected Bedrock credentials"
            )))
        }
    }
}

/// Build and send a signed Bedrock InvokeModelWithResponseStream request.
pub(super) async fn build_bedrock_request(
    creds: &Credentials,
    request: &MessagesRequest,
) -> Result<reqwest::Response> {
    let aws = resolve_aws_creds(creds)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let model_id = resolve_bedrock_model(&request.model);
    let url = format!(
        "https://bedrock-runtime.{}.amazonaws.com/model/{model_id}/invoke-with-response-stream",
        aws.region
    );

    info!(
        requested_model = %request.model,
        resolved_model_id = %model_id,
        region = %aws.region,
        %url,
        "Building Bedrock request"
    );

    let body = build_bedrock_body(request)?;
    let body_bytes = serde_json::to_vec(&body)?;
    debug!(
        body_size = body_bytes.len(),
        "Bedrock request body serialized"
    );

    let signed_headers = sign_aws_request("POST", &url, &body_bytes, &aws, "bedrock")?;
    debug!(header_count = signed_headers.len(), "SigV4 headers signed");

    let client = reqwest::Client::new();
    let mut req_builder = client
        .post(&url)
        .header("content-type", "application/json")
        .body(body_bytes);

    for (name, value) in &signed_headers {
        req_builder = req_builder.header(name.as_str(), value.as_str());
    }

    req_builder
        .send()
        .await
        .context("Sending Bedrock API request")
}

/// Transform the Anthropic MessagesRequest into a Bedrock-compatible body.
///
/// Bedrock differences:
/// - No `model` field (it's in the URL)
/// - No `stream` field (InvokeModelWithResponseStream is always streaming)
/// - Uses `anthropic_version: "bedrock-2023-05-31"`
fn build_bedrock_body(request: &MessagesRequest) -> Result<serde_json::Value> {
    let mut body = serde_json::to_value(request).context("Serializing request")?;

    if let Some(obj) = body.as_object_mut() {
        obj.remove("model");
        obj.remove("stream");
        obj.insert(
            "anthropic_version".to_string(),
            serde_json::Value::String("bedrock-2023-05-31".to_string()),
        );

        let thinking_enabled = request
            .thinking
            .as_ref()
            .is_some_and(|t| t.thinking_type == "enabled");
        if thinking_enabled {
            obj.remove("temperature");
        }
    }

    Ok(body)
}

/// SigV4-sign a request using resolved AWS credentials.
pub(super) fn sign_aws_request(
    method: &str,
    url: &str,
    body: &[u8],
    aws: &AwsCreds,
    service: &str,
) -> Result<Vec<(String, String)>> {
    let identity = AwsCredentials::new(
        &aws.access_key_id,
        &aws.secret_access_key,
        aws.session_token.clone(),
        None,
        "hive",
    )
    .into();

    let settings = SigningSettings::default();

    let signing_params = v4::SigningParams::builder()
        .identity(&identity)
        .region(&aws.region)
        .name(service)
        .time(SystemTime::now())
        .settings(settings)
        .build()
        .context("Building SigV4 signing params")?;

    let signable_request = SignableRequest::new(
        method,
        url,
        std::iter::once(("content-type", "application/json")),
        SignableBody::Bytes(body),
    )
    .context("Creating signable request")?;

    let (signing_instructions, _signature) =
        sign(signable_request, &SigningParams::V4(signing_params))
            .context("SigV4 signing failed")?
            .into_parts();

    let headers: Vec<(String, String)> = signing_instructions
        .headers()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();

    Ok(headers)
}
