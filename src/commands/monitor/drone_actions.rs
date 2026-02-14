use anyhow::Result;
use std::fs;
use std::path::PathBuf;

use crate::commands::common::load_prd;
use crate::types::{DroneState, DroneStatus};

// Handler for 'New Drone' action - browse PRDs and launch
pub(crate) fn handle_new_drone<B: ratatui::backend::Backend>(
    _terminal: &mut ratatui::Terminal<B>,
) -> Result<Option<String>> {
    use dialoguer::{theme::ColorfulTheme, Input, Select};
    use std::io;

    // Find all plan files
    let plans = find_plan_files()?;

    if plans.is_empty() {
        return Ok(Some(
            "No PRD files found in .hive/prds/ or project root".to_string(),
        ));
    }

    // Disable raw mode temporarily for dialoguer
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

    let result = (|| -> Result<Option<String>> {
        // Let user select plan
        let plan_names: Vec<String> = plans.iter().map(|p| p.display().to_string()).collect();
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select PRD")
            .items(&plan_names)
            .default(0)
            .interact_opt()?;

        let plan_path = match selection {
            Some(idx) => &plans[idx],
            None => return Ok(None), // User cancelled
        };

        // Read plan to get default name
        let plan = load_prd(plan_path).ok_or_else(|| anyhow::anyhow!("Failed to load plan"))?;
        let default_name = plan.id.clone();

        // Prompt for drone name
        let drone_name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Drone name")
            .default(default_name)
            .interact_text()?;

        // Prompt for model
        let models = vec!["sonnet", "opus", "haiku"];
        let model_idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select model")
            .items(&models)
            .default(0)
            .interact()?;
        let model = models[model_idx].to_string();

        // Launch drone using start command
        crate::commands::start::run(drone_name.clone(), false, model, 3, false)?;

        Ok(Some(format!("\u{1f41d} Launched drone: {}", drone_name)))
    })();

    // Re-enable raw mode
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    result
}

// Find all plan files in .hive/plans/, .hive/prds/ (compat)
pub(crate) fn find_plan_files() -> Result<Vec<PathBuf>> {
    let mut plans = Vec::new();

    // Search in .hive/plans/ first, then .hive/prds/ for compat
    for dir_name in &["plans", "prds"] {
        let hive_dir = PathBuf::from(".hive").join(dir_name);
        if hive_dir.exists() {
            for entry in fs::read_dir(&hive_dir)? {
                let entry = entry?;
                let path = entry.path();
                let ext = path.extension().and_then(|s| s.to_str());
                if ext == Some("md") || ext == Some("json") {
                    plans.push(path);
                }
            }
            break; // prds is usually a symlink to plans, avoid duplicates
        }
    }

    Ok(plans)
}

/// Extract the last meaningful activity from a stream-json activity log.
/// Looks for tool usage, text output, or todo status to show what the agent is doing.
pub(crate) fn extract_last_activity(log_contents: &str) -> String {
    // Walk backwards through lines to find the last meaningful event
    for line in log_contents.lines().rev() {
        let parsed: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if parsed.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }

        let content = match parsed.pointer("/message/content") {
            Some(c) => c,
            None => continue,
        };

        let items = match content.as_array() {
            Some(a) => a,
            None => continue,
        };

        for item in items.iter().rev() {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

            // Tool use — show what tool is being called
            if item_type == "tool_use" {
                let tool_name = item.get("name").and_then(|n| n.as_str()).unwrap_or("?");

                // Special case: TodoWrite — extract the active form
                if tool_name == "TodoWrite" || tool_name == "TaskCreate" {
                    if let Some(input) = item.get("input") {
                        // TaskCreate: look at activeForm directly
                        if let Some(form) = input.get("activeForm").and_then(|f| f.as_str()) {
                            return form.to_string();
                        }
                        // TodoWrite: look at the first in_progress todo
                        if let Some(todos) = input.get("todos").and_then(|t| t.as_array()) {
                            for todo in todos {
                                let status =
                                    todo.get("status").and_then(|s| s.as_str()).unwrap_or("");
                                if status == "in_progress" {
                                    if let Some(form) =
                                        todo.get("activeForm").and_then(|f| f.as_str())
                                    {
                                        return form.to_string();
                                    }
                                }
                            }
                        }
                    }
                }

                // For other tools, show a description
                let desc = item
                    .pointer("/input/description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                if !desc.is_empty() {
                    return format!("{}: {}", tool_name, desc);
                }
                return format!("Using {}", tool_name);
            }

            // Text output — show a truncated version
            if item_type == "text" {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    let first_line = text.lines().next().unwrap_or(text);
                    let end = first_line.len().min(80);
                    return first_line[..end].trim().to_string();
                }
            }
        }
    }

    String::new()
}

// Handler for 'Stop' action (uses quiet mode to avoid corrupting TUI)
pub(crate) fn handle_stop_drone(drone_name: &str) -> Result<String> {
    crate::commands::kill_clean::kill_quiet(drone_name.to_string())?;
    Ok(format!("\u{1f6d1} Stopped drone: {}", drone_name))
}

// Handler for 'Clean' action - cleans in background (auto-stops if running)
pub(crate) fn handle_clean_drone(drone_name: &str) -> Result<String> {
    crate::commands::kill_clean::clean_background(drone_name.to_string());
    Ok(format!("\u{1f9f9} Cleaning drone: {}", drone_name))
}

// Handler for 'Resume' action - resumes a stopped drone
pub(crate) fn handle_resume_drone(drone_name: &str) -> Result<String> {
    // Reset status to in_progress
    let status_path = PathBuf::from(".hive")
        .join("drones")
        .join(drone_name)
        .join("status.json");
    if let Ok(status_content) = fs::read_to_string(&status_path) {
        if let Ok(mut status) = serde_json::from_str::<DroneStatus>(&status_content) {
            // Reset status to in_progress (total is tracked by Agent Teams tasks)
            status.status = DroneState::InProgress;
            status.updated = chrono::Utc::now().to_rfc3339();

            // Write updated status
            if let Ok(updated_json) = serde_json::to_string_pretty(&status) {
                let _ = fs::write(&status_path, updated_json);
            }
        }
    }

    // Launch drone (auto-resume detects existing drone)
    crate::commands::start::run(
        drone_name.to_string(),
        false,
        "sonnet".to_string(),
        3,
        false,
    )?;
    Ok(format!("\u{1f504} Resumed drone: {}", drone_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_last_activity_empty() {
        assert_eq!(extract_last_activity(""), "");
    }
}
