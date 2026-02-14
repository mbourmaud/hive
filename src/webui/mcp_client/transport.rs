use anyhow::{bail, Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use super::types::{JsonRpcRequest, JsonRpcResponse, McpServerConfig};

/// A stdio-based MCP transport: communicates with an MCP server via stdin/stdout JSON-RPC.
pub struct McpTransport {
    child: Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

impl McpTransport {
    /// Spawn the MCP server process and return a transport handle.
    pub async fn spawn(config: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);

        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn MCP server: {}", config.command))?;

        let stdin = child
            .stdin
            .take()
            .context("Failed to capture MCP server stdin")?;
        let stdout = child
            .stdout
            .take()
            .context("Failed to capture MCP server stdout")?;

        Ok(Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
            next_id: 1,
        })
    }

    /// Send a JSON-RPC request and wait for the matching response.
    pub async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let mut line = serde_json::to_string(&request).context("Serializing JSON-RPC request")?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .await
            .context("Writing to MCP server stdin")?;
        self.stdin.flush().await.ok();

        // Read lines until we get a response matching our ID
        let timeout = tokio::time::Duration::from_secs(30);
        let response = tokio::time::timeout(timeout, async {
            loop {
                let mut response_line = String::new();
                let bytes_read = self
                    .reader
                    .read_line(&mut response_line)
                    .await
                    .context("Reading from MCP server stdout")?;

                if bytes_read == 0 {
                    bail!("MCP server closed stdout unexpectedly");
                }

                let trimmed = response_line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Try to parse as JSON-RPC response
                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                    if resp.id == Some(id) {
                        if let Some(err) = resp.error {
                            bail!("MCP error ({}): {}", err.code, err.message);
                        }
                        return Ok(resp.result.unwrap_or(serde_json::Value::Null));
                    }
                    // Response for a different ID — skip (could be a notification)
                }
                // Not a JSON-RPC response — skip (could be a log line)
            }
        })
        .await
        .context("MCP server response timeout")?;

        response
    }

    /// Shut down the MCP server process.
    pub async fn shutdown(mut self) {
        let _ = self.stdin.shutdown().await;
        let _ = self.child.kill().await;
    }
}
