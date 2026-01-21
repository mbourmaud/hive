use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use crate::config;
use crate::types::{DroneState, DroneStatus, Prd};

pub fn run(
    name: String,
    _prompt: Option<String>,
    resume: bool,
    local: bool,
    model: String,
    dry_run: bool,
) -> Result<()> {
    println!("{} Launching drone '{}'...", "→".bright_blue(), name.bright_cyan());

    // 1. Check if drone already exists
    let drone_dir = PathBuf::from(".hive/drones").join(&name);
    if drone_dir.exists() && !resume {
        bail!("Drone '{}' already exists. Use --resume to resume.", name);
    }

    // 2. Find PRD
    let prd_path = find_prd(&name)?;
    let prd = load_prd(&prd_path)?;
    println!("  {} Found PRD: {}", "✓".green(), prd.title);

    // 3. Determine worktree path
    let worktree_path = if local {
        std::env::current_dir()?
    } else {
        let worktree_base = config::get_worktree_base()?;
        let project_name = get_project_name()?;
        worktree_base.join(&project_name).join(&name)
    };

    println!("  {} Worktree: {}", "✓".green(), worktree_path.display());

    // 4. Create worktree if not local and not exists
    let default_branch = format!("hive/{}", name);
    let branch = prd.target_branch.as_deref().unwrap_or(&default_branch);

    if !local && !worktree_path.exists() {
        create_worktree(&worktree_path, branch)?;
        println!("  {} Created worktree", "✓".green());
    }

    // 5. Create .hive symlink in worktree
    if !local {
        create_hive_symlink(&worktree_path)?;
        println!("  {} Symlinked .hive", "✓".green());
    }

    // 6. Create drone status
    fs::create_dir_all(&drone_dir)?;
    let status = DroneStatus {
        drone: name.clone(),
        prd: prd_path.file_name().unwrap().to_string_lossy().to_string(),
        branch: branch.to_string(),
        worktree: worktree_path.to_string_lossy().to_string(),
        local_mode: local,
        status: DroneState::Starting,
        current_story: None,
        completed: Vec::new(),
        story_times: std::collections::HashMap::new(),
        total: prd.stories.len(),
        started: chrono::Utc::now().to_rfc3339(),
        updated: chrono::Utc::now().to_rfc3339(),
        error_count: 0,
        last_error_story: None,
        blocked_reason: None,
        blocked_questions: Vec::new(),
        awaiting_human: false,
    };

    let status_path = drone_dir.join("status.json");
    let status_json = serde_json::to_string_pretty(&status)?;
    fs::write(&status_path, status_json)?;
    println!("  {} Created status.json", "✓".green());

    // 7. Launch Claude
    if dry_run {
        println!("  {} Dry run - not launching Claude", "→".yellow());
    } else {
        launch_claude(&worktree_path, &model, &name)?;
        println!("  {} Launched Claude (model: {})", "✓".green(), model.bright_cyan());
    }

    // 8. Send notification
    send_notification(&name, "started")?;

    println!("\n{} Drone '{}' is running!", "✓".green().bold(), name.bright_cyan());
    println!("\nMonitor progress:");
    println!("  hive-rust status {}", name);
    println!("  hive-rust logs {}", name);

    Ok(())
}

fn find_prd(name: &str) -> Result<PathBuf> {
    let prds_dir = PathBuf::from(".hive/prds");

    if !prds_dir.exists() {
        bail!("No PRDs directory found. Run 'hive-rust init' first.");
    }

    // Try exact match first
    let exact = prds_dir.join(format!("prd-{}.json", name));
    if exact.exists() {
        return Ok(exact);
    }

    let exact2 = prds_dir.join(format!("{}.json", name));
    if exact2.exists() {
        return Ok(exact2);
    }

    // Search for any PRD
    for entry in fs::read_dir(&prds_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            return Ok(path);
        }
    }

    bail!("No PRD found for drone '{}'", name);
}

fn load_prd(path: &PathBuf) -> Result<Prd> {
    let contents = fs::read_to_string(path)
        .context("Failed to read PRD")?;
    let prd: Prd = serde_json::from_str(&contents)
        .context("Failed to parse PRD")?;
    Ok(prd)
}

fn get_project_name() -> Result<String> {
    std::env::current_dir()?
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .context("Failed to get directory name")
}

fn create_worktree(path: &std::path::Path, branch: &str) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Try to create branch if it doesn't exist
    let _ = ProcessCommand::new("git")
        .args(["branch", branch])
        .output();

    // Create worktree
    let output = ProcessCommand::new("git")
        .args(["worktree", "add", path.to_str().unwrap(), branch])
        .output()
        .context("Failed to create worktree")?;

    if !output.status.success() {
        bail!("Failed to create worktree: {}",
              String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

fn create_hive_symlink(worktree: &std::path::Path) -> Result<()> {
    let hive_dir = std::env::current_dir()?.join(".hive");
    let symlink_path = worktree.join(".hive");

    if symlink_path.exists() {
        fs::remove_file(&symlink_path)?;
    }

    std::os::unix::fs::symlink(&hive_dir, &symlink_path)
        .context("Failed to create .hive symlink")?;

    Ok(())
}

fn launch_claude(worktree: &PathBuf, model: &str, drone_name: &str) -> Result<()> {
    // Create log file
    let log_path = PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("activity.log");
    let log_file = fs::File::create(&log_path)?;

    // Launch claude in background
    ProcessCommand::new("claude")
        .arg("--model")
        .arg(model)
        .current_dir(worktree)
        .stdin(Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file)
        .spawn()
        .context("Failed to spawn claude process")?;

    Ok(())
}

fn send_notification(drone_name: &str, action: &str) -> Result<()> {
    let message = format!("Drone {} {}", drone_name, action);

    // Try terminal-notifier first (macOS)
    let _ = ProcessCommand::new("terminal-notifier")
        .args(["-title", "🐝 Hive", "-message", &message, "-sound", "Glass"])
        .output();

    // Fallback to osascript
    let _ = ProcessCommand::new("osascript")
        .arg("-e")
        .arg(format!(
            "display notification \"{}\" with title \"🐝 Hive\" sound name \"Glass\"",
            message
        ))
        .output();

    Ok(())
}
