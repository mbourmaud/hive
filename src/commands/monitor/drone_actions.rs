use anyhow::Result;
use std::fs;
use std::path::PathBuf;

use crate::types::{DroneState, DroneStatus, Prd};

// Handler for 'New Drone' action - browse PRDs and launch
pub(crate) fn handle_new_drone<B: ratatui::backend::Backend>(
    _terminal: &mut ratatui::Terminal<B>,
) -> Result<Option<String>> {
    use dialoguer::{theme::ColorfulTheme, Input, Select};
    use std::io;

    // Find all PRD files
    let prds = find_prd_files()?;

    if prds.is_empty() {
        return Ok(Some(
            "No PRD files found in .hive/prds/ or project root".to_string(),
        ));
    }

    // Disable raw mode temporarily for dialoguer
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

    let result = (|| -> Result<Option<String>> {
        // Let user select PRD
        let prd_names: Vec<String> = prds.iter().map(|p| p.display().to_string()).collect();
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select PRD")
            .items(&prd_names)
            .default(0)
            .interact_opt()?;

        let prd_path = match selection {
            Some(idx) => &prds[idx],
            None => return Ok(None), // User cancelled
        };

        // Read PRD to get default name
        let prd_contents = fs::read_to_string(prd_path)?;
        let prd: Prd = serde_json::from_str(&prd_contents)?;
        let default_name = prd.id.clone();

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
        crate::commands::start::run(
            drone_name.clone(),
            None,
            false,
            false,
            model,
            3,
            false,
        )?;

        Ok(Some(format!("\u{1f41d} Launched drone: {}", drone_name)))
    })();

    // Re-enable raw mode
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    result
}

// Find all PRD files in .hive/prds/ and project root
pub(crate) fn find_prd_files() -> Result<Vec<PathBuf>> {
    let mut prds = Vec::new();

    // Search in .hive/prds/
    let hive_prds = PathBuf::from(".hive").join("prds");
    if hive_prds.exists() {
        for entry in fs::read_dir(&hive_prds)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                prds.push(path);
            }
        }
    }

    // Search in project root for prd*.json
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if name.starts_with("prd") && path.extension().and_then(|s| s.to_str()) == Some("json")
            {
                prds.push(path);
            }
        }
    }

    Ok(prds)
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
                                let status = todo.get("status").and_then(|s| s.as_str()).unwrap_or("");
                                if status == "in_progress" {
                                    if let Some(form) = todo.get("activeForm").and_then(|f| f.as_str()) {
                                        return form.to_string();
                                    }
                                }
                            }
                        }
                    }
                }

                // For other tools, show a description
                let desc = item.pointer("/input/description")
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

/// Extract a meaningful title from an Agent Teams task description.
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

// Handler for 'Resume' action - resumes a drone with new stories or stopped drone
pub(crate) fn handle_resume_drone(drone_name: &str) -> Result<String> {
    // Update status.json to reflect new PRD story count
    let status_path = PathBuf::from(".hive")
        .join("drones")
        .join(drone_name)
        .join("status.json");
    let _prd_path_dir = PathBuf::from(".hive").join("prds");

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

    // Launch drone with resume flag
    crate::commands::start::run(
        drone_name.to_string(),
        None,
        true,
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
