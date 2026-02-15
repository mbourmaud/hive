use super::*;

#[test]
fn test_plan_from_markdown() {
    let plan = Plan {
        id: "my-feature".to_string(),
        content: "# My Feature\n\n## Goal\nBuild X\n\n## Requirements\n- Thing A".to_string(),
        target_branch: Some("feature/my-feature".to_string()),
        base_branch: Some("main".to_string()),
        structured_tasks: Vec::new(),
    };

    assert_eq!(plan.id, "my-feature");
    assert_eq!(plan.title(), "My Feature");
    assert!(plan.content.contains("## Goal"));
}

#[test]
fn test_plan_title_fallback() {
    let plan = Plan {
        id: "no-heading".to_string(),
        content: "Just some content without a heading".to_string(),
        target_branch: None,
        base_branch: None,
        structured_tasks: Vec::new(),
    };

    // Falls back to id when no heading is present
    assert_eq!(plan.title(), "no-heading");
}

#[test]
fn test_task_type_display() {
    assert_eq!(TaskType::Setup.to_string(), "setup");
    assert_eq!(TaskType::Pr.to_string(), "pr");
    assert_eq!(TaskType::Work.to_string(), "work");
}

#[test]
fn test_structured_task_defaults() {
    let task = StructuredTask {
        number: 1,
        title: "Test task".to_string(),
        body: String::new(),
        task_type: TaskType::Work,
        model: None,
        parallel: false,
        files: Vec::new(),
        depends_on: Vec::new(),
    };

    assert_eq!(task.number, 1);
    assert_eq!(task.task_type, TaskType::Work);
    assert!(!task.parallel);
    assert!(task.model.is_none());
}

#[test]
fn test_worker_name_generation() {
    let make = |title: &str| StructuredTask {
        number: 1,
        title: title.to_string(),
        body: String::new(),
        task_type: TaskType::Work,
        model: None,
        parallel: false,
        files: Vec::new(),
        depends_on: Vec::new(),
    };

    // Stops at word boundary when would exceed 20 chars
    assert_eq!(
        make("Add JWT authentication middleware").worker_name(),
        "jwt-authentication"
    );
    assert_eq!(
        make("Implement user registration flow").worker_name(),
        "user-registration"
    );
    assert_eq!(
        make("Write integration tests for API").worker_name(),
        "integration-tests"
    );
    assert_eq!(
        make("Create database schema").worker_name(),
        "database-schema"
    );
    // Fallback to worker-N when all words are stop words
    assert_eq!(make("Set up and configure").worker_name(), "worker-1");
}

#[test]
fn test_legacy_json_plan_conversion() {
    let json = r###"{
        "id": "my-feature",
        "title": "My Feature",
        "version": "1.0.0",
        "target_branch": "feature/my-feature",
        "base_branch": "main",
        "plan": "## Goal\nBuild X\n\n## Requirements\n- Thing A",
        "tasks": [
            {"title": "Task A", "description": "Do A"}
        ]
    }"###;

    let legacy: LegacyJsonPlan = serde_json::from_str(json).unwrap();
    let plan: Plan = legacy.into();
    assert_eq!(plan.id, "my-feature");
    assert_eq!(plan.title(), "My Feature");
    assert!(plan.content.contains("## Goal"));
    assert!(plan.content.contains("Build X"));
}

#[test]
fn test_legacy_json_plan_empty_plan_field() {
    let json = r#"{
        "id": "minimal",
        "title": "Minimal PRD",
        "plan": ""
    }"#;

    let legacy: LegacyJsonPlan = serde_json::from_str(json).unwrap();
    let plan: Plan = legacy.into();
    assert_eq!(plan.id, "minimal");
    assert_eq!(plan.content, "# Minimal PRD");
}

#[test]
fn test_parse_drone_status() {
    let json = r#"{
        "drone": "test-drone",
        "prd": "test-prd.json",
        "branch": "hive/test",
        "worktree": "/path/to/worktree",
        "local_mode": false,
        "status": "in_progress",
        "current_story": "TEST-001",
        "completed": [],
        "story_times": {},
        "total": 5,
        "started": "2024-01-01T00:00:00Z",
        "updated": "2024-01-01T00:00:00Z",
        "error_count": 0,
        "last_error_story": null
    }"#;

    let status: DroneStatus = serde_json::from_str(json).unwrap();
    assert_eq!(status.drone, "test-drone");
    assert_eq!(status.status, DroneState::InProgress);
    assert_eq!(status.current_task, Some("TEST-001".to_string()));
}

#[test]
fn test_parse_hive_config() {
    let json = r#"{
        "version": "1.0.0",
        "project": "test-project",
        "worktree_base": "/tmp/hive",
        "default_model": "sonnet",
        "timestamp": "2024-01-01T00:00:00Z"
    }"#;

    let config: HiveConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.version, "1.0.0");
    assert_eq!(config.project, Some("test-project".to_string()));
}

#[test]
fn test_drone_state_display() {
    assert_eq!(DroneState::Starting.to_string(), "starting");
    assert_eq!(DroneState::Resuming.to_string(), "resuming");
    assert_eq!(DroneState::InProgress.to_string(), "in_progress");
    assert_eq!(DroneState::Completed.to_string(), "completed");
    assert_eq!(DroneState::Error.to_string(), "error");
    assert_eq!(DroneState::Stopped.to_string(), "stopped");
    assert_eq!(DroneState::Cleaning.to_string(), "cleaning");
    assert_eq!(DroneState::Zombie.to_string(), "zombie");
}

#[test]
fn test_default_config() {
    let config = HiveConfig::default();
    assert_eq!(config.version, "1.0.0");
    assert_eq!(config.default_model, Some("sonnet".to_string()));
    assert!(config.project.is_none());
}
