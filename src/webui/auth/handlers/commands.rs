use std::collections::HashSet;
use std::path::Path;

use axum::Json;

use crate::webui::error::ApiResult;

use super::super::dto::CustomCommand;

pub async fn list_commands() -> ApiResult<Json<Vec<CustomCommand>>> {
    let mut commands: Vec<CustomCommand> = Vec::new();
    let mut seen = HashSet::new();

    if let Ok(cwd) = std::env::current_dir() {
        let project_dir = cwd.join(".claude").join("commands");
        scan_commands_dir(&project_dir, "project", &mut commands, &mut seen);
    }

    if let Some(home) = dirs::home_dir() {
        let user_dir = home.join(".claude").join("commands");
        scan_commands_dir(&user_dir, "user", &mut commands, &mut seen);

        let tools_dir = home.join(".claude").join("commands").join("tools");
        scan_commands_dir(&tools_dir, "tools", &mut commands, &mut seen);
    }

    Ok(Json(commands))
}

fn scan_commands_dir(
    dir: &Path,
    source: &str,
    commands: &mut Vec<CustomCommand>,
    seen: &mut HashSet<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if path.is_dir() {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if name.is_empty() || seen.contains(&name) {
            continue;
        }

        let description = std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                    .map(|l| {
                        let trimmed = l.trim();
                        let truncated: String = trimmed.chars().take(80).collect();
                        if truncated.len() < trimmed.len() {
                            format!("{truncated}…")
                        } else {
                            truncated
                        }
                    })
            })
            .unwrap_or_default();

        seen.insert(name.clone());
        commands.push(CustomCommand {
            name,
            description,
            source: source.to_string(),
        });
    }
}
