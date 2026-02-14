use std::path::Path;

use anyhow::{Context, Result};

use super::sandbox;

const DEFAULT_TIMEOUT_MS: u64 = 120_000;
const MAX_TIMEOUT_MS: u64 = 600_000;
const MAX_OUTPUT_BYTES: usize = 30_000;

pub async fn execute(input: &serde_json::Value, cwd: &Path) -> Result<String> {
    let command = input
        .get("command")
        .and_then(|v| v.as_str())
        .context("Missing required parameter: command")?;

    sandbox::check_dangerous_command(command)?;

    let timeout_ms = input
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .min(MAX_TIMEOUT_MS);

    let child = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn bash process")?;

    let timeout_duration = std::time::Duration::from_millis(timeout_ms);
    let output = match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
        Ok(result) => result.context("Command execution failed")?,
        Err(_) => {
            anyhow::bail!("Command timed out after {}ms", timeout_ms);
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    let mut result = String::new();

    if !stdout.is_empty() {
        let truncated_stdout = truncate_output(&stdout, MAX_OUTPUT_BYTES);
        result.push_str(&truncated_stdout);
    }

    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        let truncated_stderr = truncate_output(&stderr, MAX_OUTPUT_BYTES);
        result.push_str("STDERR:\n");
        result.push_str(&truncated_stderr);
    }

    if exit_code != 0 {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&format!("Exit code: {exit_code}"));
    }

    if result.is_empty() {
        result = format!("Command completed with exit code {exit_code}");
    }

    Ok(result)
}

fn truncate_output(output: &str, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        return output.to_string();
    }

    let truncated = &output[..max_bytes];
    // Find the last newline to avoid cutting mid-line
    let end = truncated.rfind('\n').unwrap_or(max_bytes);
    let remaining = output.len() - end;
    format!(
        "{}\n\n... (truncated, {remaining} bytes omitted)",
        &output[..end]
    )
}
