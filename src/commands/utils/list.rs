use anyhow::Result;
use colored::Colorize;

use super::super::common::agent_teams_progress;
use super::super::common::cost::parse_cost_from_log;
use crate::types::{DroneState, DroneStatus};

/// List all drones with compact output
pub fn list() -> Result<()> {
    let drones = list_drones()?;

    if drones.is_empty() {
        println!("{}", "No drones found".yellow());
        println!("\nRun 'hive init' to initialize Hive");
        return Ok(());
    }

    // Honey theme header with crown emoji
    println!("{}", "👑 Hive Drones".yellow().bold());
    println!();

    // Header
    println!(
        "{:<20} {:<15} {:<15} {:<10}",
        "NAME".bright_black(),
        "STATUS".bright_black(),
        "PROGRESS".bright_black(),
        "COST".bright_black(),
    );
    println!("{}", "─".repeat(65).bright_black());

    for (name, status) in drones {
        print_drone_row(&name, &status);
    }

    Ok(())
}

fn print_drone_row(name: &str, status: &DroneStatus) {
    let status_str = match status.status {
        DroneState::Starting => "starting".yellow(),
        DroneState::Resuming => "resuming".yellow(),
        DroneState::InProgress => "in_progress".green(),
        DroneState::Completed => "completed".bright_green().bold(),
        DroneState::Error => "error".red().bold(),
        DroneState::Stopped => "stopped".bright_black(),
        DroneState::Cleaning => "cleaning".bright_black(),
        DroneState::Zombie => "zombie".magenta(),
    };

    // Get progress from Agent Teams task list (read-only, no write-back)
    let (valid_completed, total_stories) = agent_teams_progress(&status.drone);

    let progress = if total_stories > 0 {
        format!("{}/{}", valid_completed, total_stories)
    } else {
        "Planning...".to_string()
    };

    let percentage = if total_stories > 0 {
        (valid_completed as f32 / total_stories as f32 * 100.0) as u32
    } else {
        0
    };

    let cost = parse_cost_from_log(name);
    let cost_str = if cost.total_cost_usd > 0.0 {
        format!("${:.2}", cost.total_cost_usd)
    } else {
        "-".to_string()
    };

    let mode_emoji = "\u{1f41d}";
    let model_badge = status
        .lead_model
        .as_ref()
        .map(|m| format!(" [{}]", m))
        .unwrap_or_default();

    println!(
        "{:<20} {:<15} {:<15} {:<10}",
        format!("{} {}", mode_emoji, name).yellow().bold(),
        format!("{}{}", status_str, model_badge.bright_magenta()),
        format!("{} ({}%)", progress, percentage).bright_white(),
        cost_str.bright_black(),
    );
}

fn list_drones() -> Result<Vec<(String, DroneStatus)>> {
    super::super::common::list_drones()
}
