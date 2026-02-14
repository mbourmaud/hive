use std::path::Path;

use crate::commands::common::{
    elapsed_since, format_duration, is_process_running, read_drone_pid_at,
};
use crate::types::DroneState;

use super::dto::TaskInfo;

/// Check if activity.log ends with a successful result event.
pub fn has_success_result(activity_log_path: &Path) -> bool {
    let Some(contents) = std::fs::read_to_string(activity_log_path).ok() else {
        return false;
    };
    let Some(last_line) = contents.lines().rev().find(|l| !l.trim().is_empty()) else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(last_line) else {
        return false;
    };
    let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
    if typ != "result" {
        return false;
    }
    let subtype = v.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
    let is_error = v.get("is_error").and_then(|e| e.as_bool()).unwrap_or(true);
    subtype == "success" || !is_error
}

/// Determine the liveness state of a drone at a given project root.
pub fn determine_liveness(project_root: &Path, drone_name: &str, status: &DroneState) -> String {
    match status {
        DroneState::Completed => "completed".to_string(),
        DroneState::Stopped => "stopped".to_string(),
        DroneState::Zombie => "dead".to_string(),
        DroneState::InProgress | DroneState::Starting | DroneState::Resuming => {
            let pid_alive = read_drone_pid_at(project_root, drone_name)
                .map(is_process_running)
                .unwrap_or(false);
            if pid_alive {
                return "working".to_string();
            }
            let log_path = project_root
                .join(".hive/drones")
                .join(drone_name)
                .join("activity.log");
            if has_success_result(&log_path) {
                "completed".to_string()
            } else {
                "dead".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
}

pub fn determine_member_liveness(member_name: &str, tasks: &[TaskInfo]) -> String {
    let has_active_task = tasks
        .iter()
        .any(|t| t.owner.as_deref() == Some(member_name) && t.status == "in_progress");

    if has_active_task {
        "working".to_string()
    } else {
        "idle".to_string()
    }
}

pub fn compute_task_duration(
    created_at: Option<u64>,
    updated_at: Option<u64>,
    status: &str,
) -> Option<String> {
    let start = created_at?;
    let end = match status {
        "completed" => updated_at?,
        "in_progress" => std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as u64,
        _ => return None,
    };

    if end <= start {
        return None;
    }

    let secs = (end - start) / 1000;
    Some(format_duration_secs(secs))
}

pub fn compute_elapsed(started: &str) -> String {
    elapsed_since(started).unwrap_or_else(|| "?".to_string())
}

fn format_duration_secs(total_secs: u64) -> String {
    format_duration(chrono::Duration::seconds(total_secs as i64))
}
