use super::*;
use crate::types::{DroneState, DroneStatus};

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
        lead_model: None,
        title: None,
        description: None,
        active_agents: Default::default(),
    }
}
