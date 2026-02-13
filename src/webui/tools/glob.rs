use std::path::Path;

use anyhow::{Context, Result};

pub async fn execute(input: &serde_json::Value, cwd: &Path) -> Result<String> {
    let pattern = input
        .get("pattern")
        .and_then(|v| v.as_str())
        .context("Missing required parameter: pattern")?;

    let search_dir = input
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

    let full_pattern = format!("{}/{}", search_dir, pattern);

    // Use glob crate via blocking task to avoid blocking the async runtime
    let matches = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
        let mut results: Vec<(std::time::SystemTime, String)> = Vec::new();

        for entry in ::glob::glob(&full_pattern)
            .with_context(|| format!("Invalid glob pattern: {full_pattern}"))?
        {
            match entry {
                Ok(path) => {
                    let mtime = path
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    results.push((mtime, path.to_string_lossy().to_string()));
                }
                Err(_) => continue,
            }
        }

        // Sort by modification time, most recent first
        results.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(results.into_iter().map(|(_, path)| path).collect())
    })
    .await
    .context("Glob task panicked")?
    .context("Glob execution failed")?;

    if matches.is_empty() {
        return Ok("No files matched".to_string());
    }

    Ok(matches.join("\n"))
}
