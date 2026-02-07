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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::commands::common::{list_drones, load_prd, reconcile_progress_with_prd};

// ============================================================================
// JSON-RPC types
// ============================================================================

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

// ============================================================================
// MCP types
// ============================================================================

#[derive(Debug, Serialize)]
struct ToolInfo {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

#[derive(Debug, Serialize)]
struct ToolResult {
    content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "std::ops::Not::not")]
    is_error: bool,
}

#[derive(Debug, Serialize)]
struct ToolContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

// ============================================================================
// Server
// ============================================================================

/// Run the MCP server on stdio.
pub fn run_server() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
                writeln!(stdout, "{}", serde_json::to_string(&error_response)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = handle_request(&request);
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_request(request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "hive",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            error: None,
        },

        "notifications/initialized" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(Value::Null),
            error: None,
        },

        "tools/list" => {
            let tools = list_tools();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({ "tools": tools })),
                error: None,
            }
        }

        "tools/call" => {
            let tool_name = request
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));
            let result = call_tool(tool_name, &arguments);
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
                error: None,
            }
        }

        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
            }),
        },
    }
}

// ============================================================================
// Tool definitions
// ============================================================================

fn list_tools() -> Vec<ToolInfo> {
    vec![
        ToolInfo {
            name: "hive_list_drones".to_string(),
            description: "List all Hive drones with their current status, progress, and execution mode.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolInfo {
            name: "hive_drone_status".to_string(),
            description: "Get detailed status of a specific drone including current story, completed stories, timing, and error info.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "drone_name": {
                        "type": "string",
                        "description": "Name of the drone to query"
                    }
                },
                "required": ["drone_name"]
            }),
        },
        ToolInfo {
            name: "hive_drone_progress".to_string(),
            description: "Get the progress (completed/total stories) of a drone, reconciled with the current PRD.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "drone_name": {
                        "type": "string",
                        "description": "Name of the drone to query"
                    }
                },
                "required": ["drone_name"]
            }),
        },
        ToolInfo {
            name: "hive_query_dependencies".to_string(),
            description: "Check if the dependencies of a specific story are satisfied (all depended-on stories are completed).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "drone_name": {
                        "type": "string",
                        "description": "Name of the drone"
                    },
                    "story_id": {
                        "type": "string",
                        "description": "ID of the story to check dependencies for"
                    }
                },
                "required": ["drone_name", "story_id"]
            }),
        },
        ToolInfo {
            name: "hive_team_status".to_string(),
            description: "Get Agent Teams status for a drone. Returns task progress and teammate information.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "drone_name": {
                        "type": "string",
                        "description": "Name of the drone/team to query"
                    }
                },
                "required": ["drone_name"]
            }),
        },
    ]
}

// ============================================================================
// Tool implementations
// ============================================================================

fn call_tool(name: &str, arguments: &Value) -> ToolResult {
    let result = match name {
        "hive_list_drones" => tool_list_drones(),
        "hive_drone_status" => tool_drone_status(arguments),
        "hive_drone_progress" => tool_drone_progress(arguments),
        "hive_query_dependencies" => tool_query_dependencies(arguments),
        "hive_team_status" => tool_team_status(arguments),
        _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
    };

    match result {
        Ok(text) => ToolResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text,
            }],
            is_error: false,
        },
        Err(e) => ToolResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Error: {}", e),
            }],
            is_error: true,
        },
    }
}

fn tool_list_drones() -> Result<String> {
    let drones = list_drones()?;

    if drones.is_empty() {
        return Ok("No drones found. Run 'hive start <name>' to launch a drone.".to_string());
    }

    let mut entries = Vec::new();
    for (name, status) in &drones {
        let (completed, total) = crate::commands::common::reconcile_progress(status);
        entries.push(serde_json::json!({
            "name": name,
            "status": status.status.to_string(),
            "execution_mode": status.execution_mode.to_string(),
            "backend": status.backend,
            "progress": format!("{}/{}", completed, total),
            "current_story": status.current_story,
            "branch": status.branch,
            "updated": status.updated,
        }));
    }

    Ok(serde_json::to_string_pretty(&entries)?)
}

fn tool_drone_status(args: &Value) -> Result<String> {
    let drone_name = args
        .get("drone_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: drone_name"))?;

    let drones = list_drones()?;
    let (_, status) = drones
        .iter()
        .find(|(name, _)| name == drone_name)
        .ok_or_else(|| anyhow::anyhow!("Drone '{}' not found", drone_name))?;

    Ok(serde_json::to_string_pretty(status)?)
}

fn tool_drone_progress(args: &Value) -> Result<String> {
    let drone_name = args
        .get("drone_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: drone_name"))?;

    let drones = list_drones()?;
    let (_, status) = drones
        .iter()
        .find(|(name, _)| name == drone_name)
        .ok_or_else(|| anyhow::anyhow!("Drone '{}' not found", drone_name))?;

    let prd_path = PathBuf::from(".hive/prds").join(&status.prd);
    let (completed, total) = if let Some(prd) = load_prd(&prd_path) {
        reconcile_progress_with_prd(status, &prd)
    } else {
        (status.completed.len(), status.total)
    };

    let result = serde_json::json!({
        "drone": drone_name,
        "completed": completed,
        "total": total,
        "percentage": if total > 0 { (completed as f64 / total as f64 * 100.0).round() as u32 } else { 0 },
        "completed_stories": status.completed,
        "current_story": status.current_story,
        "status": status.status.to_string(),
    });

    Ok(serde_json::to_string_pretty(&result)?)
}

fn tool_query_dependencies(args: &Value) -> Result<String> {
    let drone_name = args
        .get("drone_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: drone_name"))?;
    let story_id = args
        .get("story_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: story_id"))?;

    let drones = list_drones()?;
    let (_, status) = drones
        .iter()
        .find(|(name, _)| name == drone_name)
        .ok_or_else(|| anyhow::anyhow!("Drone '{}' not found", drone_name))?;

    // Stories removed in plan mode - dependencies not supported
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "story_id": story_id,
        "has_dependencies": false,
        "all_satisfied": true,
        "message": "Story dependencies not supported in plan mode"
    }))?)
}

fn tool_team_status(args: &Value) -> Result<String> {
    let drone_name = args
        .get("drone_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: drone_name"))?;

    // Read Agent Teams task list
    let tasks = crate::agent_teams::read_task_list(drone_name)?;

    let completed = tasks.iter().filter(|t| t.status == "completed").count();
    let in_progress = tasks.iter().filter(|t| t.status == "in_progress").count();
    let pending = tasks.iter().filter(|t| t.status == "pending").count();

    let task_details: Vec<serde_json::Value> = tasks
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "subject": t.subject,
                "status": t.status,
                "blocked_by": t.blocked_by,
            })
        })
        .collect();

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "drone": drone_name,
        "is_team": true,
        "backend": "agent_team",
        "task_summary": {
            "total": tasks.len(),
            "completed": completed,
            "in_progress": in_progress,
            "pending": pending,
        },
        "tasks": task_details,
    }))?)
}
