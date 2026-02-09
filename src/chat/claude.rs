use anyhow::Result;
use std::io::BufRead;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ClaudeEvent {
    /// Session initialized
    Init {
        session_id: String,
        model: String,
    },
    /// Text content streaming
    ContentBlockStart {
        index: usize,
        content_type: String, // "text" or "tool_use:<name>"
    },
    ContentBlockDelta {
        index: usize,
        delta: String, // text chunk
    },
    ContentBlockStop {
        index: usize,
    },
    /// Final result with usage info
    Result {
        cost_usd: f64,
        input_tokens: u64,
        output_tokens: u64,
        duration_ms: u64,
        session_id: String,
    },
    /// Error
    Error {
        message: String,
    },
}

pub fn parse_event(line: &str) -> Option<ClaudeEvent> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let event_type = v.get("type")?.as_str()?;

    match event_type {
        "system" => {
            let subtype = v.get("subtype")?.as_str()?;
            if subtype == "init" {
                Some(ClaudeEvent::Init {
                    session_id: v.get("session_id")?.as_str()?.to_string(),
                    model: v
                        .get("model")
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                })
            } else {
                None
            }
        }
        "content_block_start" => {
            let index = v.get("index")?.as_u64()? as usize;
            let block = v.get("content_block")?;
            let content_type = block.get("type")?.as_str()?.to_string();
            if content_type == "tool_use" {
                let tool_name = block
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                Some(ClaudeEvent::ContentBlockStart {
                    index,
                    content_type: format!("tool_use:{}", tool_name),
                })
            } else {
                Some(ClaudeEvent::ContentBlockStart {
                    index,
                    content_type,
                })
            }
        }
        "content_block_delta" => {
            let index = v.get("index")?.as_u64()? as usize;
            let delta = v.get("delta")?;
            let delta_type = delta.get("type")?.as_str()?;
            let text = match delta_type {
                "text_delta" => delta.get("text")?.as_str()?.to_string(),
                "input_json_delta" => delta.get("partial_json")?.as_str()?.to_string(),
                _ => return None,
            };
            Some(ClaudeEvent::ContentBlockDelta { index, delta: text })
        }
        "content_block_stop" => {
            let index = v.get("index")?.as_u64()? as usize;
            Some(ClaudeEvent::ContentBlockStop { index })
        }
        "result" => {
            let is_error = v.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
            if is_error {
                Some(ClaudeEvent::Error {
                    message: v
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("Unknown error")
                        .to_string(),
                })
            } else {
                let usage = v.get("usage");
                Some(ClaudeEvent::Result {
                    cost_usd: v.get("cost_usd").and_then(|c| c.as_f64()).unwrap_or(0.0),
                    input_tokens: usage
                        .and_then(|u| u.get("input_tokens"))
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0),
                    output_tokens: usage
                        .and_then(|u| u.get("output_tokens"))
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0),
                    duration_ms: v.get("duration_ms").and_then(|d| d.as_u64()).unwrap_or(0),
                    session_id: v
                        .get("session_id")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            }
        }
        _ => None,
    }
}

pub struct ClaudeProcess {
    child: Option<Child>,
    model: String,
    working_dir: String,
    session_id: Option<String>,
}

impl ClaudeProcess {
    pub fn new(model: String, working_dir: String) -> Self {
        Self {
            child: None,
            model,
            working_dir,
            session_id: None,
        }
    }

    /// Send a message to Claude. Returns a receiver for streaming events.
    /// Spawns a new claude process for each message.
    pub fn send_message(&mut self, prompt: &str) -> Result<mpsc::Receiver<ClaudeEvent>> {
        let (tx, rx) = mpsc::channel();

        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg(prompt)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--model")
            .arg(&self.model)
            .arg("--verbose")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .current_dir(&self.working_dir);

        if let Some(ref session_id) = self.session_id {
            cmd.arg("--resume").arg(session_id);
        }

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().expect("stdout piped");

        self.child = Some(child);

        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) if !line.trim().is_empty() => {
                        if let Some(event) = parse_event(&line) {
                            if tx.send(event).is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                    _ => continue,
                }
            }
        });

        Ok(rx)
    }

    /// Interrupt the current response (sends SIGINT to child process)
    pub fn interrupt(&mut self) -> Result<()> {
        if let Some(ref child) = self.child {
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                let pid = child.id() as i32;
                let _ = kill(Pid::from_raw(pid), Signal::SIGINT);
            }
        }
        self.child = None;
        Ok(())
    }

    /// Check if a response is currently streaming
    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    /// Set the session ID for conversation continuity
    pub fn set_session_id(&mut self, id: String) {
        self.session_id = Some(id);
    }

    /// Get the current session ID
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

impl Drop for ClaudeProcess {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init_event() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc123","tools":[],"model":"claude-sonnet-4-5-20250929"}"#;
        let event = parse_event(line).unwrap();
        match event {
            ClaudeEvent::Init { session_id, model } => {
                assert_eq!(session_id, "abc123");
                assert!(model.contains("sonnet"));
            }
            _ => panic!("Expected Init event"),
        }
    }

    #[test]
    fn test_parse_content_block_start_text() {
        let line =
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#;
        let event = parse_event(line).unwrap();
        match event {
            ClaudeEvent::ContentBlockStart {
                index,
                content_type,
            } => {
                assert_eq!(index, 0);
                assert_eq!(content_type, "text");
            }
            _ => panic!("Expected ContentBlockStart"),
        }
    }

    #[test]
    fn test_parse_content_delta() {
        let line = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let event = parse_event(line).unwrap();
        match event {
            ClaudeEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                assert_eq!(delta, "Hello");
            }
            _ => panic!("Expected ContentBlockDelta"),
        }
    }

    #[test]
    fn test_parse_content_delta_json() {
        let line = r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"file\""}}"#;
        let event = parse_event(line).unwrap();
        match event {
            ClaudeEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 1);
                assert_eq!(delta, "{\"file\"");
            }
            _ => panic!("Expected ContentBlockDelta"),
        }
    }

    #[test]
    fn test_parse_content_block_stop() {
        let line = r#"{"type":"content_block_stop","index":0}"#;
        let event = parse_event(line).unwrap();
        match event {
            ClaudeEvent::ContentBlockStop { index } => {
                assert_eq!(index, 0);
            }
            _ => panic!("Expected ContentBlockStop"),
        }
    }

    #[test]
    fn test_parse_result_success() {
        let line = r#"{"type":"result","subtype":"success","cost_usd":0.003,"duration_ms":1500,"is_error":false,"num_turns":1,"session_id":"abc123","total_cost_usd":0.003,"usage":{"input_tokens":100,"output_tokens":50}}"#;
        let event = parse_event(line).unwrap();
        match event {
            ClaudeEvent::Result {
                cost_usd,
                input_tokens,
                output_tokens,
                duration_ms,
                session_id,
            } => {
                assert!((cost_usd - 0.003).abs() < f64::EPSILON);
                assert_eq!(input_tokens, 100);
                assert_eq!(output_tokens, 50);
                assert_eq!(duration_ms, 1500);
                assert_eq!(session_id, "abc123");
            }
            _ => panic!("Expected Result event"),
        }
    }

    #[test]
    fn test_parse_result_error() {
        let line = r#"{"type":"result","subtype":"error","error":"something went wrong","is_error":true,"session_id":"abc123"}"#;
        let event = parse_event(line).unwrap();
        match event {
            ClaudeEvent::Error { message } => {
                assert_eq!(message, "something went wrong");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_parse_tool_use_start() {
        let line = r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_xxx","name":"Read","input":{}}}"#;
        let event = parse_event(line).unwrap();
        match event {
            ClaudeEvent::ContentBlockStart {
                index,
                content_type,
            } => {
                assert_eq!(index, 1);
                assert!(content_type.contains("tool_use"));
                assert!(content_type.contains("Read"));
            }
            _ => panic!("Expected ContentBlockStart"),
        }
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_event("not json").is_none());
    }

    #[test]
    fn test_parse_unknown_type() {
        let line = r#"{"type":"unknown_event","data":"test"}"#;
        assert!(parse_event(line).is_none());
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_event("").is_none());
    }

    #[test]
    fn test_parse_assistant_message_ignored() {
        let line = r#"{"type":"assistant","message":{"id":"msg_xxx","type":"message","role":"assistant"}}"#;
        assert!(parse_event(line).is_none());
    }
}
