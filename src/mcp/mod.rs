//! MCP (Model Context Protocol) server for Hive.
//!
//! Exposes Hive drone state and messaging as MCP tools that Claude Code
//! (and Agent Teams teammates) can call to query and interact with drones.
//!
//! Launch via: `hive mcp-server`
//! Configure in `.mcp.json` or `~/.claude/settings.json`:
//! ```json
//! { "mcpServers": { "hive": { "command": "hive", "args": ["mcp-server"] } } }
//! ```

mod jsonrpc;
mod tools;

use anyhow::Result;
use std::io::{self, BufRead, Write};

/// Run the MCP server on stdio.
pub fn run_server() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: jsonrpc::JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let error_response = jsonrpc::JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: serde_json::Value::Null,
                    result: None,
                    error: Some(jsonrpc::JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
                writeln!(stdout, "{}", serde_json::to_string(&error_response)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = jsonrpc::handle_request(&request);
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}
