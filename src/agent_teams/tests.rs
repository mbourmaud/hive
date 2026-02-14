use super::*;

#[test]
fn test_preseed_tasks_filters_work_only() {
    let dir = tempfile::tempdir().unwrap();
    let drone_dir = dir.path().join("drone");
    fs::create_dir_all(&drone_dir).unwrap();

    let tasks = vec![
        StructuredTask {
            number: 1,
            title: "Setup".to_string(),
            body: String::new(),
            task_type: TaskType::Setup,
            model: None,
            parallel: false,
            files: Vec::new(),
            depends_on: Vec::new(),
        },
        StructuredTask {
            number: 2,
            title: "Implement feature".to_string(),
            body: "Do the work".to_string(),
            task_type: TaskType::Work,
            model: Some("sonnet".to_string()),
            parallel: true,
            files: vec!["src/main.rs".to_string()],
            depends_on: Vec::new(),
        },
        StructuredTask {
            number: 3,
            title: "Create PR".to_string(),
            body: String::new(),
            task_type: TaskType::Pr,
            model: None,
            parallel: false,
            files: Vec::new(),
            depends_on: vec![2],
        },
    ];

    let team_name = format!("test-preseed-{}", std::process::id());
    let seeded = preseed_tasks(&team_name, &tasks, &drone_dir).unwrap();

    // Only Work tasks should be seeded
    assert_eq!(seeded.len(), 1);
    assert_eq!(seeded[0].subject, "Implement feature");
    assert_eq!(seeded[0].status, "pending");
    assert_eq!(seeded[0].id, "1");

    // Check metadata
    let meta = seeded[0].metadata.as_ref().unwrap();
    assert_eq!(meta["model"], "sonnet");
    assert_eq!(meta["parallel"], true);
    assert_eq!(meta["files"], serde_json::json!(["src/main.rs"]));
    assert_eq!(meta["plan_number"], 2);

    // Check task file was written
    let task_path = team_tasks_dir(&team_name).join("1.json");
    assert!(task_path.exists());

    // Check events.ndjson was written
    let events = fs::read_to_string(drone_dir.join("events.ndjson")).unwrap();
    assert!(events.contains("TaskCreate"));
    assert!(events.contains("Implement feature"));

    // Cleanup
    let _ = cleanup_team(&team_name);
}

#[test]
fn test_preseed_tasks_maps_depends_on() {
    let dir = tempfile::tempdir().unwrap();
    let drone_dir = dir.path().join("drone");
    fs::create_dir_all(&drone_dir).unwrap();

    let tasks = vec![
        StructuredTask {
            number: 2,
            title: "Task A".to_string(),
            body: String::new(),
            task_type: TaskType::Work,
            model: None,
            parallel: false,
            files: Vec::new(),
            depends_on: Vec::new(),
        },
        StructuredTask {
            number: 3,
            title: "Task B".to_string(),
            body: String::new(),
            task_type: TaskType::Work,
            model: None,
            parallel: false,
            files: Vec::new(),
            depends_on: vec![2],
        },
    ];

    let team_name = format!("test-deps-{}", std::process::id());
    let seeded = preseed_tasks(&team_name, &tasks, &drone_dir).unwrap();

    assert_eq!(seeded.len(), 2);
    // Task B (id=2) depends on Task A (id=1)
    assert!(seeded[0].blocked_by.is_empty());
    assert_eq!(seeded[1].blocked_by, vec!["1"]);

    let _ = cleanup_team(&team_name);
}

#[test]
fn test_agent_team_task_serialization() {
    let task = AgentTeamTask {
        id: "1".to_string(),
        subject: "Create auth middleware".to_string(),
        description: "JWT verification".to_string(),
        status: "pending".to_string(),
        owner: None,
        active_form: None,
        blocked_by: Vec::new(),
        blocks: Vec::new(),
        metadata: None,
        created_at: Some(1000),
        updated_at: Some(1000),
        files: None,
    };

    let json = serde_json::to_string_pretty(&task).unwrap();
    let parsed: AgentTeamTask = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "1");
    assert_eq!(parsed.subject, "Create auth middleware");
    assert_eq!(parsed.status, "pending");
}

#[test]
fn test_claude_code_native_format_deserialization() {
    // Claude Code's TaskCreate writes `title` + `depends_on` instead of
    // Hive's `subject` + `blockedBy`. The serde aliases must handle both.
    let json = r#"{
        "id": "5",
        "title": "Enhanced Edit tool with diff",
        "status": "pending",
        "owner": null,
        "depends_on": ["3", "4"],
        "files": ["src/edit.tsx", "src/diff.tsx"]
    }"#;

    let task: AgentTeamTask = serde_json::from_str(json).unwrap();
    assert_eq!(task.id, "5");
    assert_eq!(task.subject, "Enhanced Edit tool with diff");
    assert_eq!(task.status, "pending");
    assert_eq!(task.blocked_by, vec!["3", "4"]);
    assert_eq!(
        task.files,
        Some(vec!["src/edit.tsx".to_string(), "src/diff.tsx".to_string()])
    );
    // description defaults to empty
    assert!(task.description.is_empty());
}
