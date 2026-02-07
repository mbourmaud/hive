use anyhow::{bail, Result};
use colored::Colorize;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::agent_teams::task_sync;

pub fn run(
    name: String,
    lines: Option<usize>,
    _story: Option<String>,
    follow: bool,
) -> Result<()> {
    let _drone_dir = PathBuf::from(".hive/drones").join(&name);

    if !_drone_dir.exists() {
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

        println!(
            "  🐝 {} Team Conversation",
            team_name.bright_cyan().bold()
        );
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

        // 2. Show task states with agents
        match task_sync::read_team_task_states(team_name) {
            Ok(tasks) if !tasks.is_empty() => {
                // Separate into categories
                let mut in_progress: Vec<_> = tasks
                    .values()
                    .filter(|t| t.status == "in_progress")
                    .collect();
                let mut completed: Vec<_> = tasks
                    .values()
                    .filter(|t| t.status == "completed")
                    .collect();
                let mut pending: Vec<_> = tasks
                    .values()
                    .filter(|t| t.status == "pending")
                    .collect();

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
                        let story = task.story_id.as_deref().unwrap_or(&task.id);
                        let agent = task
                            .owner
                            .as_deref()
                            .map(|a| format!(" [{}]", a.bright_cyan()))
                            .unwrap_or_default();
                        let form = task
                            .active_form
                            .as_deref()
                            .unwrap_or(&task.subject);
                        println!(
                            "    {} {} {}{}",
                            "◐".yellow(),
                            story.bright_yellow(),
                            form,
                            agent
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
                        let story = task.story_id.as_deref().unwrap_or(&task.id);
                        println!(
                            "    {} {} {}",
                            "●".green(),
                            story.green(),
                            task.subject.bright_black()
                        );
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
                        let story = task.story_id.as_deref().unwrap_or(&task.id);
                        println!(
                            "    {} {} {}",
                            "○".bright_black(),
                            story,
                            task.subject.bright_black()
                        );
                    }
                    println!();
                }
            }
            _ => {
                println!("  {}", "No tasks found".bright_black());
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
                        // Parse timestamp for display
                        let time_display = msg
                            .timestamp
                            .split('T')
                            .nth(1)
                            .and_then(|t| t.split('.').next())
                            .unwrap_or(&msg.timestamp);

                        // Try to extract meaningful content from the message
                        // Agent Teams messages are often JSON-encoded
                        let content = if msg.text.starts_with('{') {
                            // Try to parse as JSON and extract meaningful fields
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&msg.text)
                            {
                                if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                                    match msg_type {
                                        "task_assignment" => {
                                            let subject = json
                                                .get("subject")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("?");
                                            let task_id = json
                                                .get("taskId")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("?");
                                            format!("Assigned task #{}: {}", task_id, subject)
                                        }
                                        "task_completed" => {
                                            let task_id = json
                                                .get("taskId")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("?");
                                            format!("Completed task #{}", task_id)
                                        }
                                        "idle_notification" => "Idle - available for work".to_string(),
                                        "shutdown_request" => "Shutdown requested".to_string(),
                                        "shutdown_approved" => "Shutdown approved".to_string(),
                                        _ => {
                                            // Show raw content for unknown types, truncated
                                            let raw = json
                                                .get("content")
                                                .or_else(|| json.get("text"))
                                                .and_then(|v| v.as_str())
                                                .unwrap_or(&msg.text);
                                            if raw.len() > 120 {
                                                format!("{}...", &raw[..120])
                                            } else {
                                                raw.to_string()
                                            }
                                        }
                                    }
                                } else {
                                    let raw = &msg.text;
                                    if raw.len() > 120 {
                                        format!("{}...", &raw[..120])
                                    } else {
                                        raw.to_string()
                                    }
                                }
                            } else {
                                msg.text.clone()
                            }
                        } else {
                            // Plain text message
                            if msg.text.len() > 120 {
                                format!("{}...", &msg.text[..120])
                            } else {
                                msg.text.clone()
                            }
                        };

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
            println!(
                "  {}",
                "Use -f/--follow for live updates".bright_black()
            );
            break;
        }

        thread::sleep(Duration::from_secs(3));
    }

    Ok(())
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
