use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::backend::agent_team::prompts::build_verification_commands;

/// Configuration for a quality gate check.
pub struct QualityGateConfig {
    pub command: String,
    pub timeout: Duration,
    pub cwd: PathBuf,
}

/// Result of running a quality gate.
pub enum GateResult {
    Passed,
    Failed { output: String },
    Timeout,
}

/// Run a quality gate command and return the result.
pub async fn run_quality_gate(config: &QualityGateConfig) -> GateResult {
    let result = tokio::time::timeout(config.timeout, run_command(config)).await;

    match result {
        Ok(Ok(output)) => {
            if output.success {
                GateResult::Passed
            } else {
                let text = truncate(&output.combined, 2000);
                GateResult::Failed { output: text }
            }
        }
        Ok(Err(e)) => GateResult::Failed {
            output: format!("Failed to run quality gate: {e}"),
        },
        Err(_) => GateResult::Timeout,
    }
}

/// Build a quality gate config from project languages.
/// Returns None if no verification commands are available.
pub fn build_gate_config(project_languages: &[String], cwd: &Path) -> Option<QualityGateConfig> {
    let commands = build_verification_commands(project_languages);
    // Take the first command (typically `cargo check` or `npm run build`)
    let first_command = commands
        .lines()
        .find(|line| line.starts_with("- `") || line.starts_with("```"))
        .and_then(|line| {
            if line.starts_with("- `") {
                // Extract from "- `command`" format
                let start = line.find('`')? + 1;
                let end = line[start..].find('`')? + start;
                Some(line[start..end].to_string())
            } else {
                None
            }
        });

    // Fallback: detect from languages directly
    let command = first_command.or_else(|| {
        for lang in project_languages {
            match lang.to_lowercase().as_str() {
                "rust" => return Some("cargo check".to_string()),
                "typescript" | "javascript" => return Some("npx tsc --noEmit".to_string()),
                "python" => return Some("python -m py_compile".to_string()),
                "go" => return Some("go build ./...".to_string()),
                _ => {}
            }
        }
        None
    })?;

    Some(QualityGateConfig {
        command,
        timeout: Duration::from_secs(120),
        cwd: cwd.to_path_buf(),
    })
}

struct CommandOutput {
    success: bool,
    combined: String,
}

async fn run_command(config: &QualityGateConfig) -> anyhow::Result<CommandOutput> {
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&config.command)
        .current_dir(&config.cwd)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    Ok(CommandOutput {
        success: output.status.success(),
        combined,
    })
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max_len..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quality_gate_pass() {
        let config = QualityGateConfig {
            command: "true".to_string(),
            timeout: Duration::from_secs(5),
            cwd: std::env::temp_dir(),
        };
        let result = run_quality_gate(&config).await;
        assert!(matches!(result, GateResult::Passed));
    }

    #[tokio::test]
    async fn test_quality_gate_fail() {
        let config = QualityGateConfig {
            command: "echo 'error: something wrong' && false".to_string(),
            timeout: Duration::from_secs(5),
            cwd: std::env::temp_dir(),
        };
        let result = run_quality_gate(&config).await;
        assert!(matches!(result, GateResult::Failed { .. }));
    }

    #[tokio::test]
    async fn test_quality_gate_timeout() {
        let config = QualityGateConfig {
            command: "sleep 10".to_string(),
            timeout: Duration::from_millis(100),
            cwd: std::env::temp_dir(),
        };
        let result = run_quality_gate(&config).await;
        assert!(matches!(result, GateResult::Timeout));
    }

    #[test]
    fn test_build_gate_config_rust() {
        let config = build_gate_config(&["rust".to_string()], Path::new("/tmp"));
        assert!(config.is_some());
        assert!(config.unwrap().command.contains("cargo"));
    }

    #[test]
    fn test_build_gate_config_empty() {
        let config = build_gate_config(&[], Path::new("/tmp"));
        assert!(config.is_none());
    }
}
