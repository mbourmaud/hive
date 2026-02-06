use anyhow::Result;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;

/// Events received from the Claude Code subprocess
#[derive(Debug)]
pub enum ClaudeEvent {
    /// Assistant is streaming text
    AssistantText(String),
    /// A tool is being used
    ToolUse {
        tool_name: String,
        args_summary: String,
    },
    /// Tool execution completed
    ToolResult {
        tool_name: String,
        success: bool,
        output: String,
    },
    /// Process exited
    ProcessExit(i32),
    /// Error from the process
    Error(String),
}

pub struct ClaudeProcess {
    child: Option<Child>,
    event_rx: mpsc::Receiver<ClaudeEvent>,
    stdin_handle: Option<std::process::ChildStdin>,
}

impl ClaudeProcess {
    /// Spawn a new Claude Code subprocess
    pub fn spawn(prompt: &str, session_id: Option<&str>) -> Result<Self> {
        let mut cmd = Command::new("claude");
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--verbose");

        if let Some(sid) = session_id {
            cmd.arg("--resume").arg(sid);
        }

        cmd.arg("-p").arg(prompt);

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");
        let stdin_handle = child.stdin.take();

        let (tx, rx) = mpsc::channel();

        // Spawn stdout reader thread
        let tx_stdout = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if let Some(event) = parse_stream_event(&line) {
                            if tx_stdout.send(event).is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Spawn stderr reader thread
        let tx_stderr = tx;
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if !line.trim().is_empty() {
                            let _ = tx_stderr.send(ClaudeEvent::Error(line));
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            child: Some(child),
            event_rx: rx,
            stdin_handle,
        })
    }

    /// Try to receive the next event (non-blocking)
    pub fn try_recv(&self) -> Option<ClaudeEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Send input to the Claude process stdin
    pub fn send_input(&mut self, input: &str) -> Result<()> {
        if let Some(ref mut stdin) = self.stdin_handle {
            writeln!(stdin, "{}", input)?;
            stdin.flush()?;
        }
        Ok(())
    }

    /// Kill the subprocess
    pub fn kill(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Check if the process is still running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(_)) => false,
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }
}

impl Drop for ClaudeProcess {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Parse a stream-json line from Claude Code into a ClaudeEvent
fn parse_stream_event(line: &str) -> Option<ClaudeEvent> {
    let json: serde_json::Value = serde_json::from_str(line).ok()?;

    let event_type = json.get("type")?.as_str()?;

    match event_type {
        "assistant" => {
            // Assistant message with text content
            let message = json.get("message")?;
            let content = message.get("content")?;
            if let Some(arr) = content.as_array() {
                for block in arr {
                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                        let text = block.get("text")?.as_str()?.to_string();
                        return Some(ClaudeEvent::AssistantText(text));
                    }
                }
            }
            None
        }
        "content_block_delta" => {
            // Streaming text delta
            let delta = json.get("delta")?;
            if delta.get("type").and_then(|t| t.as_str()) == Some("text_delta") {
                let text = delta.get("text")?.as_str()?.to_string();
                return Some(ClaudeEvent::AssistantText(text));
            }
            None
        }
        "content_block_start" => {
            // Check if it's a tool use block
            let content_block = json.get("content_block")?;
            if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                let tool_name = content_block.get("name")?.as_str()?.to_string();
                let args = content_block
                    .get("input")
                    .map(|v| {
                        let s = v.to_string();
                        if s.len() > 100 {
                            format!("{}...", &s[..100])
                        } else {
                            s
                        }
                    })
                    .unwrap_or_default();
                return Some(ClaudeEvent::ToolUse {
                    tool_name,
                    args_summary: args,
                });
            }
            None
        }
        "result" => {
            // Final result
            let result_text = json
                .get("result")
                .and_then(|r| r.as_str())
                .unwrap_or("Process completed")
                .to_string();
            Some(ClaudeEvent::AssistantText(result_text))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_assistant_text() {
        let json =
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#;
        let event = parse_stream_event(json);
        assert!(matches!(event, Some(ClaudeEvent::AssistantText(ref s)) if s == "Hello world"));
    }

    #[test]
    fn test_parse_text_delta() {
        let json = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"chunk"}}"#;
        let event = parse_stream_event(json);
        assert!(matches!(event, Some(ClaudeEvent::AssistantText(ref s)) if s == "chunk"));
    }

    #[test]
    fn test_parse_tool_use() {
        let json = r#"{"type":"content_block_start","content_block":{"type":"tool_use","name":"Read","input":{"path":"/tmp/file.txt"}}}"#;
        let event = parse_stream_event(json);
        assert!(
            matches!(event, Some(ClaudeEvent::ToolUse { ref tool_name, .. }) if tool_name == "Read")
        );
    }

    #[test]
    fn test_parse_tool_use_long_args() {
        let long_input = "x".repeat(200);
        let json = format!(
            r#"{{"type":"content_block_start","content_block":{{"type":"tool_use","name":"Write","input":{{"content":"{}"}}}}}}"#,
            long_input
        );
        let event = parse_stream_event(&json);
        if let Some(ClaudeEvent::ToolUse { args_summary, .. }) = event {
            assert!(args_summary.len() <= 103); // 100 chars + "..."
            assert!(args_summary.ends_with("..."));
        } else {
            panic!("Expected ToolUse event");
        }
    }

    #[test]
    fn test_parse_result() {
        let json = r#"{"type":"result","result":"Done"}"#;
        let event = parse_stream_event(json);
        assert!(matches!(event, Some(ClaudeEvent::AssistantText(ref s)) if s == "Done"));
    }

    #[test]
    fn test_parse_result_without_text() {
        let json = r#"{"type":"result"}"#;
        let event = parse_stream_event(json);
        assert!(
            matches!(event, Some(ClaudeEvent::AssistantText(ref s)) if s == "Process completed")
        );
    }

    #[test]
    fn test_parse_unknown_type() {
        let json = r#"{"type":"unknown_event"}"#;
        let event = parse_stream_event(json);
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        let event = parse_stream_event("not json");
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_empty_string() {
        let event = parse_stream_event("");
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_missing_type_field() {
        let json = r#"{"data":"something"}"#;
        let event = parse_stream_event(json);
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_assistant_no_text_block() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"image","data":"abc"}]}}"#;
        let event = parse_stream_event(json);
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_content_block_delta_non_text() {
        let json = r#"{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{}"}}"#;
        let event = parse_stream_event(json);
        assert!(event.is_none());
    }
}
