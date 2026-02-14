use crate::agent_teams::task_sync;

/// Extract the time portion from an ISO timestamp for compact display.
pub fn extract_time_display(timestamp: &str) -> &str {
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
pub fn extract_message_content(text: &str) -> String {
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
pub fn extract_task_display(task: &task_sync::TeamTaskInfo) -> (String, Option<String>) {
    if task.is_internal {
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
                task.description
                    .lines()
                    .find(|l| l.contains("Task ") && l.contains(':'))
                    .and_then(|l| l.split_once("Task "))
                    .and_then(|(_, rest)| rest.split_once(':'))
                    .map(|(_, desc)| desc.trim().to_string())
            })
            .filter(|s| !s.is_empty())
            .or_else(|| {
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
