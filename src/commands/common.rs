//! Common utilities shared across command modules.
//!
//! This module extracts duplicated functionality from status.rs and utils.rs
//! to provide a single source of truth for common operations.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Output};
use std::time::{Duration, Instant};

use crate::types::{DroneStatus, LegacyJsonPlan, Plan};

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

/// Load a plan from the given file path (supports .md and legacy .json).
pub fn load_prd(path: &Path) -> Option<Plan> {
    let contents = fs::read_to_string(path).ok()?;
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "md" => {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let structured_tasks = crate::plan_parser::parse_tasks(&contents);
            Some(Plan {
                id,
                content: contents,
                target_branch: None,
                base_branch: None,
                structured_tasks,
            })
        }
        "json" => {
            let legacy: LegacyJsonPlan = serde_json::from_str(&contents).ok()?;
            Some(legacy.into())
        }
        _ => None,
    }
}

/// Get read-only progress from Agent Teams task list (filesystem only).
///
/// Reads from `~/.claude/tasks/<drone>/` — no events.ndjson fallback.
/// Returns (completed_tasks, total_tasks). Filters out internal tracking tasks.
/// Note: This is a simple read-only function. The snapshot store in the TUI
/// provides monotonicity guarantees on top of this.
pub fn agent_teams_progress(drone_name: &str) -> (usize, usize) {
    use crate::agent_teams;

    let tasks = agent_teams::read_task_list_safe(drone_name);

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

    // Source 1: user tasks (non-internal)
    if total > 0 {
        return (completed, total);
    }

    // Source 2: internal tasks (team lead used TeamCreate, not TaskCreate)
    let all_total = tasks.len();
    let all_completed = tasks.iter().filter(|t| t.status == "completed").count();
    if all_total > 0 {
        return (all_completed, all_total);
    }

    // Nothing available
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

/// Run a command with a timeout. Returns None if the command times out or fails to spawn.
pub fn command_with_timeout(cmd: &mut ProcessCommand, timeout_secs: u64) -> Option<Output> {
    let mut child = cmd.spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) if start.elapsed() > Duration::from_secs(timeout_secs) => {
                let _ = child.kill();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => return None,
        }
    }
}

// ============================================================================
// Drone Listing
// ============================================================================

/// List all drones with their status, sorted by most recently updated.
pub fn list_drones() -> Result<Vec<(String, DroneStatus)>> {
    let drones_dir = PathBuf::from(".hive").join("drones");
    list_drones_in(drones_dir)
}

/// List all drones at a specific project root (absolute path).
pub fn list_drones_at(project_root: &Path) -> Result<Vec<(String, DroneStatus)>> {
    let drones_dir = project_root.join(".hive").join("drones");
    list_drones_in(drones_dir)
}

fn list_drones_in(drones_dir: PathBuf) -> Result<Vec<(String, DroneStatus)>> {
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

/// Read the PID from a drone's .pid file at a specific project root.
pub fn read_drone_pid_at(project_root: &Path, drone_name: &str) -> Option<i32> {
    let pid_path = project_root
        .join(".hive")
        .join("drones")
        .join(drone_name)
        .join(".pid");

    fs::read_to_string(pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

// ============================================================================
// String Utilities
// ============================================================================

/// Truncate a string with ellipsis if it exceeds max_len (char-aware).
pub fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    } else {
        s.chars().take(max_len).collect()
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

    #[test]
    fn test_parse_timestamp_with_timezone() {
        // Test with actual timestamp format from status.json
        let timestamp = "2026-02-09T09:43:19.919277+00:00";
        let result = parse_timestamp(timestamp);
        assert!(
            result.is_some(),
            "Should parse RFC3339 timestamp with timezone"
        );
    }

    #[test]
    fn test_elapsed_since_returns_value() {
        // Test with a timestamp 5 minutes ago
        let five_mins_ago = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
        let result = elapsed_since(&five_mins_ago);
        assert!(result.is_some(), "Should calculate elapsed time");
        let elapsed = result.unwrap();
        // Should show something like "5m 0s" or similar
        assert!(
            elapsed.contains("m") || elapsed.contains("s"),
            "Should format as time string"
        );
    }

    #[test]
    fn test_elapsed_since_with_malformed_timestamp() {
        let result = elapsed_since("not-a-timestamp");
        assert!(result.is_none(), "Should return None for invalid timestamp");
    }
}
