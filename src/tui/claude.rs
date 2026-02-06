use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use super::messages::{ClaudeEvent, ToolUseInfo};

pub struct ClaudeProcess {
    child: Child,
    stdout_reader: BufReader<std::process::ChildStdout>,
    pending_line: String,
}

impl ClaudeProcess {
    pub fn spawn() -> anyhow::Result<Self> {
        let mut child = Command::new("claude")
            .args(["--output-format", "stream-json"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
        let stdout_reader = BufReader::new(stdout);

        Ok(Self {
            child,
            stdout_reader,
            pending_line: String::new(),
        })
    }

    pub fn send_message(&mut self, msg: &str) -> anyhow::Result<()> {
        if let Some(stdin) = self.child.stdin.as_mut() {
            writeln!(stdin, "{}", msg)?;
            stdin.flush()?;
        }
        Ok(())
    }

    pub fn send_permission_response(&mut self, allow: bool) -> anyhow::Result<()> {
        let response = if allow {
            r#"{"type":"permission_response","allowed":true}"#
        } else {
            r#"{"type":"permission_response","allowed":false}"#
        };
        self.send_message(response)
    }

    pub fn poll_events(&mut self) -> Vec<ClaudeEvent> {
        let mut events = Vec::new();
        loop {
            self.pending_line.clear();
            match self.stdout_reader.read_line(&mut self.pending_line) {
                Ok(0) => {
                    events.push(ClaudeEvent::Finished);
                    break;
                }
                Ok(_) => {
                    let line = self.pending_line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(ev) = parse_stream_event(line) {
                        let is_terminal =
                            matches!(ev, ClaudeEvent::Finished | ClaudeEvent::Error(_));
                        events.push(ev);
                        if is_terminal {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }

            // Non-blocking: check if more data is available
            // For simplicity, break after each line read to avoid blocking
            break;
        }
        events
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ClaudeProcess {
    fn drop(&mut self) {
        self.kill();
    }
}

fn parse_stream_event(line: &str) -> Option<ClaudeEvent> {
    let json: serde_json::Value = serde_json::from_str(line).ok()?;
    let event_type = json.get("type")?.as_str()?;

    match event_type {
        "assistant" => {
            let text = json
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            Some(ClaudeEvent::AssistantText(text))
        }
        "content_block_delta" => {
            let text = json
                .get("delta")
                .and_then(|d| d.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                None
            } else {
                Some(ClaudeEvent::AssistantText(text))
            }
        }
        "tool_use" => {
            let tool_name = json
                .get("tool")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();
            let tool_id = json
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            let input = json
                .get("input")
                .map(|i| {
                    let s = i.to_string();
                    if s.len() > 200 {
                        format!("{}...", &s[..200])
                    } else {
                        s
                    }
                })
                .unwrap_or_default();
            Some(ClaudeEvent::ToolUse(ToolUseInfo {
                tool_name,
                tool_id,
                input_preview: input,
            }))
        }
        "tool_result" => {
            let tool_id = json
                .get("tool_use_id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            let output = json
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let is_error = json
                .get("is_error")
                .and_then(|e| e.as_bool())
                .unwrap_or(false);
            let preview = if output.len() > 200 {
                format!("{}...", &output[..200])
            } else {
                output
            };
            Some(ClaudeEvent::ToolResult {
                tool_id,
                output_preview: preview,
                is_error,
            })
        }
        "permission_request" => {
            let tool_name = json
                .get("tool")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();
            let args = json
                .get("args")
                .map(|a| {
                    let s = a.to_string();
                    if s.len() > 300 {
                        format!("{}...", &s[..300])
                    } else {
                        s
                    }
                })
                .unwrap_or_default();
            Some(ClaudeEvent::PermissionRequest {
                tool_name,
                args_preview: args,
            })
        }
        "error" => {
            let msg = json
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            Some(ClaudeEvent::Error(msg))
        }
        "message_stop" | "result" => Some(ClaudeEvent::Finished),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_assistant_text_event() {
        let line = r#"{"type":"assistant","message":{"content":"Hello world"}}"#;
        let event = parse_stream_event(line).unwrap();
        match event {
            ClaudeEvent::AssistantText(text) => assert_eq!(text, "Hello world"),
            other => panic!("Expected AssistantText, got {:?}", other),
        }
    }

    #[test]
    fn parse_content_block_delta() {
        let line = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"streaming chunk"}}"#;
        let event = parse_stream_event(line).unwrap();
        match event {
            ClaudeEvent::AssistantText(text) => assert_eq!(text, "streaming chunk"),
            other => panic!("Expected AssistantText from delta, got {:?}", other),
        }
    }

    #[test]
    fn parse_empty_delta_returns_none() {
        let line = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":""}}"#;
        assert!(parse_stream_event(line).is_none());
    }

    #[test]
    fn parse_tool_use_event() {
        let line =
            r#"{"type":"tool_use","id":"tool_123","tool":"Read","input":{"file":"main.rs"}}"#;
        let event = parse_stream_event(line).unwrap();
        match event {
            ClaudeEvent::ToolUse(info) => {
                assert_eq!(info.tool_name, "Read");
                assert_eq!(info.tool_id, "tool_123");
                assert!(info.input_preview.contains("main.rs"));
            }
            other => panic!("Expected ToolUse, got {:?}", other),
        }
    }

    #[test]
    fn parse_tool_result_success() {
        let line = r#"{"type":"tool_result","tool_use_id":"tool_123","content":"file contents here","is_error":false}"#;
        let event = parse_stream_event(line).unwrap();
        match event {
            ClaudeEvent::ToolResult {
                tool_id,
                output_preview,
                is_error,
            } => {
                assert_eq!(tool_id, "tool_123");
                assert_eq!(output_preview, "file contents here");
                assert!(!is_error);
            }
            other => panic!("Expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn parse_tool_result_error() {
        let line = r#"{"type":"tool_result","tool_use_id":"tool_456","content":"file not found","is_error":true}"#;
        let event = parse_stream_event(line).unwrap();
        match event {
            ClaudeEvent::ToolResult { is_error, .. } => assert!(is_error),
            other => panic!("Expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn parse_permission_request() {
        let line =
            r#"{"type":"permission_request","tool":"Bash","args":{"command":"rm -rf /tmp/test"}}"#;
        let event = parse_stream_event(line).unwrap();
        match event {
            ClaudeEvent::PermissionRequest {
                tool_name,
                args_preview,
            } => {
                assert_eq!(tool_name, "Bash");
                assert!(args_preview.contains("rm -rf"));
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }

    #[test]
    fn parse_error_event() {
        let line = r#"{"type":"error","message":"Rate limit exceeded"}"#;
        let event = parse_stream_event(line).unwrap();
        match event {
            ClaudeEvent::Error(msg) => assert_eq!(msg, "Rate limit exceeded"),
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn parse_result_event_returns_finished() {
        let line = r#"{"type":"result","subtype":"success"}"#;
        let event = parse_stream_event(line).unwrap();
        assert!(matches!(event, ClaudeEvent::Finished));
    }

    #[test]
    fn parse_message_stop_returns_finished() {
        let line = r#"{"type":"message_stop"}"#;
        let event = parse_stream_event(line).unwrap();
        assert!(matches!(event, ClaudeEvent::Finished));
    }

    #[test]
    fn parse_unknown_event_returns_none() {
        let line = r#"{"type":"ping"}"#;
        assert!(parse_stream_event(line).is_none());
    }

    #[test]
    fn parse_invalid_json_returns_none() {
        assert!(parse_stream_event("not json at all").is_none());
        assert!(parse_stream_event("").is_none());
        assert!(parse_stream_event("{}").is_none());
    }

    #[test]
    fn parse_tool_use_truncates_long_input() {
        let long_input = "x".repeat(300);
        let line =
            format!(r#"{{"type":"tool_use","id":"t1","tool":"Write","input":"{long_input}"}}"#,);
        let event = parse_stream_event(&line).unwrap();
        match event {
            ClaudeEvent::ToolUse(info) => {
                assert!(info.input_preview.len() <= 203); // 200 + "..."
            }
            other => panic!("Expected ToolUse, got {:?}", other),
        }
    }
}
