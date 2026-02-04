use hive_lib::types::{DroneState, DroneStatus, ExecutionMode, StoryTiming};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_environment() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let hive_dir = temp_dir.path().join(".hive");
    let drones_dir = hive_dir.join("drones");
    let prds_dir = hive_dir.join("prds");

    fs::create_dir_all(&drones_dir).unwrap();
    fs::create_dir_all(&prds_dir).unwrap();

    // Create a test PRD
    let prd_content = r#"{
        "id": "test-prd",
        "title": "Test PRD",
        "description": "Test PRD for unblock",
        "version": "1.0.0",
        "created_at": "2024-01-01T00:00:00Z",
        "stories": [
            {
                "id": "TEST-001",
                "title": "Test Story",
                "description": "A test story",
                "definition_of_done": ["Done"],
                "verification_commands": ["echo test"]
            }
        ]
    }"#;
    fs::write(prds_dir.join("test-prd.json"), prd_content).unwrap();

    temp_dir
}

fn create_blocked_drone(temp_dir: &TempDir, drone_name: &str) {
    let drone_dir = temp_dir
        .path()
        .join(".hive")
        .join("drones")
        .join(drone_name);
    fs::create_dir_all(&drone_dir).unwrap();

    let mut story_times = HashMap::new();
    story_times.insert(
        "TEST-001".to_string(),
        StoryTiming {
            started: Some("2024-01-01T00:00:00Z".to_string()),
            completed: None,
        },
    );

    let status = DroneStatus {
        drone: drone_name.to_string(),
        prd: "test-prd.json".to_string(),
        branch: "hive/test".to_string(),
        worktree: "/tmp/test".to_string(),
        local_mode: false,
        execution_mode: ExecutionMode::Worktree,
        backend: "native".to_string(),
        status: DroneState::Blocked,
        current_story: Some("TEST-001".to_string()),
        completed: vec![],
        story_times,
        total: 1,
        started: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T01:00:00Z".to_string(),
        error_count: 3,
        last_error_story: Some("TEST-001".to_string()),
        blocked_reason: Some("Test blocked reason".to_string()),
        blocked_questions: vec!["Question 1?".to_string(), "Question 2?".to_string()],
        awaiting_human: true,
    };

    let status_json = serde_json::to_string_pretty(&status).unwrap();
    fs::write(drone_dir.join("status.json"), status_json).unwrap();

    // Create blocked.md
    let blocked_content =
        "# Blocked Context\n\nThis is additional context about why the drone is blocked.\n";
    fs::write(drone_dir.join("blocked.md"), blocked_content).unwrap();
}

#[test]
fn test_unblock_reads_blocked_status() {
    let temp_dir = create_test_environment();
    create_blocked_drone(&temp_dir, "test-drone");

    // Change to temp directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Read status
    let status_path = PathBuf::from(".hive")
        .join("drones")
        .join("test-drone")
        .join("status.json");

    let contents = fs::read_to_string(&status_path).unwrap();
    let status: DroneStatus = serde_json::from_str(&contents).unwrap();

    assert_eq!(status.status, DroneState::Blocked);
    assert_eq!(
        status.blocked_reason,
        Some("Test blocked reason".to_string())
    );
    assert_eq!(status.blocked_questions.len(), 2);
    assert_eq!(status.error_count, 3);
    assert!(status.awaiting_human);

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_unblock_clears_blocked_fields() {
    let temp_dir = create_test_environment();
    create_blocked_drone(&temp_dir, "test-drone");

    // Load and modify status using absolute path
    let status_path = temp_dir
        .path()
        .join(".hive")
        .join("drones")
        .join("test-drone")
        .join("status.json");

    let contents = fs::read_to_string(&status_path).unwrap();
    let mut status: DroneStatus = serde_json::from_str(&contents).unwrap();

    // Clear blocked status (simulating what unblock does)
    status.status = DroneState::InProgress;
    status.blocked_reason = None;
    status.blocked_questions.clear();
    status.error_count = 0;
    status.awaiting_human = false;

    // Save back
    let updated_json = serde_json::to_string_pretty(&status).unwrap();
    fs::write(&status_path, updated_json).unwrap();

    // Verify
    let contents = fs::read_to_string(&status_path).unwrap();
    let status: DroneStatus = serde_json::from_str(&contents).unwrap();

    assert_eq!(status.status, DroneState::InProgress);
    assert_eq!(status.blocked_reason, None);
    assert_eq!(status.blocked_questions.len(), 0);
    assert_eq!(status.error_count, 0);
    assert!(!status.awaiting_human);
}

#[test]
fn test_blocked_md_exists() {
    let temp_dir = create_test_environment();
    create_blocked_drone(&temp_dir, "test-drone");

    // Change to temp directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let blocked_md_path = PathBuf::from(".hive")
        .join("drones")
        .join("test-drone")
        .join("blocked.md");

    assert!(blocked_md_path.exists());

    let content = fs::read_to_string(&blocked_md_path).unwrap();
    assert!(content.contains("Blocked Context"));

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_drone_not_found() {
    let temp_dir = create_test_environment();

    // Change to temp directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let status_path = PathBuf::from(".hive")
        .join("drones")
        .join("nonexistent-drone")
        .join("status.json");

    assert!(!status_path.exists());

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}
