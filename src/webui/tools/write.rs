use std::path::Path;

use anyhow::{Context, Result};

use super::sandbox;

pub async fn execute(input: &serde_json::Value, cwd: &Path) -> Result<String> {
    let file_path = input
        .get("file_path")
        .and_then(|v| v.as_str())
        .context("Missing required parameter: file_path")?;
    let content = input
        .get("content")
        .and_then(|v| v.as_str())
        .context("Missing required parameter: content")?;

    let resolved = sandbox::validate_path(file_path, cwd)?;

    // Create parent directories if needed
    if let Some(parent) = resolved.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Cannot create directory '{}'", parent.display()))?;
    }

    tokio::fs::write(&resolved, content)
        .await
        .with_context(|| format!("Cannot write file '{}'", resolved.display()))?;

    let line_count = content.lines().count();
    Ok(format!(
        "Wrote {} lines to {}",
        line_count,
        resolved.display()
    ))
}
