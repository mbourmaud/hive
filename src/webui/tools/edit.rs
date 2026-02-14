use std::path::Path;

use anyhow::{bail, Context, Result};

use super::sandbox;

pub async fn execute(input: &serde_json::Value, cwd: &Path) -> Result<String> {
    let file_path = input
        .get("file_path")
        .and_then(|v| v.as_str())
        .context("Missing required parameter: file_path")?;
    let old_string = input
        .get("old_string")
        .and_then(|v| v.as_str())
        .context("Missing required parameter: old_string")?;
    let new_string = input
        .get("new_string")
        .and_then(|v| v.as_str())
        .context("Missing required parameter: new_string")?;
    let replace_all = input
        .get("replace_all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let resolved = sandbox::validate_path(file_path, cwd)?;

    let content = tokio::fs::read_to_string(&resolved)
        .await
        .with_context(|| format!("Cannot read file '{}'", resolved.display()))?;

    let match_count = content.matches(old_string).count();

    if match_count == 0 {
        bail!(
            "old_string not found in '{}'. Make sure the string matches exactly.",
            resolved.display()
        );
    }

    if !replace_all && match_count > 1 {
        bail!(
            "old_string matches {} locations in '{}'. Use replace_all: true or provide more context to make it unique.",
            match_count,
            resolved.display()
        );
    }

    let new_content = if replace_all {
        content.replace(old_string, new_string)
    } else {
        content.replacen(old_string, new_string, 1)
    };

    tokio::fs::write(&resolved, &new_content)
        .await
        .with_context(|| format!("Cannot write file '{}'", resolved.display()))?;

    let replaced = if replace_all { match_count } else { 1 };
    Ok(format!(
        "Replaced {} occurrence(s) in {}",
        replaced,
        resolved.display()
    ))
}
