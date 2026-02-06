use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use super::messages::{ClaudeEvent, Message};

/// Claude Code subprocess manager
#[allow(dead_code)]
pub struct ClaudeBackend {
    /// The Claude subprocess
    process: Option<Child>,
    /// Channel for receiving messages from Claude
    message_rx: Receiver<Message>,
    /// Sender for piping user input to Claude
    input_tx: Option<Sender<String>>,
}

impl ClaudeBackend {
    /// Spawn a new Claude Code subprocess
    #[allow(dead_code)]
    pub fn spawn() -> Result<Self> {
        let mut child = Command::new("claude")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn claude subprocess")?;

        let stdout = child.stdout.take().context("Failed to get stdout")?;
        let mut stdin = child.stdin.take().context("Failed to get stdin")?;

        let (message_tx, message_rx) = channel::<Message>();
        let (input_tx, input_rx) = channel::<String>();

        // Spawn thread to read from Claude's stdout
        let stdout_tx = message_tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) if !line.trim().is_empty() => {
                        match serde_json::from_str::<ClaudeEvent>(&line) {
                            Ok(event) => {
                                let message = Message::from_claude_event(event);
                                let _ = stdout_tx.send(message);
                            }
                            Err(_) => {
                                // Ignore parse errors
                            }
                        }
                    }
                    Err(e) => {
                        let _ = stdout_tx.send(Message::Error(format!("Connection lost: {}", e)));
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Spawn thread to write user input to Claude's stdin
        thread::spawn(move || {
            while let Ok(prompt) = input_rx.recv() {
                if writeln!(stdin, "{}", prompt).is_err() {
                    break;
                }
                if stdin.flush().is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            process: Some(child),
            message_rx,
            input_tx: Some(input_tx),
        })
    }

    /// Send a user prompt to Claude
    #[allow(dead_code)]
    pub fn send_prompt(&self, prompt: &str) -> Result<()> {
        if let Some(tx) = &self.input_tx {
            tx.send(prompt.to_string())
                .context("Failed to send prompt")?;
        }
        Ok(())
    }

    /// Try to receive a message from Claude (non-blocking)
    #[allow(dead_code)]
    pub fn try_recv(&self) -> Option<Message> {
        self.message_rx.try_recv().ok()
    }

    /// Check if the Claude process is still running
    #[allow(dead_code)]
    pub fn is_running(&mut self) -> bool {
        if let Some(child) = &mut self.process {
            child.try_wait().ok().flatten().is_none()
        } else {
            false
        }
    }

    /// Kill the Claude process
    pub fn kill(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            child.kill().context("Failed to kill Claude process")?;
            child.wait()?;
        }
        Ok(())
    }
}

impl Drop for ClaudeBackend {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}
