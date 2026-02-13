pub mod config;
pub mod transport;
pub mod types;

use std::path::Path;

use anyhow::{bail, Result};

use crate::webui::anthropic::types::ToolDefinition;
use transport::McpTransport;
use types::McpToolInfo;

/// Discover MCP tools available for a given working directory.
/// Spawns configured MCP servers, calls initialize + tools/list, then shuts them down.
/// Returns tool definitions with server-prefixed names (e.g., "servername__toolname").
pub async fn discover_tools_for_cwd(cwd: &Path) -> Vec<ToolDefinition> {
    let configs = config::load_mcp_configs(cwd);
    if configs.is_empty() {
        return Vec::new();
    }

    let mut all_tools = Vec::new();

    for (server_name, server_config) in &configs {
        match discover_server_tools(server_name, server_config).await {
            Ok(tools) => all_tools.extend(tools),
            Err(e) => {
                eprintln!("MCP server '{server_name}' tool discovery failed: {e:#}");
            }
        }
    }

    all_tools
}

/// Discover tools from a single MCP server.
async fn discover_server_tools(
    server_name: &str,
    server_config: &types::McpServerConfig,
) -> Result<Vec<ToolDefinition>> {
    let mut transport = McpTransport::spawn(server_config).await?;

    // Send initialize
    let _init_result = transport
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

    // Send initialized notification (no response expected, but we send it as a request)
    // For notifications, we write directly without expecting a response
    let _ = transport
        .send_request("notifications/initialized", None)
        .await;

    // List tools
    let tools_result = transport
        .send_request("tools/list", Some(serde_json::json!({})))
        .await?;

    let mcp_tools: Vec<McpToolInfo> = if let Some(tools_array) = tools_result.get("tools") {
        serde_json::from_value(tools_array.clone()).unwrap_or_default()
    } else {
        Vec::new()
    };

    let definitions: Vec<ToolDefinition> = mcp_tools
        .into_iter()
        .map(|t| ToolDefinition {
            name: format!("{server_name}__{}", t.name),
            description: t.description.unwrap_or_default(),
            input_schema: t.input_schema.unwrap_or(serde_json::json!({
                "type": "object",
                "properties": {}
            })),
        })
        .collect();

    transport.shutdown().await;

    Ok(definitions)
}

/// Call an MCP tool by its prefixed name (e.g., "servername__toolname").
/// Spawns the appropriate MCP server, calls the tool, and shuts down.
pub async fn call_mcp_tool(
    prefixed_name: &str,
    input: &serde_json::Value,
    cwd: &Path,
) -> Result<String> {
    let (server_name, tool_name) = prefixed_name
        .split_once("__")
        .unwrap_or(("", prefixed_name));

    let configs = config::load_mcp_configs(cwd);
    let server_config = configs
        .get(server_name)
        .ok_or_else(|| anyhow::anyhow!("MCP server '{server_name}' not found in config"))?;

    let mut transport = McpTransport::spawn(server_config).await?;

    // Initialize
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

    // Call the tool
    let result = transport
        .send_request(
            "tools/call",
            Some(serde_json::json!({
                "name": tool_name,
                "arguments": input
            })),
        )
        .await;

    transport.shutdown().await;

    match result {
        Ok(val) => {
            // Extract text content from MCP response
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
        Err(e) => bail!("MCP tool call failed: {e:#}"),
    }
}
