use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, Seek, SeekFrom};
use std::path::PathBuf;

/// Events emitted by Claude Code hooks into `.hive/drones/{name}/events.ndjson`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum HiveEvent {
    TaskCreate {
        ts: String,
        subject: String,
        #[serde(default)]
        description: String,
    },
    TaskUpdate {
        ts: String,
        task_id: String,
        #[serde(default)]
        status: String,
        #[serde(default)]
        owner: Option<String>,
    },
    Message {
        ts: String,
        #[serde(default)]
        recipient: String,
        #[serde(default)]
        summary: String,
    },
    TaskDone {
        ts: String,
        task_id: String,
        #[serde(default)]
        subject: String,
        #[serde(default)]
        agent: Option<String>,
    },
    Idle {
        ts: String,
        agent: String,
    },
    Stop {
        ts: String,
    },
    Start {
        ts: String,
        #[serde(default)]
        model: String,
    },
    /// Agent spawned via PreToolUse Task matcher
    AgentSpawn {
        ts: String,
        name: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        subagent_type: Option<String>,
    },
    /// Subagent started (SubagentStart hook)
    SubagentStart {
        ts: String,
        agent_id: String,
        #[serde(default)]
        agent_type: Option<String>,
    },
    /// Subagent stopped (SubagentStop hook)
    SubagentStop {
        ts: String,
        agent_id: String,
        #[serde(default)]
        agent_type: Option<String>,
    },
    /// Tool completed (PostToolUse hook)
    ToolDone {
        ts: String,
        tool: String,
        #[serde(default)]
        tool_use_id: Option<String>,
    },
    /// Full snapshot of all todos from TodoWrite (full-replace semantics)
    TodoSnapshot {
        ts: String,
        todos: Vec<TodoItem>,
    },
}

/// A single todo item from Claude Code's TodoWrite tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    #[serde(default)]
    pub status: String,
    #[serde(default, rename = "activeForm")]
    pub active_form: Option<String>,
}

impl HiveEvent {
    pub fn timestamp(&self) -> &str {
        match self {
            HiveEvent::TaskCreate { ts, .. } => ts,
            HiveEvent::TaskUpdate { ts, .. } => ts,
            HiveEvent::Message { ts, .. } => ts,
            HiveEvent::TaskDone { ts, .. } => ts,
            HiveEvent::Idle { ts, .. } => ts,
            HiveEvent::Stop { ts } => ts,
            HiveEvent::Start { ts, .. } => ts,
            HiveEvent::AgentSpawn { ts, .. } => ts,
            HiveEvent::SubagentStart { ts, .. } => ts,
            HiveEvent::SubagentStop { ts, .. } => ts,
            HiveEvent::ToolDone { ts, .. } => ts,
            HiveEvent::TodoSnapshot { ts, .. } => ts,
        }
    }
}

/// A task reconstructed from events.ndjson
pub struct EventTask {
    pub task_id: String,
    pub subject: String,
    pub status: String,
    pub owner: Option<String>,
    pub description: String,
    pub agent_active_form: Option<String>,
}

/// Normalize a string for fuzzy matching: lowercase, replace separators with hyphens.
fn normalize_for_match(s: &str) -> String {
    s.to_lowercase().replace(['_', ' '], "-")
}

/// Try to match an agent spawn name to a task subject.
/// Returns true if there's a fuzzy match (exact, contains, or normalized).
fn names_match(task_subject: &str, agent_name: &str) -> bool {
    let task_norm = normalize_for_match(task_subject);
    let agent_norm = normalize_for_match(agent_name);

    // Exact (normalized)
    if task_norm == agent_norm {
        return true;
    }
    // Contains
    if task_norm.contains(&agent_norm) || agent_norm.contains(&task_norm) {
        return true;
    }
    false
}

/// Reconstruct structured task data from events.ndjson by replaying events.
///
/// **Dual-mode reconstruction:**
/// - When `TaskCreate` events exist (structured plans with Agent Teams), they form the
///   primary task list. `TodoSnapshot` events are ignored for the main list but can
///   contribute agent activity status. `AgentSpawn` events are correlated by name
///   to assign agents to main tasks.
/// - When NO `TaskCreate` events exist (freeform plans), falls back to `TodoSnapshot`-based
///   reconstruction (last snapshot wins).
pub fn reconstruct_tasks(drone_name: &str) -> Vec<EventTask> {
    let path = PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("events.ndjson");

    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Collect all events first
    let mut main_tasks: Vec<EventTask> = Vec::new();
    let mut create_counter: usize = 0;
    let mut agent_spawns: Vec<String> = Vec::new(); // agent names from AgentSpawn events
    let mut last_todo_snapshot: Option<Vec<TodoItem>> = None;
    let mut has_task_creates = false;

    // Task updates/dones to apply after (they reference task_id)
    let mut task_updates: Vec<(String, String, Option<String>)> = Vec::new(); // (task_id, status, owner)
    let mut task_dones: Vec<(String, String)> = Vec::new(); // (task_id, subject)

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<HiveEvent>(trimmed) {
            Ok(event) => match event {
                HiveEvent::TaskCreate {
                    subject,
                    description,
                    ..
                } => {
                    has_task_creates = true;
                    create_counter += 1;
                    let id = create_counter.to_string();
                    main_tasks.push(EventTask {
                        task_id: id,
                        subject,
                        status: "pending".to_string(),
                        owner: None,
                        description,
                        agent_active_form: None,
                    });
                }
                HiveEvent::TaskUpdate {
                    task_id,
                    status,
                    owner,
                    ..
                } => {
                    task_updates.push((task_id, status, owner));
                }
                HiveEvent::TaskDone {
                    task_id, subject, ..
                } => {
                    task_dones.push((task_id, subject));
                }
                HiveEvent::AgentSpawn { name, .. } => {
                    if !agent_spawns.contains(&name) {
                        agent_spawns.push(name);
                    }
                }
                HiveEvent::TodoSnapshot { todos, .. } => {
                    last_todo_snapshot = Some(todos);
                }
                _ => {}
            },
            Err(_) => {
                eprintln!(
                    "[hive] Malformed event line in {}: {}",
                    path.display(),
                    &trimmed[..trimmed.len().min(100)]
                );
            }
        }
    }

    if has_task_creates {
        // === Structured mode: TaskCreate-based main tasks ===

        // Apply TaskUpdate events
        for (task_id, status, owner) in task_updates {
            if let Some(task) = main_tasks.iter_mut().find(|t| t.task_id == task_id) {
                if !status.is_empty() {
                    task.status = status;
                }
                if owner.is_some() {
                    task.owner = owner;
                }
            }
        }

        // Apply TaskDone events
        for (task_id, subject) in task_dones {
            if let Some(task) = main_tasks.iter_mut().find(|t| t.task_id == task_id) {
                task.status = "completed".to_string();
                if !subject.is_empty() {
                    task.subject = subject;
                }
            }
        }

        // Correlate AgentSpawn names with main tasks
        for agent_name in &agent_spawns {
            // Find best matching task that doesn't already have an agent
            if let Some(task) = main_tasks
                .iter_mut()
                .find(|t| t.owner.is_none() && names_match(&t.subject, agent_name))
            {
                task.owner = Some(agent_name.clone());
            }
        }

        // Extract agent activity from the last TodoSnapshot
        // Each agent's in_progress item active_form is used as status
        if let Some(ref todos) = last_todo_snapshot {
            for todo in todos {
                if todo.status == "in_progress" {
                    if let Some(ref active_form) = todo.active_form {
                        // Try to match this todo to a main task's agent
                        // The todo.content might match the agent name or the task subject
                        for task in main_tasks.iter_mut() {
                            if let Some(ref agent) = task.owner {
                                // Check if this todo belongs to this agent:
                                // agent's own subtask is in_progress
                                if task.agent_active_form.is_none() && task.status != "completed" {
                                    // Use name correlation: if the todo content references
                                    // the agent's work area, attribute it
                                    if names_match(&todo.content, agent)
                                        || names_match(&todo.content, &task.subject)
                                    {
                                        task.agent_active_form = Some(active_form.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        main_tasks
    } else {
        // === Freeform fallback: TodoSnapshot-based (last snapshot wins) ===
        let mut task_map: HashMap<String, EventTask> = HashMap::new();

        if let Some(todos) = last_todo_snapshot {
            let mut counter: usize = 0;
            for todo in todos {
                counter += 1;
                let id = counter.to_string();
                let status = if todo.status.is_empty() {
                    "pending".to_string()
                } else {
                    todo.status
                };
                task_map.insert(
                    id.clone(),
                    EventTask {
                        task_id: id,
                        subject: todo.content,
                        status,
                        owner: None,
                        description: String::new(),
                        agent_active_form: todo.active_form,
                    },
                );
            }
        }

        // Also apply any TaskUpdate/TaskDone on top of snapshot
        for (task_id, status, owner) in task_updates {
            if let Some(task) = task_map.get_mut(&task_id) {
                if !status.is_empty() {
                    task.status = status;
                }
                if owner.is_some() {
                    task.owner = owner;
                }
            }
        }
        for (task_id, subject) in task_dones {
            let entry = task_map
                .entry(task_id.clone())
                .or_insert_with(|| EventTask {
                    task_id,
                    subject: subject.clone(),
                    status: "completed".to_string(),
                    owner: None,
                    description: String::new(),
                    agent_active_form: None,
                });
            entry.status = "completed".to_string();
            if !subject.is_empty() {
                entry.subject = subject;
            }
        }

        let mut tasks: Vec<EventTask> = task_map.into_values().collect();
        tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
        tasks
    }
}

/// Reconstruct progress from events.ndjson by replaying TaskCreate/TaskUpdate events.
/// Returns (completed_count, total_count).
pub fn reconstruct_progress(drone_name: &str) -> (usize, usize) {
    let tasks = reconstruct_tasks(drone_name);
    let total = tasks.len();
    let completed = tasks.iter().filter(|t| t.status == "completed").count();
    (completed, total)
}

/// Check if a Stop event exists in the drone's events.ndjson.
pub fn has_stop_event(drone_name: &str) -> bool {
    let path = PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("events.ndjson");

    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    for line in contents.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<HiveEvent>(trimmed) {
            if matches!(event, HiveEvent::Stop { .. }) {
                return true;
            }
        }
    }

    false
}

/// Incrementally reads events from an ndjson file, tracking byte offset.
pub struct EventReader {
    offset: u64,
    path: PathBuf,
}

impl EventReader {
    pub fn new(drone_name: &str) -> Self {
        let path = PathBuf::from(".hive/drones")
            .join(drone_name)
            .join("events.ndjson");
        EventReader { offset: 0, path }
    }

    /// Check if the events file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Read new events since last call. Returns empty vec if no new data.
    pub fn read_new(&mut self) -> Vec<HiveEvent> {
        let meta = match fs::metadata(&self.path) {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        let file_size = meta.len();
        if file_size < self.offset {
            // File was truncated (e.g. recreated), reset offset
            eprintln!(
                "[hive] Events file truncated for {}, resetting",
                self.path.display()
            );
            self.offset = 0;
        }
        if file_size <= self.offset {
            return Vec::new();
        }

        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let mut reader = std::io::BufReader::new(file);
        if self.offset > 0 && reader.seek(SeekFrom::Start(self.offset)).is_err() {
            return Vec::new();
        }

        let mut events = Vec::new();
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(n) => {
                    self.offset += n as u64;
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<HiveEvent>(trimmed) {
                        Ok(event) => events.push(event),
                        Err(_) => {
                            eprintln!(
                                "[hive] Malformed event line in {}: {}",
                                self.path.display(),
                                &trimmed[..trimmed.len().min(100)]
                            );
                        }
                    }
                }
                Err(_) => break,
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_task_create_event() {
        let json = r#"{"event":"TaskCreate","ts":"2025-01-15T10:00:00Z","subject":"Implement auth","description":"Add JWT auth"}"#;
        let event: HiveEvent = serde_json::from_str(json).unwrap();
        match event {
            HiveEvent::TaskCreate {
                ts,
                subject,
                description,
            } => {
                assert_eq!(ts, "2025-01-15T10:00:00Z");
                assert_eq!(subject, "Implement auth");
                assert_eq!(description, "Add JWT auth");
            }
            _ => panic!("Expected TaskCreate"),
        }
    }

    #[test]
    fn test_parse_task_update_event() {
        let json = r#"{"event":"TaskUpdate","ts":"2025-01-15T10:01:00Z","task_id":"1","status":"in_progress","owner":"researcher"}"#;
        let event: HiveEvent = serde_json::from_str(json).unwrap();
        match event {
            HiveEvent::TaskUpdate {
                ts,
                task_id,
                status,
                owner,
            } => {
                assert_eq!(ts, "2025-01-15T10:01:00Z");
                assert_eq!(task_id, "1");
                assert_eq!(status, "in_progress");
                assert_eq!(owner, Some("researcher".to_string()));
            }
            _ => panic!("Expected TaskUpdate"),
        }
    }

    #[test]
    fn test_parse_stop_event() {
        let json = r#"{"event":"Stop","ts":"2025-01-15T10:05:00Z"}"#;
        let event: HiveEvent = serde_json::from_str(json).unwrap();
        match event {
            HiveEvent::Stop { ts } => assert_eq!(ts, "2025-01-15T10:05:00Z"),
            _ => panic!("Expected Stop"),
        }
    }

    #[test]
    fn test_event_reader_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.ndjson");

        // Write some events
        let mut file = fs::File::create(&events_path).unwrap();
        writeln!(
            file,
            r#"{{"event":"Start","ts":"2025-01-15T10:00:00Z","model":"opus"}}"#
        )
        .unwrap();
        writeln!(file, r#"{{"event":"TaskCreate","ts":"2025-01-15T10:01:00Z","subject":"Story 1","description":"desc"}}"#).unwrap();

        let mut reader = EventReader {
            offset: 0,
            path: events_path.clone(),
        };

        let events = reader.read_new();
        assert_eq!(events.len(), 2);

        // No new events
        let events = reader.read_new();
        assert!(events.is_empty());

        // Append more
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&events_path)
            .unwrap();
        writeln!(file, r#"{{"event":"Stop","ts":"2025-01-15T10:05:00Z"}}"#).unwrap();

        let events = reader.read_new();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], HiveEvent::Stop { .. }));
    }

    #[test]
    fn test_event_reader_nonexistent() {
        let mut reader = EventReader {
            offset: 0,
            path: PathBuf::from("/nonexistent/events.ndjson"),
        };
        assert!(!reader.exists());
        assert!(reader.read_new().is_empty());
    }

    #[test]
    fn test_event_reader_skips_invalid_lines() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.ndjson");

        let mut file = fs::File::create(&events_path).unwrap();
        writeln!(
            file,
            r#"{{"event":"Start","ts":"2025-01-15T10:00:00Z","model":"opus"}}"#
        )
        .unwrap();
        writeln!(file, "not valid json").unwrap();
        writeln!(file, r#"{{"event":"Stop","ts":"2025-01-15T10:05:00Z"}}"#).unwrap();

        let mut reader = EventReader {
            offset: 0,
            path: events_path,
        };

        let events = reader.read_new();
        assert_eq!(events.len(), 2); // skipped the invalid line
    }

    #[test]
    fn test_event_timestamp() {
        let event = HiveEvent::Start {
            ts: "2025-01-15T10:00:00Z".to_string(),
            model: "opus".to_string(),
        };
        assert_eq!(event.timestamp(), "2025-01-15T10:00:00Z");
    }

    #[test]
    fn test_parse_todo_snapshot() {
        let json = r#"{"event":"TodoSnapshot","ts":"2025-01-15T10:00:00Z","todos":[{"content":"Build auth","status":"completed","activeForm":"Building auth"},{"content":"Write tests","status":"in_progress","activeForm":"Writing tests"},{"content":"Deploy","status":"pending"}]}"#;
        let event: HiveEvent = serde_json::from_str(json).unwrap();
        match event {
            HiveEvent::TodoSnapshot { ts, todos } => {
                assert_eq!(ts, "2025-01-15T10:00:00Z");
                assert_eq!(todos.len(), 3);
                assert_eq!(todos[0].content, "Build auth");
                assert_eq!(todos[0].status, "completed");
                assert_eq!(todos[1].status, "in_progress");
                assert_eq!(todos[2].status, "pending");
                assert_eq!(todos[0].active_form, Some("Building auth".to_string()));
            }
            _ => panic!("Expected TodoSnapshot"),
        }
    }

    #[test]
    fn test_reconstruct_tasks_from_todo_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let drone_dir = dir.path().join(".hive/drones/snap-test");
        fs::create_dir_all(&drone_dir).unwrap();
        let events_path = drone_dir.join("events.ndjson");

        let mut file = fs::File::create(&events_path).unwrap();
        // First snapshot: 2 tasks
        writeln!(file, r#"{{"event":"TodoSnapshot","ts":"T1","todos":[{{"content":"Task A","status":"pending"}},{{"content":"Task B","status":"pending"}}]}}"#).unwrap();
        // Second snapshot: Task A completed, Task B in_progress
        writeln!(file, r#"{{"event":"TodoSnapshot","ts":"T2","todos":[{{"content":"Task A","status":"completed"}},{{"content":"Task B","status":"in_progress"}}]}}"#).unwrap();

        // We need to be in the right directory for reconstruct_tasks to find files
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let tasks = reconstruct_tasks("snap-test");

        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].subject, "Task A");
        assert_eq!(tasks[0].status, "completed");
        assert_eq!(tasks[1].subject, "Task B");
        assert_eq!(tasks[1].status, "in_progress");
    }

    #[test]
    fn test_reconstruct_structured_tasks_not_cleared_by_todo_snapshot() {
        // When TaskCreate events exist, TodoSnapshot should NOT clear the main task list
        let dir = tempfile::tempdir().unwrap();
        let drone_dir = dir.path().join(".hive/drones/structured-test");
        fs::create_dir_all(&drone_dir).unwrap();
        let events_path = drone_dir.join("events.ndjson");

        let mut file = fs::File::create(&events_path).unwrap();
        // Lead creates 3 main tasks
        writeln!(file, r#"{{"event":"TaskCreate","ts":"T1","subject":"axum-core","description":"Build core API"}}"#).unwrap();
        writeln!(file, r#"{{"event":"TaskCreate","ts":"T2","subject":"nextjs-foundation","description":"Setup Next.js"}}"#).unwrap();
        writeln!(file, r#"{{"event":"TaskCreate","ts":"T3","subject":"database-schema","description":"Design DB schema"}}"#).unwrap();
        // An agent writes its own subtask list via TodoSnapshot (should NOT overwrite main tasks)
        writeln!(file, r#"{{"event":"TodoSnapshot","ts":"T4","todos":[{{"content":"Install deps","status":"completed"}},{{"content":"Setup routes","status":"in_progress"}}]}}"#).unwrap();
        // Another agent writes a different subtask list
        writeln!(file, r#"{{"event":"TodoSnapshot","ts":"T5","todos":[{{"content":"Create models","status":"pending"}},{{"content":"Write migrations","status":"pending"}},{{"content":"Seed data","status":"pending"}}]}}"#).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let tasks = reconstruct_tasks("structured-test");

        std::env::set_current_dir(original_dir).unwrap();

        // Should have 3 main tasks (from TaskCreate), NOT 3 from last TodoSnapshot
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].subject, "axum-core");
        assert_eq!(tasks[1].subject, "nextjs-foundation");
        assert_eq!(tasks[2].subject, "database-schema");
        // All should be pending (no TaskUpdate events)
        assert_eq!(tasks[0].status, "pending");
        assert_eq!(tasks[1].status, "pending");
        assert_eq!(tasks[2].status, "pending");
    }

    #[test]
    fn test_reconstruct_agent_correlation_via_spawn() {
        // AgentSpawn names should be matched to TaskCreate subjects
        let dir = tempfile::tempdir().unwrap();
        let drone_dir = dir.path().join(".hive/drones/agent-corr-test");
        fs::create_dir_all(&drone_dir).unwrap();
        let events_path = drone_dir.join("events.ndjson");

        let mut file = fs::File::create(&events_path).unwrap();
        writeln!(file, r#"{{"event":"TaskCreate","ts":"T1","subject":"axum-core","description":"Build core API"}}"#).unwrap();
        writeln!(file, r#"{{"event":"TaskCreate","ts":"T2","subject":"nextjs-foundation","description":"Setup Next.js"}}"#).unwrap();
        writeln!(file, r#"{{"event":"TaskCreate","ts":"T3","subject":"database-schema","description":"Design DB schema"}}"#).unwrap();
        // Lead spawns agents with matching names
        writeln!(
            file,
            r#"{{"event":"AgentSpawn","ts":"T4","name":"axum-core","model":"sonnet"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"event":"AgentSpawn","ts":"T5","name":"nextjs-foundation","model":"sonnet"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"event":"AgentSpawn","ts":"T6","name":"database-schema","model":"haiku"}}"#
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let tasks = reconstruct_tasks("agent-corr-test");

        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].owner, Some("axum-core".to_string()));
        assert_eq!(tasks[1].owner, Some("nextjs-foundation".to_string()));
        assert_eq!(tasks[2].owner, Some("database-schema".to_string()));
    }

    #[test]
    fn test_reconstruct_agent_correlation_fuzzy_match() {
        // Agent names can be slightly different from task subjects (contains match)
        let dir = tempfile::tempdir().unwrap();
        let drone_dir = dir.path().join(".hive/drones/fuzzy-test");
        fs::create_dir_all(&drone_dir).unwrap();
        let events_path = drone_dir.join("events.ndjson");

        let mut file = fs::File::create(&events_path).unwrap();
        writeln!(file, r#"{{"event":"TaskCreate","ts":"T1","subject":"Setup Axum Core API","description":"desc"}}"#).unwrap();
        writeln!(file, r#"{{"event":"TaskCreate","ts":"T2","subject":"nextjs_foundation","description":"desc"}}"#).unwrap();
        // Agent name is a substring of the task subject
        writeln!(
            file,
            r#"{{"event":"AgentSpawn","ts":"T3","name":"axum-core-api"}}"#
        )
        .unwrap();
        // Agent name uses different case/separator
        writeln!(
            file,
            r#"{{"event":"AgentSpawn","ts":"T4","name":"nextjs-foundation"}}"#
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let tasks = reconstruct_tasks("fuzzy-test");

        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(tasks.len(), 2);
        // "setup-axum-core-api" contains "axum-core-api"
        assert_eq!(tasks[0].owner, Some("axum-core-api".to_string()));
        // "nextjs_foundation" normalizes to "nextjs-foundation"
        assert_eq!(tasks[1].owner, Some("nextjs-foundation".to_string()));
    }

    #[test]
    fn test_reconstruct_task_update_applies_to_structured() {
        let dir = tempfile::tempdir().unwrap();
        let drone_dir = dir.path().join(".hive/drones/update-test");
        fs::create_dir_all(&drone_dir).unwrap();
        let events_path = drone_dir.join("events.ndjson");

        let mut file = fs::File::create(&events_path).unwrap();
        writeln!(
            file,
            r#"{{"event":"TaskCreate","ts":"T1","subject":"auth","description":""}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"event":"TaskCreate","ts":"T2","subject":"tests","description":""}}"#
        )
        .unwrap();
        // Update task 1 to in_progress
        writeln!(file, r#"{{"event":"TaskUpdate","ts":"T3","task_id":"1","status":"in_progress","owner":"auth-agent"}}"#).unwrap();
        // Complete task 1
        writeln!(
            file,
            r#"{{"event":"TaskDone","ts":"T4","task_id":"1","subject":"auth"}}"#
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let tasks = reconstruct_tasks("update-test");

        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].status, "completed");
        assert_eq!(tasks[0].owner, Some("auth-agent".to_string()));
        assert_eq!(tasks[1].status, "pending");
    }

    #[test]
    fn test_reconstruct_freeform_fallback_uses_last_snapshot() {
        // When there are NO TaskCreate events, the last TodoSnapshot should be used
        let dir = tempfile::tempdir().unwrap();
        let drone_dir = dir.path().join(".hive/drones/freeform-test");
        fs::create_dir_all(&drone_dir).unwrap();
        let events_path = drone_dir.join("events.ndjson");

        let mut file = fs::File::create(&events_path).unwrap();
        // Only TodoSnapshot events (no TaskCreate)
        writeln!(file, r#"{{"event":"TodoSnapshot","ts":"T1","todos":[{{"content":"A","status":"pending"}},{{"content":"B","status":"pending"}}]}}"#).unwrap();
        writeln!(file, r#"{{"event":"TodoSnapshot","ts":"T2","todos":[{{"content":"A","status":"completed"}},{{"content":"B","status":"in_progress"}},{{"content":"C","status":"pending"}}]}}"#).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let tasks = reconstruct_tasks("freeform-test");

        std::env::set_current_dir(original_dir).unwrap();

        // Should use the LAST snapshot (3 tasks)
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].subject, "A");
        assert_eq!(tasks[0].status, "completed");
        assert_eq!(tasks[1].subject, "B");
        assert_eq!(tasks[1].status, "in_progress");
        assert_eq!(tasks[2].subject, "C");
        assert_eq!(tasks[2].status, "pending");
    }

    #[test]
    fn test_reconstruct_description_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let drone_dir = dir.path().join(".hive/drones/desc-test");
        fs::create_dir_all(&drone_dir).unwrap();
        let events_path = drone_dir.join("events.ndjson");

        let mut file = fs::File::create(&events_path).unwrap();
        writeln!(file, r#"{{"event":"TaskCreate","ts":"T1","subject":"auth","description":"Implement JWT authentication with refresh tokens"}}"#).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let tasks = reconstruct_tasks("desc-test");

        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(
            tasks[0].description,
            "Implement JWT authentication with refresh tokens"
        );
    }

    #[test]
    fn test_names_match_exact() {
        assert!(names_match("axum-core", "axum-core"));
        assert!(names_match("AXUM-CORE", "axum-core"));
    }

    #[test]
    fn test_names_match_underscore_vs_hyphen() {
        assert!(names_match("nextjs_foundation", "nextjs-foundation"));
    }

    #[test]
    fn test_names_match_contains() {
        assert!(names_match("Setup Axum Core API", "axum-core-api"));
        assert!(names_match("auth", "auth-module")); // "auth" contained in "auth-module"
    }

    #[test]
    fn test_names_match_no_match() {
        assert!(!names_match("auth", "database"));
        assert!(!names_match("frontend", "backend"));
    }
}
