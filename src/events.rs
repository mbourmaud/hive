use serde::{Deserialize, Serialize};
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
}
