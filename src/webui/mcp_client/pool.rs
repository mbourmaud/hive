use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{bail, Result};

use super::config;
use super::transport::McpTransport;

/// Per-session MCP connection pool.
/// Keeps initialized transports alive between tool calls instead of
/// spawning and killing a server process for every single call.
pub struct McpPool {
    connections: HashMap<String, McpTransport>,
    cwd: PathBuf,
}

impl McpPool {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            connections: HashMap::new(),
            cwd,
        }
    }

    /// Call an MCP tool by its prefixed name (e.g., "servername__toolname").
    /// Lazily spawns and initializes the server on first use, then reuses it.
    pub async fn call_tool(
        &mut self,
        prefixed_name: &str,
        input: &serde_json::Value,
    ) -> Result<String> {
        let (server_name, tool_name) = prefixed_name
            .split_once("__")
            .unwrap_or(("", prefixed_name));

        // Get or create the transport for this server
        if !self.connections.contains_key(server_name) {
            let transport = self.spawn_and_init(server_name).await?;
            self.connections.insert(server_name.to_string(), transport);
        }

        let transport = self
            .connections
            .get_mut(server_name)
            .expect("just inserted");

        let result = transport
            .send_request(
                "tools/call",
                Some(serde_json::json!({
                    "name": tool_name,
                    "arguments": input
                })),
            )
            .await;

        match result {
            Ok(val) => {
                if let Some(content) = val.get("content").and_then(|c| c.as_array()) {
                    let texts: Vec<String> = content
                        .iter()
                        .filter_map(|item| {
                            if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                item.get("text").and_then(|t| t.as_str()).map(String::from)
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !texts.is_empty() {
                        return Ok(texts.join("\n"));
                    }
                }
                Ok(val.to_string())
            }
            Err(e) => {
                // If the tool call failed, remove the connection so it gets
                // re-spawned on the next attempt (the server may have crashed)
                self.connections.remove(server_name);
                bail!("MCP tool call failed: {e:#}")
            }
        }
    }

    /// Spawn an MCP server, send initialize + initialized, return transport.
    async fn spawn_and_init(&self, server_name: &str) -> Result<McpTransport> {
        let configs = config::load_mcp_configs(&self.cwd);
        let server_config = configs
            .get(server_name)
            .ok_or_else(|| anyhow::anyhow!("MCP server '{server_name}' not found in config"))?;

        let mut transport = McpTransport::spawn(server_config).await?;

        let _init = transport
            .send_request(
                "initialize",
                Some(serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "hive",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })),
            )
            .await?;

        let _ = transport
            .send_request("notifications/initialized", None)
            .await;

        Ok(transport)
    }

    /// Shut down all pooled MCP server connections.
    pub async fn shutdown_all(&mut self) {
        for (_name, transport) in self.connections.drain() {
            transport.shutdown().await;
        }
    }
}
