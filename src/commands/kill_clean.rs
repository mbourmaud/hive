use anyhow::{bail, Context, Result};
use chrono::Utc;
use colored::Colorize;
use dialoguer::Confirm;
use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use crate::backend::{self, SpawnHandle};
use crate::notification;
use crate::types::{DroneState, DroneStatus};

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

    // Load drone status to get worktree path
    let status_path = drone_dir.join("status.json");
    let status: DroneStatus = if status_path.exists() {
        let contents = fs::read_to_string(&status_path)?;
        serde_json::from_str(&contents)?
    } else {
        bail!("No status file found for drone '{}'", name);
    };

    // Persist progress from events.ndjson before stopping
    {
        let (completed, total) = crate::events::reconstruct_progress(&name);
        let mut updated_status = status.clone();
        updated_status.status = DroneState::Stopped;
        updated_status.updated = Utc::now().to_rfc3339();
        if total > 0 {
            updated_status.total = total;
            updated_status.completed.truncate(completed);
            while updated_status.completed.len() < completed {
                updated_status
                    .completed
                    .push(format!("task-{}", updated_status.completed.len() + 1));
            }
        }
        let _ = fs::write(&status_path, serde_json::to_string_pretty(&updated_status)?);
    }

    // Use the backend to stop the drone process
    let backend = backend::resolve_agent_team_backend();
    let handle = SpawnHandle {
        pid: None,
        backend_id: status.worktree.clone(),
        backend_type: status.backend.clone(),
    };

    if !quiet {
        println!(
            "  {} Stopping via {} backend...",
            "→".bright_blue(),
            status.backend
        );
    }

    backend.stop(&handle)?;

    if !quiet {
        println!("  {} Process stopped", "✓".green());
    }

    // Send notification
    notification::notify("Hive", &format!("Drone '{}' stopped", name));

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
    clean_impl(name, force, false)
}

/// Clean a drone quietly (no stdout output) - for use from TUI
pub fn clean_quiet(name: String) -> Result<()> {
    clean_impl(name, true, true)
}

/// Clean in background thread (returns immediately, cleans async).
/// Sets drone status to "cleaning" so the TUI can show it.
pub fn clean_background(name: String) {
    // Mark as "cleaning" in status.json before background thread starts
    let status_path = PathBuf::from(".hive/drones")
        .join(&name)
        .join("status.json");
    if let Ok(contents) = fs::read_to_string(&status_path) {
        if let Ok(mut status) = serde_json::from_str::<DroneStatus>(&contents) {
            status.status = crate::types::DroneState::Cleaning;
            status.updated = Utc::now().to_rfc3339();
            if let Ok(json) = serde_json::to_string_pretty(&status) {
                let _ = fs::write(&status_path, json);
            }
        }
    }

    std::thread::spawn(move || {
        let _ = clean_impl(name, true, true);
    });
}

fn clean_impl(name: String, force: bool, quiet: bool) -> Result<()> {
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
        if quiet {
            // In quiet/background mode (TUI), auto-stop before cleaning
            let backend = backend::resolve_agent_team_backend();
            let handle = SpawnHandle {
                pid: None,
                backend_id: status.worktree.clone(),
                backend_type: status.backend.clone(),
            };
            let _ = backend.stop(&handle);
        } else {
            bail!(
                "Drone '{}' is still running. Stop it first with 'hive stop {}'",
                name,
                name
            );
        }
    }

    // Confirm cleanup (only in interactive mode)
    if !force && !quiet {
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

    if !quiet {
        println!(
            "{} Cleaning up drone '{}'...",
            "→".bright_blue(),
            name.bright_cyan()
        );
    }

    // Remove drone directory first (so it disappears from list immediately)
    fs::remove_dir_all(&drone_dir).context("Failed to remove drone directory")?;
    if !quiet {
        println!("  {} Removed drone state", "✓".green());
    }

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

            if !quiet {
                if let Ok(out) = output {
                    if out.status.success() {
                        println!("  {} Removed worktree", "✓".green());
                    }
                }
            }
        }

        // Delete branch
        let _ = ProcessCommand::new("git")
            .args(["branch", "-D", &status.branch])
            .output();

        if !quiet {
            println!("  {} Deleted branch {}", "✓".green(), status.branch);
        }
    }

    // Clean up Agent Teams directories
    if let Err(e) = crate::agent_teams::cleanup_team(&name) {
        if !quiet {
            println!("  {} Failed to clean Agent Teams dirs: {}", "⚠".yellow(), e);
        }
    } else if !quiet {
        println!("  {} Cleaned Agent Teams directories", "✓".green());
    }

    if !quiet {
        println!(
            "\n{} Drone '{}' cleaned up",
            "✓".green().bold(),
            name.bright_cyan()
        );
    }

    Ok(())
}
