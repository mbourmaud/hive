use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::process::Command as ProcessCommand;

pub fn get_project_name(project_root: &std::path::Path) -> Result<String> {
    project_root
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .context("Failed to get directory name")
}

pub fn create_worktree(
    path: &std::path::Path,
    branch: &str,
    explicit_base: Option<&str>,
) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Fetch latest from origin to ensure we have up-to-date refs
    println!("  {} Fetching latest from origin...", "→".bright_blue());
    let _ = ProcessCommand::new("git")
        .args(["fetch", "origin"])
        .output();

    // Determine the base ref for the worktree
    let base_ref = if let Some(base) = explicit_base {
        if base == "master" || base == "main" {
            let remote_ref = format!("origin/{}", base);
            let exists = ProcessCommand::new("git")
                .args(["rev-parse", "--verify", &remote_ref])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if exists {
                remote_ref
            } else {
                base.to_string()
            }
        } else {
            base.to_string()
        }
    } else {
        get_worktree_base_ref(branch)?
    };

    let branch_exists = ProcessCommand::new("git")
        .args(["rev-parse", "--verify", branch])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let output = if branch_exists {
        ProcessCommand::new("git")
            .args(["worktree", "add", path.to_str().unwrap(), branch])
            .output()
            .context("Failed to create worktree")?
    } else {
        println!(
            "  {} Creating branch '{}' from '{}'",
            "→".bright_blue(),
            branch,
            base_ref
        );
        ProcessCommand::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                branch,
                path.to_str().unwrap(),
                &base_ref,
            ])
            .output()
            .context("Failed to create worktree")?
    };

    if !output.status.success() {
        bail!(
            "Failed to create worktree: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

fn get_worktree_base_ref(branch: &str) -> Result<String> {
    let is_main_branch = branch == "master" || branch == "main";

    if is_main_branch {
        let remote_ref = format!("origin/{}", branch);
        let exists = ProcessCommand::new("git")
            .args(["rev-parse", "--verify", &remote_ref])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if exists {
            return Ok(remote_ref);
        }
        return Ok(branch.to_string());
    }

    let local_exists = ProcessCommand::new("git")
        .args(["rev-parse", "--verify", branch])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if local_exists {
        return Ok(branch.to_string());
    }

    let remote_ref = format!("origin/{}", branch);
    let remote_exists = ProcessCommand::new("git")
        .args(["rev-parse", "--verify", &remote_ref])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if remote_exists {
        return Ok(remote_ref);
    }

    for default_branch in &["origin/master", "origin/main"] {
        let exists = ProcessCommand::new("git")
            .args(["rev-parse", "--verify", default_branch])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if exists {
            return Ok(default_branch.to_string());
        }
    }

    Ok("HEAD".to_string())
}

pub fn create_hive_symlink(
    worktree: &std::path::Path,
    project_root: &std::path::Path,
) -> Result<()> {
    let hive_dir = project_root.join(".hive");
    let symlink_path = worktree.join(".hive");

    if symlink_path.exists() || symlink_path.is_symlink() {
        if symlink_path.is_dir() && !symlink_path.is_symlink() {
            fs::remove_dir_all(&symlink_path)?;
        } else {
            fs::remove_file(&symlink_path)?;
        }
    }

    std::os::unix::fs::symlink(&hive_dir, &symlink_path)
        .context("Failed to create .hive symlink")?;

    Ok(())
}
