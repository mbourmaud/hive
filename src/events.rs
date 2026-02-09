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
        }
    }
}

/// A task reconstructed from events.ndjson
pub struct EventTask {
    pub task_id: String,
    pub subject: String,
    pub status: String,
    pub owner: Option<String>,
}

/// Reconstruct structured task data from events.ndjson by replaying events.
/// Returns a list of tasks with their latest status.
pub fn reconstruct_tasks(drone_name: &str) -> Vec<EventTask> {
    let path = PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("events.ndjson");

    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut task_map: std::collections::HashMap<String, EventTask> =
        std::collections::HashMap::new();
    let mut create_counter: usize = 0;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<HiveEvent>(trimmed) {
            Ok(event) => match event {
                HiveEvent::TaskCreate { subject, .. } => {
                    create_counter += 1;
                    let id = create_counter.to_string();
                    task_map.insert(
                        id.clone(),
                        EventTask {
                            task_id: id,
                            subject,
                            status: "pending".to_string(),
                            owner: None,
                        },
                    );
                }
                HiveEvent::TaskUpdate {
                    task_id,
                    status,
                    owner,
                    ..
                } => {
                    let entry = task_map
                        .entry(task_id.clone())
                        .or_insert_with(|| EventTask {
                            task_id: task_id.clone(),
                            subject: format!("Task {}", task_id),
                            status: "pending".to_string(),
                            owner: None,
                        });
                    if !status.is_empty() {
                        entry.status = status;
                    }
                    if owner.is_some() {
                        entry.owner = owner;
                    }
                }
                HiveEvent::TaskDone {
                    task_id, subject, ..
                } => {
                    let entry = task_map
                        .entry(task_id.clone())
                        .or_insert_with(|| EventTask {
                            task_id,
                            subject: subject.clone(),
                            status: "completed".to_string(),
                            owner: None,
                        });
                    entry.status = "completed".to_string();
                    if !subject.is_empty() {
                        entry.subject = subject;
                    }
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

    let mut tasks: Vec<EventTask> = task_map.into_values().collect();
    tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    tasks
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
}
