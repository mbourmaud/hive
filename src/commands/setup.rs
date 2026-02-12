use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command as ProcessCommand;

/// Detect the project type and run setup commands before launching the team lead.
/// This replaces the team lead's "Environment Setup" step for structured plans.
///
/// Runs synchronously — if setup fails, the drone doesn't start.
pub fn run_setup(worktree: &Path) -> Result<()> {
    let mut ran_something = false;

    // Node.js: package.json
    if worktree.join("package.json").exists() {
        ran_something = true;
        let pm = detect_node_package_manager(worktree);
        println!(
            "    {} Detected Node.js project ({})",
            "→".bright_blue(),
            pm
        );
        run_cmd(worktree, &pm, &["install"])?;
    }

    // Rust: Cargo.toml
    if worktree.join("Cargo.toml").exists() {
        ran_something = true;
        println!("    {} Detected Rust project", "→".bright_blue());
        run_cmd(worktree, "cargo", &["check"])?;
    }

    // Go: go.mod
    if worktree.join("go.mod").exists() {
        ran_something = true;
        println!("    {} Detected Go project", "→".bright_blue());
        run_cmd(worktree, "go", &["mod", "download"])?;
    }

    // Python: pyproject.toml or requirements.txt
    if worktree.join("pyproject.toml").exists() {
        ran_something = true;
        println!(
            "    {} Detected Python project (pyproject.toml)",
            "→".bright_blue()
        );
        // Try uv first, then pip
        if which_exists("uv") {
            run_cmd(worktree, "uv", &["sync"])?;
        } else {
            run_cmd(worktree, "pip", &["install", "-e", "."])?;
        }
    } else if worktree.join("requirements.txt").exists() {
        ran_something = true;
        println!(
            "    {} Detected Python project (requirements.txt)",
            "→".bright_blue()
        );
        run_cmd(worktree, "pip", &["install", "-r", "requirements.txt"])?;
    }

    // Prisma codegen
    if worktree.join("prisma").exists() || worktree.join("prisma/schema.prisma").exists() {
        println!("    {} Running Prisma generate", "→".bright_blue());
        let pm = detect_node_package_manager(worktree);
        let npx = if pm == "pnpm" { "pnpm" } else { "npx" };
        let args = if pm == "pnpm" {
            vec!["exec", "prisma", "generate"]
        } else {
            vec!["prisma", "generate"]
        };
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
        run_cmd(worktree, npx, &args_ref)?;
    }

    if !ran_something {
        println!(
            "    {} No recognized project type detected, skipping setup",
            "→".bright_blue()
        );
    }

    Ok(())
}

/// Detect Node.js package manager: pnpm > yarn > npm
fn detect_node_package_manager(worktree: &Path) -> String {
    if worktree.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if worktree.join("yarn.lock").exists() {
        "yarn".to_string()
    } else if worktree.join("bun.lockb").exists() || worktree.join("bun.lock").exists() {
        "bun".to_string()
    } else {
        "npm".to_string()
    }
}

/// Check if a command exists on PATH
fn which_exists(cmd: &str) -> bool {
    ProcessCommand::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run a command in the worktree directory, failing with a clear error message.
fn run_cmd(worktree: &Path, cmd: &str, args: &[&str]) -> Result<()> {
    let display = format!("{} {}", cmd, args.join(" "));
    println!("    {} Running: {}", "→".bright_blue(), display.dimmed());

    let output = ProcessCommand::new(cmd)
        .args(args)
        .current_dir(worktree)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Setup command failed: `{}`\n{}", display, stderr.trim());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_node_package_manager_pnpm() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect_node_package_manager(dir.path()), "pnpm");
    }

    #[test]
    fn test_detect_node_package_manager_yarn() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
        assert_eq!(detect_node_package_manager(dir.path()), "yarn");
    }

    #[test]
    fn test_detect_node_package_manager_bun() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("bun.lockb"), "").unwrap();
        assert_eq!(detect_node_package_manager(dir.path()), "bun");
    }

    #[test]
    fn test_detect_node_package_manager_npm_fallback() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_node_package_manager(dir.path()), "npm");
    }

    #[test]
    fn test_run_setup_no_project_files() {
        let dir = TempDir::new().unwrap();
        // Should succeed (no-op) when no project files are found
        run_setup(dir.path()).unwrap();
    }
}
