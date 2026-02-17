//! `ToolSearch` meta-tool: lets Claude discover available MCP tools.
//!
//! When invoked, returns a filtered list of deferred (MCP) tools matching
//! the query. Also signals that the deferred tier should be activated.

use crate::webui::anthropic::types::ToolDefinition;

/// Search the full tool list for MCP tools matching the query.
/// Returns a compact markdown summary.
pub fn execute(input: &serde_json::Value, available_tools: &[ToolDefinition]) -> String {
    let query = input
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();

    // Only list MCP (deferred) tools
    let mcp_tools: Vec<&ToolDefinition> = available_tools
        .iter()
        .filter(|t| t.name.contains("__"))
        .collect();

    if mcp_tools.is_empty() {
        return "No MCP tools are available in this session.".to_string();
    }

    let matches: Vec<&ToolDefinition> = if query.is_empty() {
        mcp_tools
    } else {
        mcp_tools
            .into_iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&query)
                    || t.description.to_lowercase().contains(&query)
            })
            .collect()
    };

    if matches.is_empty() {
        return format!(
            "No MCP tools matching \"{query}\". Use ToolSearch without a query to list all."
        );
    }

    let mut out = format!("Found {} MCP tool(s):\n\n", matches.len());
    for tool in &matches {
        // Extract server name from "mcp__server__action" pattern
        let parts: Vec<&str> = tool.name.splitn(3, "__").collect();
        let server = *parts.get(1).unwrap_or(&"unknown");
        let name_ref = tool.name.as_str();
        let action = *parts.get(2).unwrap_or(&name_ref);

        let desc_preview = if tool.description.len() > 80 {
            format!("{}…", &tool.description[..80])
        } else {
            tool.description.clone()
        };

        out.push_str(&format!("- **{action}** (`{server}`) — {desc_preview}\n",));
    }

    out.push_str("\nThese tools are now activated for this session.");
    out
}

/// Definition for the ToolSearch meta-tool.
pub fn tool_search_definition() -> ToolDefinition {
    ToolDefinition {
        name: "ToolSearch".to_string(),
        description: "Search for available MCP tools by keyword. Use this to discover browser automation, documentation lookup, and other external tools. Activates deferred tools for the session.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Optional keyword to filter tools (matches name and description). Omit to list all."
                }
            },
            "required": []
        }),
    }
}
