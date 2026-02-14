use std::path::Path;

use anyhow::{Context, Result};

pub async fn execute(input: &serde_json::Value, cwd: &Path) -> Result<String> {
    let pattern = input
        .get("pattern")
        .and_then(|v| v.as_str())
        .context("Missing required parameter: pattern")?;

    let search_path = input
        .get("path")
        .and_then(|v| v.as_str())
        .map(|p| {
            if Path::new(p).is_absolute() {
                p.to_string()
            } else {
                cwd.join(p).to_string_lossy().to_string()
            }
        })
        .unwrap_or_else(|| cwd.to_string_lossy().to_string());

    let case_insensitive = input.get("-i").and_then(|v| v.as_bool()).unwrap_or(false);

    let glob_filter = input.get("glob").and_then(|v| v.as_str());

    let output_mode = input
        .get("output_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("files_with_matches");

    let mut args: Vec<String> = Vec::new();

    match output_mode {
        "files_with_matches" => args.push("--files-with-matches".to_string()),
        "count" => args.push("--count".to_string()),
        "content" => {
            args.push("--line-number".to_string());
        }
        _ => args.push("--files-with-matches".to_string()),
    }

    if case_insensitive {
        args.push("--ignore-case".to_string());
    }

    if let Some(glob) = glob_filter {
        args.push("--glob".to_string());
        args.push(glob.to_string());
    }

    args.push("--".to_string());
    args.push(pattern.to_string());
    args.push(search_path);

    let output = tokio::process::Command::new("rg")
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to run ripgrep (rg). Is it installed?")?
        .wait_with_output()
        .await
        .context("Ripgrep execution failed")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() && stdout.is_empty() {
        if !stderr.is_empty() {
            anyhow::bail!("Grep error: {}", stderr.trim());
        }
        return Ok("No matches found".to_string());
    }

    Ok(stdout.to_string())
}
