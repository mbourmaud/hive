use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use crate::commands::common::{agent_teams_progress, list_drones};

#[derive(Debug, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

#[derive(Debug, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

pub fn list_tools() -> Vec<ToolInfo> {
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
            description: "Get detailed status of a specific drone including current task, completed tasks, timing, and error info.".to_string(),
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
            description: "Get the progress (completed/total tasks) of a drone, reconciled with the current plan.".to_string(),
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

pub fn call_tool(name: &str, arguments: &Value) -> ToolResult {
    let result = match name {
        "hive_list_drones" => tool_list_drones(),
        "hive_drone_status" => tool_drone_status(arguments),
        "hive_drone_progress" => tool_drone_progress(arguments),
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
        let (completed, total) = agent_teams_progress(&status.drone);
        entries.push(serde_json::json!({
            "name": name,
            "status": status.status.to_string(),
            "execution_mode": status.execution_mode.to_string(),
            "backend": status.backend,
            "progress": format!("{}/{}", completed, total),
            "current_task": status.current_task,
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

    let (completed, total) = agent_teams_progress(drone_name);

    let result = serde_json::json!({
        "drone": drone_name,
        "completed": completed,
        "total": total,
        "percentage": if total > 0 { (completed as f64 / total as f64 * 100.0).round() as u32 } else { 0 },
        "completed_tasks": status.completed,
        "current_task": status.current_task,
        "status": status.status.to_string(),
    });

    Ok(serde_json::to_string_pretty(&result)?)
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
