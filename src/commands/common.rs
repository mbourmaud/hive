//! Common utilities shared across command modules.
//!
//! This module extracts duplicated functionality from status.rs and utils.rs
//! to provide a single source of truth for common operations.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{DroneStatus, Plan};

// ============================================================================
// Constants
// ============================================================================

/// Default threshold in seconds for considering a drone inactive (1 hour)
pub const DEFAULT_INACTIVE_THRESHOLD_SECS: i64 = 3600;

/// Maximum drone name length before truncation
pub const MAX_DRONE_NAME_LEN: usize = 35;

/// Seconds in an hour
pub const SECONDS_PER_HOUR: i64 = 3600;

/// Seconds in a minute
pub const SECONDS_PER_MINUTE: i64 = 60;

// ============================================================================
// Time Utilities
// ============================================================================

/// Parse an ISO8601/RFC3339 timestamp string into a DateTime.
pub fn parse_timestamp(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Calculate the duration between two timestamp strings.
pub fn duration_between(start: &str, end: &str) -> Option<chrono::Duration> {
    let start_dt = parse_timestamp(start)?;
    let end_dt = parse_timestamp(end)?;
    Some(end_dt.signed_duration_since(start_dt))
}

/// Format a duration as a human-readable string (e.g., "1h 23m" or "5m 30s").
pub fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / SECONDS_PER_HOUR;
    let minutes = (total_seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let seconds = total_seconds % SECONDS_PER_MINUTE;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Calculate and format elapsed time since a timestamp.
pub fn elapsed_since(start: &str) -> Option<String> {
    let start_dt = parse_timestamp(start)?;
    let duration = Utc::now().signed_duration_since(start_dt);
    Some(format_duration(duration))
}

// ============================================================================
// PRD Utilities
// ============================================================================

/// Load a PRD from the given file path.
pub fn load_prd(path: &Path) -> Option<Plan> {
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Reconcile status with actual PRD.
/// Returns (completed_tasks, total_tasks) from Agent Teams progress.
/// Also updates status.json with latest progress as a cache.
///
/// Returns (valid_completed_count, total).
pub fn reconcile_progress(status: &DroneStatus) -> (usize, usize) {
    let (completed, total) = agent_teams_progress(&status.drone);

    // Persist progress to status.json as a cache
    if total > 0 && (completed != status.completed.len() || total != status.total) {
        let status_path = PathBuf::from(".hive/drones")
            .join(&status.drone)
            .join("status.json");
        if status_path.exists() {
            if let Ok(contents) = fs::read_to_string(&status_path) {
                if let Ok(mut cached_status) = serde_json::from_str::<DroneStatus>(&contents) {
                    cached_status.total = total;
                    // Update completed list length to match
                    cached_status.completed.truncate(completed);
                    while cached_status.completed.len() < completed {
                        cached_status
                            .completed
                            .push(format!("task-{}", cached_status.completed.len() + 1));
                    }
                    cached_status.updated = chrono::Utc::now().to_rfc3339();
                    let _ = fs::write(
                        &status_path,
                        serde_json::to_string_pretty(&cached_status).unwrap_or_default(),
                    );
                }
            }
        }
    }

    (completed, total)
}

/// Reconcile status with a provided PRD.
/// Returns Agent Teams task progress.
///
/// Returns (valid_completed_count, total).
pub fn reconcile_progress_with_prd(status: &DroneStatus, _prd: &Plan) -> (usize, usize) {
    agent_teams_progress(&status.drone)
}

/// Get progress from Agent Teams task list (for plan-only PRDs).
///
/// Multi-source fallback:
/// 1. Try live tasks from `~/.claude/tasks/<drone>/` (current logic)
/// 2. If empty, fall back to `reconstruct_progress()` from events.ndjson
/// 3. If both empty, return (0, 0)
///
/// Returns (completed_tasks, total_tasks). Filters out internal tracking tasks.
fn agent_teams_progress(drone_name: &str) -> (usize, usize) {
    use crate::agent_teams;
    use crate::events;

    let tasks = agent_teams::read_task_list(drone_name).unwrap_or_default();

    // Filter out internal tasks (auto-created teammate tracking)
    let user_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| {
            !t.metadata
                .as_ref()
                .and_then(|m| m.get("_internal"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .collect();

    let total = user_tasks.len();
    let completed = user_tasks
        .iter()
        .filter(|t| t.status == "completed")
        .count();

    // Source 1: live tasks are available
    if total > 0 {
        return (completed, total);
    }

    // Source 2: fall back to event-sourced progress from events.ndjson
    let (event_completed, event_total) = events::reconstruct_progress(drone_name);
    if event_total > 0 {
        return (event_completed, event_total);
    }

    // Source 3: nothing available
    (0, 0)
}

// ============================================================================
// Process Utilities
// ============================================================================

/// Check if a process is running by PID.
pub fn is_process_running(pid: i32) -> bool {
    #[cfg(unix)]
    {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;

        match kill(Pid::from_raw(pid), None) {
            Ok(()) => true,
            Err(nix::errno::Errno::ESRCH) => false,
            Err(_) => true, // Process exists but we lack permission
        }
    }

    #[cfg(not(unix))]
    {
        Path::new(&format!("/proc/{}", pid)).exists()
    }
}

/// Read the PID from a drone's .pid file.
pub fn read_drone_pid(drone_name: &str) -> Option<i32> {
    let pid_path = PathBuf::from(".hive")
        .join("drones")
        .join(drone_name)
        .join(".pid");

    fs::read_to_string(pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

// ============================================================================
// PR / GitHub Utilities
// ============================================================================

/// Check if a PR for the given branch has been merged.
pub fn is_pr_merged(branch: &str) -> bool {
    std::process::Command::new("gh")
        .args(["pr", "view", branch, "--json", "state", "-q", ".state"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "MERGED")
        .unwrap_or(false)
}

/// Check if a PR exists and is open for the given branch.
pub fn is_pr_open(branch: &str) -> bool {
    std::process::Command::new("gh")
        .args(["pr", "view", branch, "--json", "state", "-q", ".state"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "OPEN")
        .unwrap_or(false)
}

// ============================================================================
// Drone Listing
// ============================================================================

/// List all drones with their status, sorted by most recently updated.
pub fn list_drones() -> Result<Vec<(String, DroneStatus)>> {
    let drones_dir = PathBuf::from(".hive").join("drones");

    if !drones_dir.exists() {
        return Ok(Vec::new());
    }

    let mut drones = Vec::new();

    for entry in fs::read_dir(&drones_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let drone_name = entry.file_name().to_string_lossy().into_owned();
        let status_path = entry.path().join("status.json");

        if status_path.exists() {
            let contents = fs::read_to_string(&status_path)
                .with_context(|| format!("Failed to read status for drone '{}'", drone_name))?;
            let status: DroneStatus = serde_json::from_str(&contents)
                .with_context(|| format!("Failed to parse status for drone '{}'", drone_name))?;
            drones.push((drone_name, status));
        }
    }

    drones.sort_by(|a, b| b.1.updated.cmp(&a.1.updated));

    Ok(drones)
}

// ============================================================================
// String Utilities
// ============================================================================

/// Truncate a string with ellipsis if it exceeds max_len.
pub fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

/// Word-wrap text to fit within max_width.
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        let duration = chrono::Duration::seconds(45);
        assert_eq!(format_duration(duration), "45s");
    }

    #[test]
    fn test_format_duration_minutes() {
        let duration = chrono::Duration::seconds(125);
        assert_eq!(format_duration(duration), "2m 5s");
    }

    #[test]
    fn test_format_duration_hours() {
        let duration = chrono::Duration::seconds(3725);
        assert_eq!(format_duration(duration), "1h 2m");
    }

    #[test]
    fn test_truncate_with_ellipsis_short() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_with_ellipsis_long() {
        assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
    }

    #[test]
    fn test_wrap_text_single_line() {
        let result = wrap_text("hello world", 20);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn test_wrap_text_multiple_lines() {
        let result = wrap_text("hello world foo bar", 10);
        assert_eq!(result, vec!["hello", "world foo", "bar"]);
    }

    #[test]
    fn test_wrap_text_empty() {
        let result = wrap_text("", 10);
        assert_eq!(result, vec![""]);
    }
}
