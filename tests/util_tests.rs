use hive_lib::types::{DroneState, DroneStatus, ExecutionMode, StoryTiming};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn create_test_environment() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let hive_dir = temp_dir.path().join(".hive");
    let drones_dir = hive_dir.join("drones");

    fs::create_dir_all(&drones_dir).unwrap();

    temp_dir
}

fn create_test_drone(
    temp_dir: &TempDir,
    drone_name: &str,
    status: DroneState,
    completed: usize,
    total: usize,
) {
    let drone_dir = temp_dir
        .path()
        .join(".hive")
        .join("drones")
        .join(drone_name);
    fs::create_dir_all(&drone_dir).unwrap();

    let mut completed_stories = Vec::new();
    let mut story_times = HashMap::new();

    for i in 0..completed {
        let story_id = format!("TEST-{:03}", i + 1);
        completed_stories.push(story_id.clone());
        story_times.insert(
            story_id,
            StoryTiming {
                started: Some("2024-01-01T00:00:00Z".to_string()),
                completed: Some("2024-01-01T01:00:00Z".to_string()),
            },
        );
    }

    let drone_status = DroneStatus {
        drone: drone_name.to_string(),
        prd: "test-prd.json".to_string(),
        branch: format!("hive/{}", drone_name),
        worktree: format!("/tmp/{}", drone_name),
        local_mode: false,
        execution_mode: ExecutionMode::Worktree,
        backend: "native".to_string(),
        status,
        current_story: None,
        completed: completed_stories,
        story_times,
        total,
        started: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T01:00:00Z".to_string(),
        error_count: 0,
        last_error_story: None,
        blocked_reason: None,
        blocked_questions: vec![],
        awaiting_human: false,
        active_agents: HashMap::new(),
    };

    let status_json = serde_json::to_string_pretty(&drone_status).unwrap();
    fs::write(drone_dir.join("status.json"), status_json).unwrap();
}

#[test]
fn test_list_no_drones() {
    let temp_dir = create_test_environment();

    // Use absolute path instead of changing current directory (avoids race conditions)
    let drones_dir = temp_dir.path().join(".hive").join("drones");
    assert!(drones_dir.exists());
    assert_eq!(fs::read_dir(&drones_dir).unwrap().count(), 0);
}

#[test]
fn test_list_single_drone() {
    let temp_dir = create_test_environment();
    create_test_drone(&temp_dir, "test-drone", DroneState::InProgress, 5, 10);

    // Use absolute path instead of changing current directory (avoids race conditions)
    let status_path = temp_dir
        .path()
        .join(".hive")
        .join("drones")
        .join("test-drone")
        .join("status.json");

    let contents = fs::read_to_string(&status_path).unwrap();
    let status: DroneStatus = serde_json::from_str(&contents).unwrap();

    assert_eq!(status.drone, "test-drone");
    assert_eq!(status.status, DroneState::InProgress);
    assert_eq!(status.completed.len(), 5);
    assert_eq!(status.total, 10);
}

#[test]
fn test_list_multiple_drones() {
    let temp_dir = create_test_environment();
    create_test_drone(&temp_dir, "drone-1", DroneState::InProgress, 3, 10);
    create_test_drone(&temp_dir, "drone-2", DroneState::Completed, 5, 5);
    create_test_drone(&temp_dir, "drone-3", DroneState::Error, 1, 8);

    // Use absolute path instead of changing current directory (avoids race conditions)
    let drones_dir = temp_dir.path().join(".hive").join("drones");
    let drone_count = fs::read_dir(&drones_dir).unwrap().count();
    assert_eq!(drone_count, 3);

    // Verify each drone
    for drone_name in &["drone-1", "drone-2", "drone-3"] {
        let status_path = drones_dir.join(drone_name).join("status.json");
        assert!(status_path.exists());
    }
}

#[test]
fn test_version_format() {
    // Version should be in semver format
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        version.split('.').count() >= 2,
        "Version should be in semver format"
    );
}

#[test]
fn test_drone_states() {
    let temp_dir = create_test_environment();

    // Test all drone states
    create_test_drone(&temp_dir, "starting-drone", DroneState::Starting, 0, 5);
    create_test_drone(&temp_dir, "inprogress-drone", DroneState::InProgress, 2, 5);
    create_test_drone(&temp_dir, "completed-drone", DroneState::Completed, 5, 5);
    create_test_drone(&temp_dir, "error-drone", DroneState::Error, 1, 5);
    create_test_drone(&temp_dir, "blocked-drone", DroneState::Blocked, 2, 5);
    create_test_drone(&temp_dir, "stopped-drone", DroneState::Stopped, 3, 5);

    // Use absolute path instead of changing current directory (avoids race conditions)
    let drones_dir = temp_dir.path().join(".hive").join("drones");
    let drone_count = fs::read_dir(&drones_dir).unwrap().count();
    assert_eq!(drone_count, 6);
}

#[test]
fn test_progress_calculation() {
    let temp_dir = create_test_environment();
    create_test_drone(&temp_dir, "half-done", DroneState::InProgress, 5, 10);

    // Use absolute path instead of changing current directory (avoids race conditions)
    let status_path = temp_dir
        .path()
        .join(".hive")
        .join("drones")
        .join("half-done")
        .join("status.json");

    let contents = fs::read_to_string(&status_path).unwrap();
    let status: DroneStatus = serde_json::from_str(&contents).unwrap();

    let percentage = (status.completed.len() as f32 / status.total as f32 * 100.0) as u32;
    assert_eq!(percentage, 50);
}
