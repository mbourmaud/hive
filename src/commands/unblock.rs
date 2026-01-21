use anyhow::{bail, Context, Result};
use colored::Colorize;
use dialoguer::{Confirm, Select};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::types::{DroneState, DroneStatus, Prd};

pub fn run(name: String, no_interactive: bool) -> Result<()> {
    // Load drone status
    let status_path = PathBuf::from(".hive")
        .join("drones")
        .join(&name)
        .join("status.json");

    if !status_path.exists() {
        bail!("Drone '{}' not found", name);
    }

    let contents = fs::read_to_string(&status_path).context("Failed to read drone status")?;
    let mut status: DroneStatus =
        serde_json::from_str(&contents).context("Failed to parse drone status")?;

    // Check if drone is actually blocked
    if status.status != DroneState::Blocked {
        println!(
            "{}",
            format!(
                "⚠ Drone '{}' is not in blocked state (current: {})",
                name, status.status
            )
            .yellow()
        );
        if !no_interactive {
            let proceed = Confirm::new()
                .with_prompt("Do you want to proceed anyway?")
                .default(false)
                .interact()?;
            if !proceed {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    }

    // Display blocked information
    println!(
        "{}",
        format!("🐝 Unblocking Drone: {}", name)
            .bright_cyan()
            .bold()
    );
    println!();

    if let Some(ref reason) = status.blocked_reason {
        println!("{}", "Blocked Reason:".red().bold());
        println!("{}", reason);
        println!();
    }

    if !status.blocked_questions.is_empty() {
        println!("{}", "Blocked Questions:".red().bold());
        for (i, question) in status.blocked_questions.iter().enumerate() {
            println!("  {}. {}", i + 1, question);
        }
        println!();
    }

    // Try to load blocked.md if it exists
    let blocked_md_path = PathBuf::from(".hive")
        .join("drones")
        .join(&name)
        .join("blocked.md");

    if blocked_md_path.exists() {
        println!("{}", "Additional Context (blocked.md):".yellow().bold());
        println!();
        let blocked_content =
            fs::read_to_string(&blocked_md_path).context("Failed to read blocked.md")?;
        println!("{}", blocked_content);
        println!();
    }

    // Load PRD for context
    let prd_path = PathBuf::from(".hive").join("prds").join(&status.prd);

    let prd = if prd_path.exists() {
        let prd_contents = fs::read_to_string(&prd_path).context("Failed to read PRD")?;
        Some(serde_json::from_str::<Prd>(&prd_contents).context("Failed to parse PRD")?)
    } else {
        None
    };

    // Show current story context
    if let Some(ref story_id) = status.current_story {
        if let Some(ref prd) = prd {
            if let Some(story) = prd.stories.iter().find(|s| s.id == *story_id) {
                println!(
                    "{}",
                    format!("Current Story: {}", story_id)
                        .bright_yellow()
                        .bold()
                );
                println!("  Title: {}", story.title);
                println!("  Description: {}", story.description);
                println!();
            }
        }
    }

    // Show stats
    println!("{}", "Drone Statistics:".bright_black());
    println!("  Progress: {}/{}", status.completed.len(), status.total);
    println!("  Error count: {}", status.error_count);
    if let Some(ref last_error) = status.last_error_story {
        println!("  Last error story: {}", last_error);
    }
    println!();

    // Interactive mode
    if !no_interactive {
        let options = vec![
            "View/Edit PRD",
            "Clear blocked status and resume",
            "Clear blocked status without resuming",
            "Cancel (do nothing)",
        ];

        let selection = Select::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                // Edit PRD
                println!("\n{}", "Opening PRD in editor...".bright_green());
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                let mut child = Command::new(&editor)
                    .arg(&prd_path)
                    .spawn()
                    .context(format!("Failed to open editor '{}'", editor))?;
                child.wait()?;
                println!("{}", "✓ PRD editor closed".green());

                // Ask if they want to resume
                let resume = Confirm::new()
                    .with_prompt("Clear blocked status and resume drone?")
                    .default(true)
                    .interact()?;

                if resume {
                    clear_blocked_status(&mut status)?;
                    save_status(&name, &status)?;
                    println!(
                        "{}",
                        format!(
                            "✓ Drone '{}' unblocked. Use 'hive-rust start {} --resume' to resume.",
                            name, name
                        )
                        .green()
                    );
                }
            }
            1 => {
                // Clear and resume
                clear_blocked_status(&mut status)?;
                save_status(&name, &status)?;
                println!(
                    "{}",
                    format!(
                        "✓ Drone '{}' unblocked. Use 'hive-rust start {} --resume' to resume.",
                        name, name
                    )
                    .green()
                );
            }
            2 => {
                // Clear without resume
                clear_blocked_status(&mut status)?;
                save_status(&name, &status)?;
                println!(
                    "{}",
                    format!("✓ Drone '{}' blocked status cleared.", name).green()
                );
            }
            3 => {
                // Cancel
                println!("{}", "Cancelled.".yellow());
                return Ok(());
            }
            _ => unreachable!(),
        }
    } else {
        // Non-interactive mode: just clear blocked status
        clear_blocked_status(&mut status)?;
        save_status(&name, &status)?;
        println!(
            "{}",
            format!("✓ Drone '{}' blocked status cleared.", name).green()
        );
    }

    Ok(())
}

fn clear_blocked_status(status: &mut DroneStatus) -> Result<()> {
    status.status = DroneState::InProgress;
    status.blocked_reason = None;
    status.blocked_questions.clear();
    status.error_count = 0;
    status.awaiting_human = false;
    status.updated = chrono::Utc::now().to_rfc3339();
    Ok(())
}

fn save_status(name: &str, status: &DroneStatus) -> Result<()> {
    let status_path = PathBuf::from(".hive")
        .join("drones")
        .join(name)
        .join("status.json");

    let contents = serde_json::to_string_pretty(status).context("Failed to serialize status")?;

    fs::write(&status_path, contents).context("Failed to write status")?;

    Ok(())
}
