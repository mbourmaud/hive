use hive_lib::types::{DroneState, DroneStatus, ExecutionMode, HiveConfig, StoryTiming};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn create_hive_structure(temp_dir: &TempDir) -> PathBuf {
    let hive_dir = temp_dir.path().join(".hive");
    let drones_dir = hive_dir.join("drones");
    let plans_dir = hive_dir.join("plans");
    // Also create prds dir for backward compat tests
    let prds_dir = hive_dir.join("prds");

    fs::create_dir_all(&drones_dir).unwrap();
    fs::create_dir_all(&plans_dir).unwrap();
    fs::create_dir_all(&prds_dir).unwrap();

    hive_dir
}

fn create_test_plan(plans_dir: &Path, plan_name: &str) {
    let content = format!(
        "# {} Title\n\n## Goal\nThis is a test plan.\n\n## Tasks\n- Test task: A test task\n",
        plan_name
    );
    let plan_path = plans_dir.join(format!("{}.md", plan_name));
    fs::write(&plan_path, content).unwrap();
}

fn create_test_drone(drones_dir: &Path, drone_name: &str, prd_name: &str) {
    let drone_dir = drones_dir.join(drone_name);
    fs::create_dir_all(&drone_dir).unwrap();

    let status = DroneStatus {
        drone: drone_name.to_string(),
        prd: format!("{}.json", prd_name),
        branch: format!("hive/{}", drone_name),
        worktree: format!("/tmp/{}", drone_name),
        local_mode: false,
        execution_mode: ExecutionMode::AgentTeam,
        backend: "agent_team".to_string(),
        status: DroneState::InProgress,
        current_task: Some("TASK-001".to_string()),
        completed: vec![],
        story_times: HashMap::new(),
        total: 5,
        started: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T01:00:00Z".to_string(),
        error_count: 0,
        last_error: None,
        lead_model: None,
        active_agents: HashMap::new(),
    };

    let status_json = serde_json::to_string_pretty(&status).unwrap();
    fs::write(drone_dir.join("status.json"), status_json).unwrap();

    // Create activity log
    fs::write(
        drone_dir.join("activity.log"),
        "[10:00:00] 🔨 Starting TASK-001\n",
    )
    .unwrap();
}

#[test]
fn test_complete_hive_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let hive_dir = create_hive_structure(&temp_dir);

    // Create config
    let config = HiveConfig::default();
    let config_json = serde_json::to_string_pretty(&config).unwrap();
    fs::write(hive_dir.join("config.json"), config_json).unwrap();

    // Create plan (markdown)
    create_test_plan(&hive_dir.join("plans"), "test-plan");

    // Create drone
    create_test_drone(&hive_dir.join("drones"), "test-drone", "test-plan");

    // Verify structure
    assert!(hive_dir.exists());
    assert!(hive_dir.join("config.json").exists());
    assert!(hive_dir.join("plans").join("test-plan.md").exists());
    assert!(hive_dir
        .join("drones")
        .join("test-drone")
        .join("status.json")
        .exists());
    assert!(hive_dir
        .join("drones")
        .join("test-drone")
        .join("activity.log")
        .exists());
}

#[test]
fn test_backward_compatible_status_json() {
    let temp_dir = TempDir::new().unwrap();
    let hive_dir = create_hive_structure(&temp_dir);

    // Create an old-style status.json (from bash version)
    let old_status_json = r#"{
        "drone": "legacy-drone",
        "prd": "legacy-prd.json",
        "branch": "hive/legacy",
        "worktree": "/tmp/legacy",
        "local_mode": false,
        "status": "in_progress",
        "current_story": "LEGACY-001",
        "completed": ["LEGACY-000"],
        "story_times": {
            "LEGACY-000": {
                "started": "2024-01-01T00:00:00Z",
                "completed": "2024-01-01T01:00:00Z"
            }
        },
        "total": 10,
        "started": "2024-01-01T00:00:00Z",
        "updated": "2024-01-01T01:00:00Z",
        "error_count": 0,
        "last_error_story": null
    }"#;

    let drone_dir = hive_dir.join("drones").join("legacy-drone");
    fs::create_dir_all(&drone_dir).unwrap();
    fs::write(drone_dir.join("status.json"), old_status_json).unwrap();

    // Try to parse it
    let contents = fs::read_to_string(drone_dir.join("status.json")).unwrap();
    let status: Result<DroneStatus, _> = serde_json::from_str(&contents);

    assert!(status.is_ok());
    let status = status.unwrap();
    assert_eq!(status.drone, "legacy-drone");
    assert_eq!(status.status, DroneState::InProgress);
    assert_eq!(status.completed.len(), 1);
}

#[test]
fn test_existing_prd_compatibility() {
    let temp_dir = TempDir::new().unwrap();
    let hive_dir = create_hive_structure(&temp_dir);

    // Create a PRD similar to existing ones (legacy JSON format)
    let prd_json = r#"{
        "id": "existing-prd",
        "title": "Existing PRD Title",
        "description": "This is an existing PRD from the bash version",
        "version": "1.0.0",
        "created_at": "2024-01-01T00:00:00Z",
        "target_platforms": ["macos", "linux"],
        "target_branch": "main",
        "stories": [
            {
                "id": "STORY-001",
                "title": "First Story",
                "description": "Description of first story",
                "acceptance_criteria": ["Criterion 1", "Criterion 2"],
                "definition_of_done": ["DoD 1", "DoD 2"],
                "verification_commands": ["echo test"],
                "notes": "Some notes"
            }
        ]
    }"#;

    let prds_dir = hive_dir.join("prds");
    fs::write(prds_dir.join("existing-prd.json"), prd_json).unwrap();

    // LegacyJsonPlan can still parse old JSON (ignores unknown fields)
    let contents = fs::read_to_string(prds_dir.join("existing-prd.json")).unwrap();
    let legacy: Result<hive_lib::types::LegacyJsonPlan, _> = serde_json::from_str(&contents);

    assert!(legacy.is_ok());
    let legacy = legacy.unwrap();
    assert_eq!(legacy.id, "existing-prd");
    assert!(legacy.plan.is_empty()); // No plan field in old JSON, defaults to empty string
}

#[test]
fn test_config_compatibility() {
    let temp_dir = TempDir::new().unwrap();
    let hive_dir = create_hive_structure(&temp_dir);

    // Create a config file
    let config_json = r#"{
        "version": "1.0.0",
        "project": "test-project",
        "worktree_base": "/tmp/hive",
        "default_model": "sonnet",
        "timestamp": "2024-01-01T00:00:00Z"
    }"#;

    fs::write(hive_dir.join("config.json"), config_json).unwrap();

    // Try to parse it
    let contents = fs::read_to_string(hive_dir.join("config.json")).unwrap();
    let config: Result<HiveConfig, _> = serde_json::from_str(&contents);

    assert!(config.is_ok());
    let config = config.unwrap();
    assert_eq!(config.version, "1.0.0");
    assert_eq!(config.project, Some("test-project".to_string()));
}

#[test]
fn test_story_timing_structure() {
    let timing = StoryTiming {
        started: Some("2024-01-01T00:00:00Z".to_string()),
        completed: Some("2024-01-01T01:00:00Z".to_string()),
    };

    let json = serde_json::to_string(&timing).unwrap();
    let parsed: StoryTiming = serde_json::from_str(&json).unwrap();

    assert_eq!(timing.started, parsed.started);
    assert_eq!(timing.completed, parsed.completed);
}

#[test]
fn test_multiple_drones_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let hive_dir = create_hive_structure(&temp_dir);

    // Create multiple plans and drones
    for i in 1..=3 {
        let plan_name = format!("plan-{}", i);
        let drone_name = format!("drone-{}", i);

        create_test_plan(&hive_dir.join("plans"), &plan_name);
        create_test_drone(&hive_dir.join("drones"), &drone_name, &plan_name);
    }

    // Verify all exist
    for i in 1..=3 {
        let plan_path = hive_dir.join("plans").join(format!("plan-{}.md", i));
        let drone_path = hive_dir
            .join("drones")
            .join(format!("drone-{}", i))
            .join("status.json");

        assert!(plan_path.exists());
        assert!(drone_path.exists());
    }
}

#[test]
fn test_drone_state_transitions() {
    // Test all possible drone states
    for state in [
        DroneState::Starting,
        DroneState::InProgress,
        DroneState::Completed,
        DroneState::Error,
        DroneState::Stopped,
    ] {
        let status = DroneStatus {
            drone: "test".to_string(),
            prd: "test.json".to_string(),
            branch: "hive/test".to_string(),
            worktree: "/tmp/test".to_string(),
            local_mode: false,
            execution_mode: ExecutionMode::AgentTeam,
            backend: "agent_team".to_string(),
            status: state.clone(),
            current_task: None,
            completed: vec![],
            story_times: HashMap::new(),
            total: 1,
            started: "2024-01-01T00:00:00Z".to_string(),
            updated: "2024-01-01T01:00:00Z".to_string(),
            error_count: 0,
            last_error: None,
            lead_model: None,
            active_agents: HashMap::new(),
        };

        let json = serde_json::to_string(&status).unwrap();
        let parsed: DroneStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status.status, parsed.status);
    }
}
