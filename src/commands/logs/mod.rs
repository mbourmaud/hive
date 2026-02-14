mod format;

use anyhow::{bail, Result};
use colored::Colorize;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::agent_teams::task_sync;

use format::{extract_message_content, extract_task_display, extract_time_display};

pub fn run(name: String, lines: Option<usize>, follow: bool) -> Result<()> {
    let drone_dir = PathBuf::from(".hive/drones").join(&name);

    if !drone_dir.exists() {
        bail!("Drone '{}' not found", name);
    }

    show_team_conversation(&name, lines, follow)
}

/// Show the team conversation: inbox messages, task state changes, agent activity
fn show_team_conversation(team_name: &str, lines: Option<usize>, follow: bool) -> Result<()> {
    loop {
        if follow {
            print!("\x1B[2J\x1B[1;1H"); // Clear screen
        }

        println!("  🐝 {} Team Conversation", team_name.bright_cyan().bold());
        println!();

        print_team_roster(team_name);
        print_task_states(team_name);
        print_inbox_messages(team_name, lines);

        if !follow {
            println!("  {}", "Use -f/--follow for live updates".bright_black());
            break;
        }

        thread::sleep(Duration::from_secs(3));
    }

    Ok(())
}

fn print_team_roster(team_name: &str) {
    match task_sync::read_team_members(team_name) {
        Ok(members) if !members.is_empty() => {
            println!(
                "  👥 {} ({} agents)",
                "Team Roster".bright_white().bold(),
                members.len()
            );
            for member in &members {
                let model_short = if member.model.contains("opus") {
                    "opus".bright_magenta()
                } else if member.model.contains("sonnet") {
                    "sonnet".bright_blue()
                } else {
                    member.model.as_str().into()
                };
                println!(
                    "    {} {} ({})",
                    "→".bright_blue(),
                    member.name.bright_cyan(),
                    model_short
                );
            }
            println!();
        }
        _ => {}
    }
}

fn print_task_states(team_name: &str) {
    let fs_tasks = crate::agent_teams::read_task_list_safe(team_name);
    let tasks: std::collections::HashMap<_, _> = fs_tasks
        .into_iter()
        .map(crate::agent_teams::snapshot::map_task)
        .filter(|t| !t.is_internal)
        .map(|t| (t.id.clone(), t))
        .collect();

    if tasks.is_empty() {
        println!("  {} No tasks found", "○".dimmed());
        println!();
        return;
    }

    let mut in_progress: Vec<_> = tasks
        .values()
        .filter(|t| t.status == "in_progress")
        .collect();
    let mut completed: Vec<_> = tasks.values().filter(|t| t.status == "completed").collect();
    let mut pending: Vec<_> = tasks.values().filter(|t| t.status == "pending").collect();

    in_progress.sort_by_key(|t| &t.id);
    completed.sort_by_key(|t| &t.id);
    pending.sort_by_key(|t| &t.id);

    if !in_progress.is_empty() {
        println!(
            "  {} {}",
            "◐".yellow(),
            "Active Tasks".bright_yellow().bold()
        );
        for task in &in_progress {
            let (title, agent) = extract_task_display(task);
            let agent_str = agent
                .map(|a| format!(" @{}", a.bright_cyan().bold()))
                .unwrap_or_default();
            println!(
                "    {} {}{}",
                "◐".yellow(),
                title.bright_yellow(),
                agent_str
            );
        }
        println!();
    }

    if !completed.is_empty() {
        println!(
            "  {} {} ({})",
            "●".green(),
            "Completed".green().bold(),
            completed.len()
        );
        for task in &completed {
            let (title, agent) = extract_task_display(task);
            let agent_str = agent
                .map(|a| format!(" @{}", a.bright_cyan().bold()))
                .unwrap_or_default();
            println!("    {} {}{}", "●".green(), title.green(), agent_str);
        }
        println!();
    }

    if !pending.is_empty() {
        println!(
            "  {} {} ({})",
            "○".bright_black(),
            "Pending".bright_black().bold(),
            pending.len()
        );
        for task in &pending {
            let (title, _) = extract_task_display(task);
            println!("    {} {}", "○".bright_black(), title.bright_black());
        }
        println!();
    }
}

fn print_inbox_messages(team_name: &str, lines: Option<usize>) {
    match task_sync::read_team_inboxes(team_name) {
        Ok(inboxes) if !inboxes.is_empty() => {
            let mut all_messages: Vec<(String, &task_sync::InboxMessage)> = Vec::new();
            for (recipient, messages) in &inboxes {
                for msg in messages {
                    all_messages.push((recipient.clone(), msg));
                }
            }
            all_messages.sort_by(|a, b| a.1.timestamp.cmp(&b.1.timestamp));

            let messages_to_show = if let Some(n) = lines {
                if all_messages.len() > n {
                    &all_messages[all_messages.len() - n..]
                } else {
                    &all_messages
                }
            } else {
                &all_messages
            };

            if !messages_to_show.is_empty() {
                println!(
                    "  💬 {} ({} messages)",
                    "Inbox Messages".bright_white().bold(),
                    all_messages.len()
                );
                println!();

                for (recipient, msg) in messages_to_show {
                    let time_display = extract_time_display(&msg.timestamp);
                    let content = extract_message_content(&msg.text);
                    let read_marker = if msg.read { "" } else { " *" };
                    println!(
                        "    {} 💬 {} → {}: {}{}",
                        time_display.bright_black(),
                        msg.from.bright_cyan(),
                        recipient.bright_green(),
                        content,
                        read_marker.bright_yellow()
                    );
                }
                println!();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
