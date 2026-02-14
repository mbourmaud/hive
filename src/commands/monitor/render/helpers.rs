use ratatui::style::Color;

use crate::agent_teams::task_sync::TeamTaskInfo;

/// Agent color palette for team member display
const AGENT_PALETTE: [Color; 8] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Blue,
    Color::Green,
    Color::Red,
    Color::LightCyan,
    Color::LightMagenta,
];

/// Get a unique color for an agent based on their index
pub fn get_agent_color(agent_index: usize) -> Color {
    AGENT_PALETTE[agent_index % AGENT_PALETTE.len()]
}

/// Shorten a Claude model name for compact display.
/// e.g. "claude-sonnet-4-5-20250929" -> "sonnet-4.5"
///      "claude-opus-4-6" -> "opus-4.6"
///      "claude-haiku-4-5-20251001" -> "haiku-4.5"
pub fn shorten_model_name(model: &str) -> String {
    // Strip "claude-" prefix
    let name = model.strip_prefix("claude-").unwrap_or(model);

    // Try to extract family and version: "sonnet-4-5-20250929" -> "sonnet", "4", "5"
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() >= 3 {
        let family = parts[0]; // sonnet, opus, haiku
                               // Check if parts[1] and parts[2] are version digits
        if parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].chars().all(|c| c.is_ascii_digit())
        {
            return format!("{}-{}.{}", family, parts[1], parts[2]);
        }
    }

    // Fallback: just use the stripped name, truncated
    if name.len() > 15 {
        name[..15].to_string()
    } else {
        name.to_string()
    }
}

/// Extract a meaningful title from an internal task's description.
/// Internal tasks have the agent name as `subject` and the actual work in `description`.
pub fn extract_internal_task_title(task: &TeamTaskInfo) -> String {
    let title = task
        .description
        .lines()
        .find(|l| l.starts_with("Your task:") || l.starts_with("Your tasks:"))
        .map(|l| {
            l.trim_start_matches("Your task:")
                .trim_start_matches("Your tasks:")
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .or_else(|| {
            task.description
                .lines()
                .find(|l| {
                    !l.is_empty() && !l.starts_with("You are") && !l.starts_with("Check the task")
                })
                .map(|l| l.to_string())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| task.subject.clone());
    // Strip markdown header prefixes (e.g. "## Title" -> "Title")
    title.trim_start_matches('#').trim().to_string()
}
