use std::path::Path;

use anyhow::{Context, Result};

use super::sandbox;

pub async fn execute(input: &serde_json::Value, cwd: &Path) -> Result<String> {
    let file_path = input
        .get("file_path")
        .and_then(|v| v.as_str())
        .context("Missing required parameter: file_path")?;

    let resolved = sandbox::validate_path(file_path, cwd)?;

    let content = tokio::fs::read_to_string(&resolved)
        .await
        .with_context(|| format!("Cannot read file '{}'", resolved.display()))?;

    let offset = input.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let limit = input
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let lines: Vec<&str> = content.lines().collect();
    let start = if offset > 0 { offset - 1 } else { 0 };
    let end = match limit {
        Some(l) => (start + l).min(lines.len()),
        None => lines.len(),
    };

    if start >= lines.len() {
        return Ok(String::new());
    }

    let mut result = String::new();
    for (i, line) in lines[start..end].iter().enumerate() {
        let line_num = start + i + 1;
        result.push_str(&format!("{line_num:>6}\t{line}\n"));
    }

    Ok(result)
}
