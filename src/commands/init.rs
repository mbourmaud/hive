use anyhow::{bail, Context, Result};
use colored::Colorize;
use dialoguer::Confirm;
use std::path::PathBuf;

use crate::config;
use crate::types::HiveConfig;

pub fn run() -> Result<()> {
    // 1. Verify we're in a git repository
    if !is_git_repo()? {
        bail!("Not a git repository. Please run 'git init' first.");
    }

    println!("{}", "Initializing Hive...".bright_blue());

    // 2. Create .hive directory structure
    let hive_dir = PathBuf::from(".hive");
    let prds_dir = hive_dir.join("prds");
    let drones_dir = hive_dir.join("drones");

    std::fs::create_dir_all(&prds_dir)
        .context("Failed to create .hive/prds directory")?;
    std::fs::create_dir_all(&drones_dir)
        .context("Failed to create .hive/drones directory")?;

    println!("  {} Created .hive directory structure", "✓".green());

    // 3. Get project name from git remote or directory name
    let project_name = get_project_name()?;

    // 4. Create .hive/config.json
    let config_path = hive_dir.join("config.json");
    if !config_path.exists() {
        let config = HiveConfig {
            project: Some(project_name.clone()),
            ..Default::default()
        };
        config::save_local_config(&config)
            .context("Failed to save config")?;
        println!("  {} Created .hive/config.json", "✓".green());
    } else {
        println!("  {} .hive/config.json already exists", "→".yellow());
    }

    // 5. Add .hive/ to .gitignore
    add_to_gitignore(".hive/")?;
    println!("  {} Updated .gitignore", "✓".green());

    // 6. First-time global setup
    let skip_prompts = std::env::var("HIVE_SKIP_PROMPTS").is_ok();

    if config::load_global_config().is_err() {
        println!("\n{}", "First-time setup".bright_yellow().bold());

        let worktree_base = if skip_prompts {
            // Use default for tests
            let home = dirs::home_dir().context("Failed to get home directory")?;
            home.join(".hive").join("worktrees")
        } else if Confirm::new()
            .with_prompt("Use default worktree location (~/.hive/worktrees)?")
            .default(true)
            .interact()?
        {
            let home = dirs::home_dir().context("Failed to get home directory")?;
            home.join(".hive").join("worktrees")
        } else {
            let input = dialoguer::Input::<String>::new()
                .with_prompt("Enter custom worktree base directory")
                .interact_text()?;
            PathBuf::from(input)
        };

        let global_config = HiveConfig {
            worktree_base: Some(worktree_base.to_string_lossy().to_string()),
            ..Default::default()
        };

        config::save_global_config(&global_config)
            .context("Failed to save global config")?;

        println!("  {} Created global config at ~/.config/hive/config.json", "✓".green());
    }

    println!("\n{} Hive initialized successfully for project '{}'",
             "✓".green().bold(),
             project_name.bright_cyan());
    println!("\nNext steps:");
    println!("  1. Create a PRD file in .hive/prds/");
    println!("  2. Run 'hive-rust start <drone-name>' to launch a drone");

    Ok(())
}

fn is_git_repo() -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .context("Failed to execute git command")?;
    Ok(output.status.success())
}

fn get_project_name() -> Result<String> {
    // Try to get from git remote
    if let Ok(output) = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
    {
        if output.status.success() {
            let url = String::from_utf8_lossy(&output.stdout);
            if let Some(name) = extract_repo_name(&url) {
                return Ok(name);
            }
        }
    }

    // Fallback to directory name
    std::env::current_dir()?
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .context("Failed to get directory name")
}

fn extract_repo_name(url: &str) -> Option<String> {
    let url = url.trim();
    let parts: Vec<&str> = url.split('/').collect();
    parts.last()
        .map(|s| s.trim_end_matches(".git").to_string())
}

fn add_to_gitignore(pattern: &str) -> Result<()> {
    let gitignore_path = PathBuf::from(".gitignore");

    // Read existing content
    let content = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path)
            .context("Failed to read .gitignore")?
    } else {
        String::new()
    };

    // Check if pattern already exists
    if content.lines().any(|line| line.trim() == pattern) {
        return Ok(());
    }

    // Append pattern
    let new_content = if content.is_empty() || content.ends_with('\n') {
        format!("{}{}\n", content, pattern)
    } else {
        format!("{}\n{}\n", content, pattern)
    };

    std::fs::write(&gitignore_path, new_content)
        .context("Failed to write .gitignore")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo.git"),
            Some("repo".to_string())
        );
        assert_eq!(
            extract_repo_name("git@github.com:user/repo.git"),
            Some("repo".to_string())
        );
        assert_eq!(
            extract_repo_name("https://github.com/user/repo"),
            Some("repo".to_string())
        );
    }

    #[test]
    fn test_is_git_repo() {
        // This will work if running in hive repo
        let is_git = is_git_repo().unwrap();
        assert!(is_git);
    }
}
