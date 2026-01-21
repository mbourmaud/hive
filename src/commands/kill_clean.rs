use anyhow::{bail, Context, Result};
use colored::Colorize;
use dialoguer::Confirm;
use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use crate::types::DroneStatus;

/// Stop a drone by name. If `quiet` is true, no output is printed (for TUI use).
pub fn kill(name: String) -> Result<()> {
    kill_impl(name, false)
}

/// Stop a drone quietly (no stdout output) - for use from TUI
pub fn kill_quiet(name: String) -> Result<()> {
    kill_impl(name, true)
}

fn kill_impl(name: String, quiet: bool) -> Result<()> {
    let drone_dir = PathBuf::from(".hive/drones").join(&name);

    if !drone_dir.exists() {
        bail!("Drone '{}' not found", name);
    }

    if !quiet {
        println!(
            "{} Stopping drone '{}'...",
            "→".bright_blue(),
            name.bright_cyan()
        );
    }

    // Find Claude process
    let ps_output = ProcessCommand::new("ps")
        .args(["aux"])
        .output()
        .context("Failed to run ps command")?;

    let ps_str = String::from_utf8_lossy(&ps_output.stdout);

    // Look for claude process in worktree directory
    let status_path = drone_dir.join("status.json");
    let status: DroneStatus = if status_path.exists() {
        let contents = fs::read_to_string(&status_path)?;
        serde_json::from_str(&contents)?
    } else {
        bail!("No status file found for drone '{}'", name);
    };

    let worktree_path = &status.worktree;

    // Find PIDs matching the worktree path
    let mut pids = Vec::new();
    for line in ps_str.lines() {
        if line.contains("claude") && line.contains(worktree_path) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                if let Ok(pid) = parts[1].parse::<i32>() {
                    pids.push(pid);
                }
            }
        }
    }

    if pids.is_empty() {
        if !quiet {
            println!("  {} No running process found", "→".yellow());
        }
    } else {
        for pid in &pids {
            // Try SIGTERM first
            let _ = ProcessCommand::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output();

            if !quiet {
                println!("  {} Sent SIGTERM to PID {}", "✓".green(), pid);
            }

            // Wait a bit
            std::thread::sleep(std::time::Duration::from_secs(2));

            // Check if still running
            let still_running = ProcessCommand::new("ps")
                .args(["-p", &pid.to_string()])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if still_running {
                // Force kill with SIGKILL
                let _ = ProcessCommand::new("kill")
                    .args(["-KILL", &pid.to_string()])
                    .output();

                if !quiet {
                    println!("  {} Sent SIGKILL to PID {}", "✓".green(), pid);
                }
            }
        }
    }

    // Send notification
    send_notification(&name, "stopped")?;

    if !quiet {
        println!(
            "\n{} Drone '{}' stopped",
            "✓".green().bold(),
            name.bright_cyan()
        );
    }

    Ok(())
}

pub fn clean(name: String, force: bool) -> Result<()> {
    let drone_dir = PathBuf::from(".hive/drones").join(&name);

    if !drone_dir.exists() {
        bail!("Drone '{}' not found", name);
    }

    // Load status to get worktree path and branch
    let status_path = drone_dir.join("status.json");
    let status: DroneStatus = if status_path.exists() {
        let contents = fs::read_to_string(&status_path)?;
        serde_json::from_str(&contents)?
    } else {
        bail!("No status file found for drone '{}'", name);
    };

    // Check if drone is stopped
    let ps_output = ProcessCommand::new("ps")
        .args(["aux"])
        .output()
        .context("Failed to run ps command")?;

    let ps_str = String::from_utf8_lossy(&ps_output.stdout);
    let is_running = ps_str
        .lines()
        .any(|line| line.contains("claude") && line.contains(&status.worktree));

    if is_running {
        bail!(
            "Drone '{}' is still running. Stop it first with 'hive-rust kill {}'",
            name,
            name
        );
    }

    // Confirm cleanup
    if !force {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Clean up drone '{}'? This will remove the worktree and all drone data.",
                name
            ))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Cancelled");
            return Ok(());
        }
    }

    println!(
        "{} Cleaning up drone '{}'...",
        "→".bright_blue(),
        name.bright_cyan()
    );

    // Remove worktree if not in local mode
    if !status.local_mode {
        let worktree_path = PathBuf::from(&status.worktree);

        if worktree_path.exists() {
            // Remove git worktree
            let output = ProcessCommand::new("git")
                .args([
                    "worktree",
                    "remove",
                    "--force",
                    worktree_path.to_str().unwrap(),
                ])
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    println!("  {} Removed worktree", "✓".green());
                }
            }
        }

        // Delete branch
        let _ = ProcessCommand::new("git")
            .args(["branch", "-D", &status.branch])
            .output();

        println!("  {} Deleted branch {}", "✓".green(), status.branch);
    }

    // Remove drone directory
    fs::remove_dir_all(&drone_dir).context("Failed to remove drone directory")?;
    println!("  {} Removed drone state", "✓".green());

    println!(
        "\n{} Drone '{}' cleaned up",
        "✓".green().bold(),
        name.bright_cyan()
    );

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
