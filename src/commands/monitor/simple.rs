use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use std::path::PathBuf;

use crate::agent_teams::task_sync;
use crate::commands::common::{
    duration_between, elapsed_since, format_duration, is_process_running, list_drones, load_prd,
    parse_timestamp, read_drone_pid, reconcile_progress, truncate_with_ellipsis,
    DEFAULT_INACTIVE_THRESHOLD_SECS, FULL_PROGRESS_BAR_WIDTH, MAX_STORY_TITLE_LEN,
};
use crate::types::{DroneState, DroneStatus};

/// Refresh interval for follow mode in seconds
const FOLLOW_REFRESH_SECS: u64 = 30;

/// ANSI escape sequence to clear screen and move cursor to top-left
const CLEAR_SCREEN: &str = "\x1B[2J\x1B[1;1H";

pub(crate) fn run_simple(name: Option<String>, follow: bool) -> Result<()> {
    loop {
        if follow {
            print!("{}", CLEAR_SCREEN);
        }

        let drones = list_drones()?;

        if drones.is_empty() {
            println!("{}", "No drones found".yellow());
            println!("\nRun 'hive init' to initialize Hive");
            return Ok(());
        }

        let filtered: Vec<_> = match name {
            Some(ref n) => drones
                .into_iter()
                .filter(|(drone_name, _)| drone_name == n)
                .collect(),
            None => drones,
        };

        if filtered.is_empty() {
            eprintln!("Drone '{}' not found", name.unwrap());
            return Ok(());
        }

        println!(
            "  {} v{}",
            "👑 hive".yellow().bold(),
            env!("CARGO_PKG_VERSION")
        );
        println!();

        let mut sorted = filtered;
        sorted.sort_by_key(|(_, status)| match status.status {
            DroneState::Completed => 1,
            _ => 0,
        });

        for (drone_name, status) in &sorted {
            let collapsed = status.status == DroneState::Completed;
            print_drone_status(drone_name, status, collapsed);
            println!();
        }

        if !follow {
            suggest_cleanup_for_inactive(&sorted);
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(FOLLOW_REFRESH_SECS));
    }

    Ok(())
}

pub(crate) fn suggest_cleanup_for_inactive(drones: &[(String, DroneStatus)]) {
    let threshold_seconds = std::env::var("HIVE_INACTIVE_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(DEFAULT_INACTIVE_THRESHOLD_SECS);

    let now = Utc::now();

    for (name, status) in drones {
        if status.status != DroneState::Completed {
            continue;
        }

        let Some(updated) = parse_timestamp(&status.updated) else {
            continue;
        };

        let inactive_seconds = now.signed_duration_since(updated).num_seconds();

        if inactive_seconds > threshold_seconds {
            let duration = chrono::Duration::seconds(inactive_seconds);
            let duration_str = format_duration(duration);

            println!();
            println!(
                "{} Drone {} completed {} ago. Clean up? {}",
                "💡".bright_yellow(),
                name.bright_cyan(),
                duration_str.bright_black(),
                format!("(hive clean {})", name).bright_black()
            );

            break;
        }
    }
}

pub(crate) fn print_drone_status(name: &str, status: &DroneStatus, collapsed: bool) {
    // Check if process is actually running
    let process_running = read_drone_pid(name)
        .map(is_process_running)
        .unwrap_or(false);

    // Determine status symbol based on actual state and process status
    // ◐ = half-full (in progress), ● = full (completed), ○ = empty (pending)
    let status_symbol = match status.status {
        DroneState::Starting => "◐".yellow(),
        DroneState::Resuming => "◐".yellow(),
        DroneState::InProgress => {
            if process_running {
                "◐".green() // Half-full green = in progress
            } else {
                "○".yellow() // Empty yellow = stalled
            }
        }
        DroneState::Completed => "●".bright_green().bold(), // Full green = completed
        DroneState::Error => "◐".red().bold(),              // Half-full red = error
        DroneState::Blocked => "◐".red().bold(),            // Half-full red = blocked
        DroneState::Stopped => "○".bright_black(),
        DroneState::Cleaning => "🧹".normal(),
    };

    // Calculate total elapsed time
    let elapsed = elapsed_since(&status.started)
        .map(|e| format!("  {}", e))
        .unwrap_or_default();

    let mode_emoji = "🐝";

    // Reconcile progress with actual PRD (filters out old completed stories)
    let (valid_completed, total_stories) = reconcile_progress(status);

    // If collapsed view (completed drones), show single line
    if collapsed {
        let progress = if total_stories > 0 {
            format!("{}/{}", valid_completed, total_stories)
        } else {
            "Planning...".to_string()
        };

        println!(
            "  {} {}{}  {}",
            status_symbol,
            format!("{} {}", mode_emoji, name).bright_black(),
            elapsed.bright_black(),
            progress.bright_black()
        );
        return; // Exit early, don't show full details
    }

    // Full view for active drones
    println!(
        "  {} {}{}  {}",
        status_symbol,
        format!("{} {}", mode_emoji, name).yellow().bold(),
        elapsed.bright_black(),
        format!("[{}]", status.status).bright_black()
    );

    // Print progress using reconciled values
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

    println!("  Progress: {} ({}%)", progress.bright_white(), percentage);

    let filled = (FULL_PROGRESS_BAR_WIDTH as f32 * percentage as f32 / 100.0) as usize;
    let empty = FULL_PROGRESS_BAR_WIDTH - filled;
    let bar = format!(
        "[{}{}]",
        "━".repeat(filled).green(),
        "─".repeat(empty).bright_black()
    );
    println!("  {}", bar);

    // Show active agents
    {
        if let Ok(task_states) = task_sync::read_team_task_states(name) {
            let active: Vec<_> = task_states
                .values()
                .filter(|t| t.status == "in_progress" && t.owner.is_some())
                .collect();
            if !active.is_empty() {
                println!(
                    "\n  🐝 Active agents ({}):",
                    active.len().to_string().bright_white()
                );
                for task in &active {
                    let agent = task.owner.as_deref().unwrap_or("?");
                    let story = task.story_id.as_deref().unwrap_or(&task.id);
                    let form = task.active_form.as_deref().unwrap_or(&task.subject);
                    let model_str = task
                        .model
                        .as_deref()
                        .map(|m| format!(" [{}]", m))
                        .unwrap_or_default();
                    println!(
                        "    {} {}{}: {} ({})",
                        "→".bright_blue(),
                        agent.bright_cyan(),
                        model_str.bright_magenta(),
                        form,
                        story.bright_yellow()
                    );
                }
            }

            // Also show team members count
            if let Ok(members) = task_sync::read_team_members(name) {
                if !members.is_empty() {
                    println!(
                        "  👥 {} teammates spawned",
                        members.len().to_string().bright_white()
                    );
                }
            }
        }
    }

    // Load PRD to get story titles
    let prd_path = PathBuf::from(".hive").join("prds").join(&status.prd);
    if let Some(prd) = load_prd(&prd_path) {
        println!("\n  Stories:");

        // Build maps of story_id -> agent_name and story_id -> model from active tasks
        let task_states = task_sync::read_team_task_states(name).unwrap_or_default();
        let agent_map: std::collections::HashMap<String, String> = task_states
            .values()
            .filter_map(|t| {
                let story_id = t.story_id.clone().unwrap_or_else(|| t.id.clone());
                t.owner.clone().map(|owner| (story_id, owner))
            })
            .collect();
        let model_map: std::collections::HashMap<String, String> = task_states
            .values()
            .filter_map(|t| {
                let story_id = t.story_id.clone().unwrap_or_else(|| t.id.clone());
                t.model.clone().map(|model| (story_id, model))
            })
            .collect();

        for story in &prd.stories {
            let is_completed = status.completed.contains(&story.id);
            let is_current = status.current_story.as_ref() == Some(&story.id);
            let is_active = status.active_agents.values().any(|s| s == &story.id)
                || agent_map.contains_key(&story.id);

            // ◐ = half-full (in progress), ● = full (completed), ○ = empty (pending)
            let (icon, color_fn): (_, fn(String) -> colored::ColoredString) = if is_completed {
                ("●", |s| s.green()) // Full green = completed
            } else if is_current || is_active {
                ("◐", |s| s.yellow()) // Half-full yellow = in progress
            } else {
                ("○", |s| s.bright_black()) // Empty = pending
            };

            // Calculate duration
            let duration_str = if let Some(timing) = status.story_times.get(&story.id) {
                if let (Some(started), Some(completed)) = (&timing.started, &timing.completed) {
                    // Completed story - show duration
                    if let Some(dur) = duration_between(started, completed) {
                        format!(" ({})", format_duration(dur))
                    } else {
                        String::new()
                    }
                } else if let Some(started) = &timing.started {
                    // In-progress story - show elapsed time
                    if let Some(elapsed) = elapsed_since(started) {
                        format!(" ({})", elapsed)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // Show agent name and model for active stories in Agent Teams mode
            let agent_suffix = if is_active || is_current {
                agent_map
                    .get(&story.id)
                    .map(|a| {
                        let model_str = model_map
                            .get(&story.id)
                            .map(|m| format!(" {}", m))
                            .unwrap_or_default();
                        format!(" [@{}{}]", a, model_str)
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let title_display = truncate_with_ellipsis(&story.title, MAX_STORY_TITLE_LEN);
            let story_line = format!(
                "    {} {} {}{}{}",
                icon, story.id, title_display, duration_str, agent_suffix
            );
            println!("{}", color_fn(story_line));
        }
    } else {
        // Fallback: just show current story if PRD not loaded
        if let Some(ref story) = status.current_story {
            println!("  Current: {}", story.bright_yellow());
        }
    }

    // Print blocked reason
    if status.status == DroneState::Blocked {
        if let Some(ref reason) = status.blocked_reason {
            println!("  {} {}", "Blocked:".red().bold(), reason.red());
        }
    }

    // Print error info
    if status.status == DroneState::Error {
        println!("  {} {} errors", "Errors:".red().bold(), status.error_count);
        if let Some(ref last_error_story) = status.last_error_story {
            println!("  Last error in: {}", last_error_story.red());
        }
    }

    // Print metadata
    println!("  Branch: {}", status.branch.bright_black());
    println!("  PRD: {}", status.prd.bright_black());
}
