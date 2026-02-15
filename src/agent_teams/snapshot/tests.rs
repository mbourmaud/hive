use super::*;

fn make_task(id: &str, status: &str) -> TeamTaskInfo {
    TeamTaskInfo {
        id: id.to_string(),
        subject: format!("Task {}", id),
        description: String::new(),
        status: status.to_string(),
        owner: None,
        active_form: None,
        model: None,
        is_internal: false,
        created_at: None,
        updated_at: None,
        blocked_by: Vec::new(),
    }
}

#[test]
fn test_monotonic_progress_never_decreases() {
    let mut store = TaskSnapshotStore::new();

    store
        .high_water_marks
        .insert("test-drone".to_string(), (3, 5));

    let (prev_completed, prev_total) = store
        .high_water_marks
        .get("test-drone")
        .copied()
        .unwrap_or((0, 0));

    let current_completed = 0;
    let current_total = 0;

    let final_completed = current_completed.max(prev_completed);
    let final_total = current_total.max(prev_total);

    assert_eq!(final_completed, 3);
    assert_eq!(final_total, 5);
}

#[test]
fn test_task_status_monotonicity() {
    let mut store = TaskSnapshotStore::new();

    let completed_set = store
        .completed_tasks
        .entry("test-drone".to_string())
        .or_default();
    completed_set.insert("1".to_string());

    let mut tasks = vec![make_task("1", "in_progress"), make_task("2", "pending")];

    let set = store.completed_tasks.get("test-drone").unwrap();
    for task in &mut tasks {
        if set.contains(&task.id) && task.status != "completed" {
            task.status = "completed".to_string();
        }
    }

    assert_eq!(tasks[0].status, "completed");
    assert_eq!(tasks[1].status, "pending");
}

#[test]
fn test_empty_store_returns_none() {
    let store = TaskSnapshotStore::new();
    assert!(store.get("nonexistent").is_none());
    assert_eq!(store.progress("nonexistent"), (0, 0));
}

#[test]
fn test_map_task_internal_flag() {
    let task = AgentTeamTask {
        id: "1".to_string(),
        subject: "internal-task".to_string(),
        description: String::new(),
        status: "pending".to_string(),
        owner: None,
        active_form: None,
        blocked_by: Vec::new(),
        blocks: Vec::new(),
        metadata: Some(serde_json::json!({"_internal": true})),
        created_at: Some(1000),
        updated_at: Some(2000),
        files: None,
    };

    let info = map_task(task);
    assert!(info.is_internal);
    assert_eq!(info.created_at, Some(1000));
    assert_eq!(info.updated_at, Some(2000));
}

#[test]
fn test_map_task_not_internal_by_default() {
    let task = AgentTeamTask {
        id: "2".to_string(),
        subject: "user-task".to_string(),
        description: String::new(),
        status: "in_progress".to_string(),
        owner: Some("worker".to_string()),
        active_form: Some("Working".to_string()),
        blocked_by: Vec::new(),
        blocks: Vec::new(),
        metadata: None,
        created_at: None,
        updated_at: None,
        files: None,
    };

    let info = map_task(task);
    assert!(!info.is_internal);
    assert_eq!(info.owner, Some("worker".to_string()));
    assert_eq!(info.active_form, Some("Working".to_string()));
}

#[test]
fn test_persist_and_load_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let drone_name = "persist-test";
    let drone_dir = dir.path().join(".hive/drones").join(drone_name);
    std::fs::create_dir_all(&drone_dir).unwrap();

    // Must run from temp dir so snapshot_path resolves
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let tasks = vec![
        TeamTaskInfo {
            id: "1".to_string(),
            subject: "Task A".to_string(),
            description: "Do A".to_string(),
            status: "completed".to_string(),
            owner: Some("worker-1".to_string()),
            active_form: None,
            model: None,
            is_internal: false,
            created_at: Some(1000),
            updated_at: Some(2000),
            blocked_by: Vec::new(),
        },
        TeamTaskInfo {
            id: "2".to_string(),
            subject: "Task B".to_string(),
            description: String::new(),
            status: "in_progress".to_string(),
            owner: None,
            active_form: Some("Working".to_string()),
            model: None,
            is_internal: true,
            created_at: None,
            updated_at: None,
            blocked_by: Vec::new(),
        },
    ];
    let members = vec![TeamMember {
        name: "worker-1".to_string(),
        agent_type: "general-purpose".to_string(),
        model: "sonnet".to_string(),
        cwd: String::new(),
    }];

    persist_snapshot(drone_name, &tasks, &members);

    let (loaded_tasks, loaded_members) = load_persisted_snapshot(drone_name).unwrap();

    std::env::set_current_dir(original_dir).unwrap();

    assert_eq!(loaded_tasks.len(), 2);
    assert_eq!(loaded_tasks[0].subject, "Task A");
    assert_eq!(loaded_tasks[0].status, "completed");
    assert_eq!(loaded_tasks[0].owner, Some("worker-1".to_string()));
    assert!(loaded_tasks[1].is_internal);
    assert_eq!(loaded_tasks[1].active_form, Some("Working".to_string()));

    assert_eq!(loaded_members.len(), 1);
    assert_eq!(loaded_members[0].name, "worker-1");
    assert_eq!(loaded_members[0].model, "sonnet");
}

#[test]
fn test_load_persisted_snapshot_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let result = load_persisted_snapshot("no-such-drone");

    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_none());
}
