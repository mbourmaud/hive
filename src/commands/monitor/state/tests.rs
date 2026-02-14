use super::*;
use crate::types::{DroneState, DroneStatus, ExecutionMode};
use std::collections::HashMap;

#[test]
fn test_drone_elapsed_with_running_drone() {
    // Create a status with a timestamp 5 minutes ago
    let five_mins_ago = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
    let status = DroneStatus {
        drone: "test-drone".to_string(),
        prd: "test.json".to_string(),
        branch: "test-branch".to_string(),
        worktree: "/tmp/test".to_string(),
        local_mode: false,
        execution_mode: ExecutionMode::AgentTeam,
        backend: "agent_team".to_string(),
        status: DroneState::InProgress,
        current_task: None,
        completed: vec![],
        story_times: HashMap::new(),
        total: 0,
        started: five_mins_ago.clone(),
        updated: chrono::Utc::now().to_rfc3339(),
        error_count: 0,
        last_error: None,
        lead_model: None,
        active_agents: HashMap::new(),
    };

    let elapsed = TuiState::drone_elapsed(&status);
    // Should show something like "5m 0s" or similar
    assert!(!elapsed.is_empty(), "Elapsed time should not be empty");
    assert!(
        elapsed.contains("m") || elapsed.contains("s"),
        "Should format as time string, got: {}",
        elapsed
    );
}

#[test]
fn test_drone_elapsed_with_completed_drone() {
    // Create a status that started 10 minutes ago and completed 2 minutes ago
    let ten_mins_ago = (chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
    let two_mins_ago = (chrono::Utc::now() - chrono::Duration::minutes(2)).to_rfc3339();

    let status = DroneStatus {
        drone: "test-drone".to_string(),
        prd: "test.json".to_string(),
        branch: "test-branch".to_string(),
        worktree: "/tmp/test".to_string(),
        local_mode: false,
        execution_mode: ExecutionMode::AgentTeam,
        backend: "agent_team".to_string(),
        status: DroneState::Completed,
        current_task: None,
        completed: vec![],
        story_times: HashMap::new(),
        total: 0,
        started: ten_mins_ago,
        updated: two_mins_ago,
        error_count: 0,
        last_error: None,
        lead_model: None,
        active_agents: HashMap::new(),
    };

    let elapsed = TuiState::drone_elapsed(&status);
    // Should show ~8 minutes (difference between started and updated)
    assert!(!elapsed.is_empty(), "Elapsed time should not be empty");
    assert!(
        elapsed.contains("m") || elapsed.contains("s"),
        "Should format as time string, got: {}",
        elapsed
    );
}

#[test]
fn test_drone_elapsed_with_invalid_timestamp() {
    // Test with an invalid timestamp
    let status = DroneStatus {
        drone: "test-drone".to_string(),
        prd: "test.json".to_string(),
        branch: "test-branch".to_string(),
        worktree: "/tmp/test".to_string(),
        local_mode: false,
        execution_mode: ExecutionMode::AgentTeam,
        backend: "agent_team".to_string(),
        status: DroneState::InProgress,
        current_task: None,
        completed: vec![],
        story_times: HashMap::new(),
        total: 0,
        started: "not-a-valid-timestamp".to_string(),
        updated: chrono::Utc::now().to_rfc3339(),
        error_count: 0,
        last_error: None,
        lead_model: None,
        active_agents: HashMap::new(),
    };

    let elapsed = TuiState::drone_elapsed(&status);
    // Should return "?" as fallback for unparseable timestamp
    assert_eq!(elapsed, "?", "Should return '?' for invalid timestamp");
}

#[test]
fn test_drone_elapsed_with_empty_timestamp() {
    // Test with an empty timestamp
    let status = DroneStatus {
        drone: "test-drone".to_string(),
        prd: "test.json".to_string(),
        branch: "test-branch".to_string(),
        worktree: "/tmp/test".to_string(),
        local_mode: false,
        execution_mode: ExecutionMode::AgentTeam,
        backend: "agent_team".to_string(),
        status: DroneState::InProgress,
        current_task: None,
        completed: vec![],
        story_times: HashMap::new(),
        total: 0,
        started: "".to_string(),
        updated: chrono::Utc::now().to_rfc3339(),
        error_count: 0,
        last_error: None,
        lead_model: None,
        active_agents: HashMap::new(),
    };

    let elapsed = TuiState::drone_elapsed(&status);
    // Should return "?" as fallback for empty timestamp
    assert_eq!(elapsed, "?", "Should return '?' for empty timestamp");
}
