use anyhow::{Context, Result};
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::io::{BufRead, BufReader, Write};
use serde_json::Value;

use super::messages::Message;

/// Handles spawning and managing a Claude Code subprocess
pub struct ClaudeBackend {
    child: Option<Child>,
    message_tx: Option<Sender<Message>>,
    input_tx: Option<Sender<String>>,
}

/// Stream event types from Claude's stream-json output
#[derive(Debug)]
enum StreamEvent {
    Assistant(String),
    ToolUse { name: String, args: Value },
    ToolResult { success: bool, output: String },
    Error(String),
}

impl ClaudeBackend {
    /// Create a new Claude backend (not yet started)
    pub fn new() -> Self {
        Self {
            child: None,
            message_tx: None,
            input_tx: None,
        }
    }

    /// Spawn Claude Code subprocess with stream-json output
    pub fn spawn(&mut self, model: &str) -> Result<Receiver<Message>> {
        let (msg_tx, msg_rx) = channel::<Message>();
        let (input_tx, input_rx) = channel::<String>();

        let mut child = ProcessCommand::new("claude")
            .arg("--model")
            .arg(model)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn claude process")?;

        let stdout = child.stdout.take().context("Failed to get stdout")?;
        let mut stdin = child.stdin.take().context("Failed to get stdin")?;

        // Spawn thread to read stdout and parse NDJSON stream
        let msg_tx_clone = msg_tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line_content) => {
                        if let Err(e) = Self::parse_and_send_event(&line_content, &msg_tx_clone) {
                            let _ = msg_tx_clone.send(Message::error(format!("Parse error: {}", e)));
                        }
                    }
                    Err(e) => {
                        let _ = msg_tx_clone.send(Message::error(format!("Read error: {}", e)));
                        break;
                    }
                }
            }
        });

        // Spawn thread to write to stdin
        thread::spawn(move || {
            while let Ok(input) = input_rx.recv() {
                if let Err(e) = writeln!(stdin, "{}", input) {
                    eprintln!("Failed to write to Claude stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush() {
                    eprintln!("Failed to flush Claude stdin: {}", e);
                    break;
                }
            }
        });

        self.child = Some(child);
        self.message_tx = Some(msg_tx);
        self.input_tx = Some(input_tx);

        Ok(msg_rx)
    }

    /// Send user input to Claude subprocess
    pub fn send_input(&self, input: String) -> Result<()> {
        if let Some(ref tx) = self.input_tx {
            tx.send(input).context("Failed to send input to Claude")?;
        }
        Ok(())
    }

    /// Parse a single NDJSON line and send the appropriate message
    fn parse_and_send_event(line: &str, tx: &Sender<Message>) -> Result<()> {
        // Skip empty lines
        if line.trim().is_empty() {
            return Ok(());
        }

        let event: Value = serde_json::from_str(line)
            .context("Failed to parse JSON line")?;

        // Extract event type
        let event_type = event.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match event_type {
            "assistant" | "content_block_delta" => {
                // Assistant text delta
                if let Some(text) = event.get("delta")
                    .and_then(|d| d.get("text"))
                    .and_then(|t| t.as_str()) {
                    tx.send(Message::assistant(text.to_string()))?;
                } else if let Some(text) = event.get("text").and_then(|t| t.as_str()) {
                    tx.send(Message::assistant(text.to_string()))?;
                }
            }
            "tool_use" => {
                // Tool use event
                let name = event.get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let args = event.get("input")
                    .or_else(|| event.get("args"))
                    .cloned()
                    .unwrap_or(Value::Null);

                let args_summary = format!("{}", serde_json::to_string(&args)?);
                let truncated = if args_summary.len() > 100 {
                    format!("{}...", &args_summary[..100])
                } else {
                    args_summary
                };

                tx.send(Message::tool_use(name, truncated))?;
            }
            "tool_result" => {
                // Tool result event
                let success = event.get("is_error")
                    .and_then(|e| e.as_bool())
                    .map(|e| !e)
                    .unwrap_or(true);

                let output = event.get("content")
                    .or_else(|| event.get("output"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                let summary = if output.len() > 200 {
                    format!("{}...", &output[..200])
                } else {
                    output
                };

                tx.send(Message::tool_result(success, summary))?;
            }
            "error" => {
                // Error event
                let error_msg = event.get("message")
                    .or_else(|| event.get("error"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();

                tx.send(Message::error(error_msg))?;
            }
            _ => {
                // Unknown event type - log it but don't fail
                eprintln!("Unknown event type: {}", event_type);
            }
        }

        Ok(())
    }

    /// Check if the Claude process is still running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            // Try to get exit status without blocking
            match child.try_wait() {
                Ok(Some(_)) => false, // Process has exited
                Ok(None) => true,     // Process is still running
                Err(_) => false,      // Error checking status
            }
        } else {
            false
        }
    }

    /// Kill the Claude process
    pub fn kill(&mut self) -> Result<()> {
        if let Some(ref mut child) = self.child {
            child.kill().context("Failed to kill Claude process")?;
            child.wait().context("Failed to wait for Claude process")?;
        }
        self.child = None;
        self.message_tx = None;
        self.input_tx = None;
        Ok(())
    }

    /// Restart the Claude process
    pub fn restart(&mut self, model: &str) -> Result<Receiver<Message>> {
        self.kill()?;
        self.spawn(model)
    }
}

impl Drop for ClaudeBackend {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_assistant_event() {
        let (tx, rx) = channel();
        let line = r#"{"type":"assistant","delta":{"text":"Hello"}}"#;
        ClaudeBackend::parse_and_send_event(line, &tx).unwrap();

        let msg = rx.recv().unwrap();
        match msg {
            Message::Assistant { content, .. } => assert_eq!(content, "Hello"),
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_parse_tool_use_event() {
        let (tx, rx) = channel();
        let line = r#"{"type":"tool_use","name":"Read","input":{"file_path":"test.rs"}}"#;
        ClaudeBackend::parse_and_send_event(line, &tx).unwrap();

        let msg = rx.recv().unwrap();
        match msg {
            Message::ToolUse { tool_name, .. } => assert_eq!(tool_name, "Read"),
            _ => panic!("Expected ToolUse message"),
        }
    }

    #[test]
    fn test_parse_error_event() {
        let (tx, rx) = channel();
        let line = r#"{"type":"error","message":"Something went wrong"}"#;
        ClaudeBackend::parse_and_send_event(line, &tx).unwrap();

        let msg = rx.recv().unwrap();
        match msg {
            Message::Error { content, .. } => assert_eq!(content, "Something went wrong"),
            _ => panic!("Expected Error message"),
        }
    }
}
