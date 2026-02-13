use anyhow::{bail, Result};
use colored::Colorize;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::agent_teams::task_sync;

pub fn run(name: String, lines: Option<usize>, follow: bool) -> Result<()> {
    let drone_dir = PathBuf::from(".hive/drones").join(&name);

    if !drone_dir.exists() {
        bail!("Drone '{}' not found", name);
    }

    // Always show team conversation (story parameter ignored)
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

        // 1. Show team roster
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

        // 2. Show task states with agents (from filesystem)
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
        } else {
            // Separate into categories
            let mut in_progress: Vec<_> = tasks
                .values()
                .filter(|t| t.status == "in_progress")
                .collect();
            let mut completed: Vec<_> =
                tasks.values().filter(|t| t.status == "completed").collect();
            let mut pending: Vec<_> = tasks.values().filter(|t| t.status == "pending").collect();

            in_progress.sort_by_key(|t| &t.id);
            completed.sort_by_key(|t| &t.id);
            pending.sort_by_key(|t| &t.id);

            // Active tasks
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

            // Completed tasks
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

            // Pending tasks
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

        // 3. Show inbox messages (agent-to-agent conversation)
        match task_sync::read_team_inboxes(team_name) {
            Ok(inboxes) if !inboxes.is_empty() => {
                // Collect all messages with recipient info, sort by timestamp
                let mut all_messages: Vec<(String, &task_sync::InboxMessage)> = Vec::new();
                for (recipient, messages) in &inboxes {
                    for msg in messages {
                        all_messages.push((recipient.clone(), msg));
                    }
                }
                all_messages.sort_by(|a, b| a.1.timestamp.cmp(&b.1.timestamp));

                // Apply lines limit
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

        if !follow {
            // Show hint for follow mode
            println!("  {}", "Use -f/--follow for live updates".bright_black());
            break;
        }

        thread::sleep(Duration::from_secs(3));
    }

    Ok(())
}

/// Extract the time portion from an ISO timestamp for compact display.
fn extract_time_display(timestamp: &str) -> &str {
    timestamp
        .split('T')
        .nth(1)
        .and_then(|t| t.split('.').next())
        .unwrap_or(timestamp)
}

/// Truncate a string to `max_len` characters, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Extract human-readable content from a message.
/// Agent Teams messages are often JSON-encoded with a "type" field.
fn extract_message_content(text: &str) -> String {
    if !text.starts_with('{') {
        return truncate(text, 120);
    }

    let json = match serde_json::from_str::<serde_json::Value>(text) {
        Ok(v) => v,
        Err(_) => return text.to_string(),
    };

    let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) else {
        return truncate(text, 120);
    };

    match msg_type {
        "task_assignment" => {
            let subject = json.get("subject").and_then(|v| v.as_str()).unwrap_or("?");
            let task_id = json.get("taskId").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Assigned task #{}: {}", task_id, subject)
        }
        "task_completed" => {
            let task_id = json.get("taskId").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Completed task #{}", task_id)
        }
        "idle_notification" => "Idle - available for work".to_string(),
        "shutdown_request" => "Shutdown requested".to_string(),
        "shutdown_approved" => "Shutdown approved".to_string(),
        _ => {
            let raw = json
                .get("content")
                .or_else(|| json.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or(text);
            truncate(raw, 120)
        }
    }
}

/// Extract a meaningful display title and optional agent name from a task.
/// For internal teammate tasks (subject = agent name), parse the description
/// to find the actual work being done.
fn extract_task_display(task: &task_sync::TeamTaskInfo) -> (String, Option<String>) {
    if task.is_internal {
        // Internal tasks have the agent name as subject and work description in description.
        // Try to extract "Task N: <description>" from the prompt text.
        let title = task
            .description
            .lines()
            .find(|l| l.contains("Your task:") || l.contains("Your tasks:"))
            .map(|l| {
                l.trim_start_matches("Your task:")
                    .trim_start_matches("Your tasks:")
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .or_else(|| {
                // Try "Task N: <description>" pattern from first line
                task.description
                    .lines()
                    .find(|l| l.contains("Task ") && l.contains(':'))
                    .and_then(|l| l.split_once("Task "))
                    .and_then(|(_, rest)| rest.split_once(':'))
                    .map(|(_, desc)| desc.trim().to_string())
            })
            .filter(|s| !s.is_empty())
            .or_else(|| {
                // Fallback: first non-boilerplate line
                task.description
                    .lines()
                    .find(|l| {
                        !l.is_empty()
                            && !l.starts_with("You are")
                            && !l.starts_with("Check the task")
                    })
                    .map(|l| l.to_string())
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| task.subject.clone());
        let title = title.trim_start_matches('#').trim().to_string();
        (title, Some(task.subject.clone()))
    } else {
        let subj = task.subject.trim_start_matches('#').trim().to_string();
        (subj, task.owner.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_show_team_conversation_no_panic() {
        // Basic smoke test - just ensure it doesn't panic when team doesn't exist
        let result = show_team_conversation("nonexistent-team", Some(10), false);
        // Might fail or succeed depending on filesystem, just ensuring no panic
        let _ = result;
    }
}
