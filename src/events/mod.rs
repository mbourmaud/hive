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
    /// Quality gate check result
    QualityGateResult {
        ts: String,
        task_id: String,
        passed: bool,
        #[serde(default)]
        output: String,
    },
    /// Worker error (task failure details)
    WorkerError {
        ts: String,
        task_id: String,
        #[serde(default)]
        error_message: String,
    },
    /// Phase transition in the coordinator lifecycle
    PhaseTransition {
        ts: String,
        from_phase: String,
        to_phase: String,
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
            HiveEvent::QualityGateResult { ts, .. } => ts,
            HiveEvent::WorkerError { ts, .. } => ts,
            HiveEvent::PhaseTransition { ts, .. } => ts,
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
mod tests;
