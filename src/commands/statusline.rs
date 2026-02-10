use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use super::common;
use crate::types::{DroneState, DroneStatus};

// ANSI escape codes
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const GRAY: &str = "\x1b[90m";
const BRIGHT_GREEN: &str = "\x1b[92m";
const BRIGHT_RED: &str = "\x1b[91m";
const LIGHT_BLUE: &str = "\x1b[94m";

const SEP: &str = " \x1b[90m\u{2502}\x1b[0m ";

#[derive(Deserialize, Default)]
struct StatuslineInput {
    workspace: Option<Workspace>,
    model: Option<Model>,
    context_window: Option<ContextWindow>,
}

#[derive(Deserialize)]
struct Workspace {
    current_dir: String,
}

#[derive(Deserialize)]
struct Model {
    display_name: String,
}

#[derive(Deserialize)]
struct ContextWindow {
    used_percentage: f64,
}

pub fn run() -> Result<()> {
    let input = read_input();
    let current_dir = input
        .workspace
        .as_ref()
        .map(|w| w.current_dir.as_str())
        .unwrap_or(".");

    let line1 = build_line1(current_dir, &input);
    let line2 = build_line2(current_dir);

    if let Some(l2) = line2 {
        println!("{}\n{}", line1, l2);
    } else {
        println!("{}", line1);
    }

    Ok(())
}

fn read_input() -> StatuslineInput {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_ok() && !buf.trim().is_empty() {
        serde_json::from_str(&buf).unwrap_or_default()
    } else {
        StatuslineInput::default()
    }
}

fn build_line1(current_dir: &str, input: &StatuslineInput) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Project name
    let project = Path::new(current_dir)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| current_dir.to_string());
    parts.push(format!("{CYAN}{BOLD}{project}{RESET}"));

    // Git branch + icons
    if let Some(branch_part) = build_git_part(current_dir) {
        parts.push(branch_part);
    }

    // Model
    if let Some(model) = &input.model {
        parts.push(format!("{LIGHT_BLUE}{}{RESET}", model.display_name));
    }

    // Context %
    if let Some(ctx) = &input.context_window {
        let color = context_color(ctx.used_percentage);
        parts.push(format!("{color}{:.0}%{RESET}", ctx.used_percentage));
    }

    parts.join(SEP)
}

fn build_git_part(current_dir: &str) -> Option<String> {
    let branch = git_branch(current_dir)?;
    let icons = git_icons(current_dir);
    let upstream = git_upstream(current_dir);

    let mut part = format!("{MAGENTA}{branch}{RESET}");
    if !icons.is_empty() {
        part.push_str(&icons);
    }
    if !upstream.is_empty() {
        part.push_str(&format!("{YELLOW}{upstream}{RESET}"));
    }
    Some(part)
}

fn git_branch(current_dir: &str) -> Option<String> {
    let output = ProcessCommand::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(current_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}

fn git_icons(current_dir: &str) -> String {
    let output = match ProcessCommand::new("git")
        .args(["status", "--porcelain"])
        .current_dir(current_dir)
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return String::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut has_untracked = false;
    let mut has_unstaged = false;
    let mut has_staged = false;

    for line in stdout.lines() {
        let bytes = line.as_bytes();
        if bytes.is_empty() {
            continue;
        }
        // Untracked: line starts with '?'
        if bytes[0] == b'?' {
            has_untracked = true;
        }
        // Staged: non-space in column 1 (index 0), excluding '?'
        if bytes[0] != b' ' && bytes[0] != b'?' {
            has_staged = true;
        }
        // Unstaged/modified: non-space in column 2 (index 1), excluding '?'
        if bytes.len() > 1 && bytes[1] != b' ' && bytes[0] != b'?' {
            has_unstaged = true;
        }
    }

    let mut icons = String::new();
    if has_untracked {
        icons.push('+');
    }
    if has_unstaged {
        icons.push('!');
    }
    if has_staged {
        icons.push('*');
    }
    icons
}

fn git_upstream(current_dir: &str) -> String {
    let mut result = String::new();

    // Ahead
    if let Ok(output) = ProcessCommand::new("git")
        .args(["rev-list", "--count", "@{upstream}..HEAD"])
        .current_dir(current_dir)
        .output()
    {
        if output.status.success() {
            if let Ok(n) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u32>()
            {
                if n > 0 {
                    result.push_str(&format!("\u{2191}{n}"));
                }
            }
        }
    }

    // Behind
    if let Ok(output) = ProcessCommand::new("git")
        .args(["rev-list", "--count", "HEAD..@{upstream}"])
        .current_dir(current_dir)
        .output()
    {
        if output.status.success() {
            if let Ok(n) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u32>()
            {
                if n > 0 {
                    result.push_str(&format!("\u{2193}{n}"));
                }
            }
        }
    }

    result
}

fn context_color(pct: f64) -> &'static str {
    if pct > 80.0 {
        RED
    } else if pct >= 50.0 {
        YELLOW
    } else {
        GREEN
    }
}

fn build_line2(current_dir: &str) -> Option<String> {
    let hive_root = find_hive_root(current_dir)?;
    let drones = list_drones_at(&hive_root).ok()?;

    if drones.is_empty() {
        return None;
    }

    // Change CWD to hive root so common:: helpers work with relative paths
    let original_dir = std::env::current_dir().ok();
    let _ = std::env::set_current_dir(&hive_root);

    let now = chrono::Utc::now();
    let mut drone_parts: Vec<String> = Vec::new();

    for (name, status) in &drones {
        // Skip stopped/zombie/cleaning
        match status.status {
            DroneState::Stopped | DroneState::Zombie | DroneState::Cleaning => continue,
            _ => {}
        }

        // Skip completed drones older than 1 hour
        if status.status == DroneState::Completed {
            if let Some(updated_dt) = common::parse_timestamp(&status.updated) {
                let age = now.signed_duration_since(updated_dt);
                if age.num_seconds() > common::DEFAULT_INACTIVE_THRESHOLD_SECS {
                    continue;
                }
            }
        }

        let (completed, total) = common::agent_teams_progress(name);
        let elapsed = common::elapsed_since(&status.started).unwrap_or_default();

        let formatted = format_drone(name, status, completed, total, &elapsed);
        if let Some(f) = formatted {
            drone_parts.push(f);
        }
    }

    // Restore original CWD
    if let Some(orig) = original_dir {
        let _ = std::env::set_current_dir(orig);
    }

    if drone_parts.is_empty() {
        return None;
    }

    let version = env!("CARGO_PKG_VERSION");
    let prefix = format!("{YELLOW}{BOLD}\u{1F41D} hive v{version}{RESET}");
    let drones_str = drone_parts.join(SEP);
    Some(format!("{prefix}{SEP}{drones_str}"))
}

fn format_drone(
    name: &str,
    status: &DroneStatus,
    completed: usize,
    total: usize,
    elapsed: &str,
) -> Option<String> {
    match status.status {
        DroneState::InProgress | DroneState::Starting | DroneState::Resuming => {
            let pid_alive = common::read_drone_pid(name)
                .map(common::is_process_running)
                .unwrap_or(false);

            if pid_alive {
                Some(format!(
                    "{YELLOW}\u{1F41D} {name} {completed}/{total} {elapsed}{RESET}"
                ))
            } else {
                Some(format!(
                    "{GRAY}\u{1F41D} {name} \u{23F8} {completed}/{total} {elapsed}{RESET}"
                ))
            }
        }
        DroneState::Completed => Some(format!(
            "{YELLOW}\u{1F41D} {name}{RESET} {BRIGHT_GREEN}\u{2713}{RESET} {completed}/{total} {elapsed}"
        )),
        DroneState::Error => Some(format!(
            "{YELLOW}\u{1F41D} {name}{RESET} {BRIGHT_RED}\u{2717}{RESET} {completed}/{total}"
        )),
        _ => None,
    }
}

fn find_hive_root(start_dir: &str) -> Option<PathBuf> {
    let mut dir = PathBuf::from(start_dir);
    loop {
        let hive_drones = dir.join(".hive").join("drones");
        if hive_drones.is_dir() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn list_drones_at(hive_root: &Path) -> Result<Vec<(String, DroneStatus)>> {
    let drones_dir = hive_root.join(".hive").join("drones");

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
            if let Ok(contents) = fs::read_to_string(&status_path) {
                if let Ok(status) = serde_json::from_str::<DroneStatus>(&contents) {
                    drones.push((drone_name, status));
                }
            }
        }
    }

    drones.sort_by(|a, b| b.1.updated.cmp(&a.1.updated));

    Ok(drones)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_input() {
        let json = r#"{
            "workspace": { "current_dir": "/home/user/project" },
            "model": { "display_name": "Claude Sonnet 4" },
            "context_window": { "used_percentage": 42.5 }
        }"#;
        let input: StatuslineInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.workspace.unwrap().current_dir, "/home/user/project");
        assert_eq!(input.model.unwrap().display_name, "Claude Sonnet 4");
        assert!((input.context_window.unwrap().used_percentage - 42.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_empty_json() {
        let json = "{}";
        let input: StatuslineInput = serde_json::from_str(json).unwrap();
        assert!(input.workspace.is_none());
        assert!(input.model.is_none());
        assert!(input.context_window.is_none());
    }

    #[test]
    fn test_context_color_green() {
        assert_eq!(context_color(0.0), GREEN);
        assert_eq!(context_color(25.0), GREEN);
        assert_eq!(context_color(49.9), GREEN);
    }

    #[test]
    fn test_context_color_yellow() {
        assert_eq!(context_color(50.0), YELLOW);
        assert_eq!(context_color(65.0), YELLOW);
        assert_eq!(context_color(80.0), YELLOW);
    }

    #[test]
    fn test_context_color_red() {
        assert_eq!(context_color(80.1), RED);
        assert_eq!(context_color(95.0), RED);
        assert_eq!(context_color(100.0), RED);
    }

    #[test]
    fn test_format_drone_completed() {
        let status = make_test_status(DroneState::Completed);
        let result = format_drone("my-drone", &status, 5, 5, "10m 30s");
        assert!(result.is_some());
        let s = result.unwrap();
        assert!(s.contains("my-drone"));
        assert!(s.contains("\u{2713}"));
        assert!(s.contains("5/5"));
        assert!(s.contains("10m 30s"));
    }

    #[test]
    fn test_format_drone_error() {
        let status = make_test_status(DroneState::Error);
        let result = format_drone("err-drone", &status, 2, 5, "5m 0s");
        assert!(result.is_some());
        let s = result.unwrap();
        assert!(s.contains("err-drone"));
        assert!(s.contains("\u{2717}"));
        assert!(s.contains("2/5"));
    }

    #[test]
    fn test_format_drone_stopped_skipped() {
        let status = make_test_status(DroneState::Stopped);
        let result = format_drone("stopped-drone", &status, 0, 0, "");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_hive_root_none() {
        // A path that definitely has no .hive/drones
        let result = find_hive_root("/nonexistent/path");
        assert!(result.is_none());
    }

    #[test]
    fn test_git_icons_empty_output() {
        // git_icons on a nonexistent dir should return empty
        let result = git_icons("/nonexistent");
        assert!(result.is_empty());
    }

    fn make_test_status(state: DroneState) -> DroneStatus {
        DroneStatus {
            drone: "test".to_string(),
            prd: "test.json".to_string(),
            branch: "hive/test".to_string(),
            worktree: "/tmp/test".to_string(),
            local_mode: false,
            execution_mode: Default::default(),
            backend: "agent_team".to_string(),
            status: state,
            current_task: None,
            completed: vec![],
            story_times: Default::default(),
            total: 5,
            started: "2026-01-01T00:00:00Z".to_string(),
            updated: chrono::Utc::now().to_rfc3339(),
            error_count: 0,
            last_error: None,
            active_agents: Default::default(),
        }
    }
}
