//! Tool execution logic extracted from the agentic loop.
//!
//! Handles dispatching to built-in tools, MCP tools, and the ToolSearch
//! meta-tool (which activates the deferred tool tier).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::webui::anthropic::types::{ContentBlock, ToolDefinition};
use crate::webui::mcp_client::pool::McpPool;
use crate::webui::tools;

use super::compressor;

/// Result of executing the ToolSearch meta-tool.
pub struct ToolSearchResult {
    pub content: String,
}

/// Execute a batch of tool calls, returning ContentBlocks for the API.
///
/// If a `ToolSearch` call is encountered, it is handled inline using the
/// full `all_tools` list and `deferred_activated` is set to `true`.
pub async fn execute_tools(
    tool_uses: &[(String, String, serde_json::Value)],
    abort_flag: &Arc<std::sync::atomic::AtomicBool>,
    mcp_pool: &Option<Arc<tokio::sync::Mutex<McpPool>>>,
    cwd: &std::path::Path,
    tx: &broadcast::Sender<String>,
    all_tools: &[ToolDefinition],
    deferred_activated: &mut bool,
) -> Vec<ContentBlock> {
    let mut tool_result_blocks: Vec<ContentBlock> = Vec::new();

    for (tool_id, tool_name, tool_input) in tool_uses {
        if abort_flag.load(Ordering::Relaxed) {
            break;
        }

        let result = if tool_name == "ToolSearch" {
            // Meta-tool: search available tools and activate deferred tier
            let content = tools::tool_search::execute(tool_input, all_tools);
            *deferred_activated = true;
            tools::ToolExecutionResult {
                content,
                is_error: false,
            }
        } else if tool_name.contains("__") {
            // MCP tool
            let mcp_result = if let Some(ref pool) = mcp_pool {
                let mut pool = pool.lock().await;
                pool.call_tool(tool_name, tool_input).await
            } else {
                crate::webui::mcp_client::call_mcp_tool(tool_name, tool_input, cwd).await
            };
            match mcp_result {
                Ok(content) => tools::ToolExecutionResult {
                    content,
                    is_error: false,
                },
                Err(e) => tools::ToolExecutionResult {
                    content: format!("{e:#}"),
                    is_error: true,
                },
            }
        } else {
            // Built-in tool
            match tools::execute_tool(tool_name, tool_input, cwd).await {
                Some(r) => r,
                None => tools::ToolExecutionResult {
                    content: format!("Unknown tool: {tool_name}"),
                    is_error: true,
                },
            }
        };

        // Broadcast full (uncompressed) output to the frontend via SSE
        let tool_result_event = serde_json::json!({
            "type": "user",
            "message": {
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": tool_id,
                    "content": result.content,
                    "is_error": result.is_error
                }]
            }
        });
        let _ = tx.send(tool_result_event.to_string());

        // Compress output for API context (saves tokens on subsequent turns)
        let api_content = compressor::compress_tool_output(&result.content, result.is_error);

        tool_result_blocks.push(ContentBlock::ToolResult {
            tool_use_id: tool_id.clone(),
            content: api_content,
            is_error: Some(result.is_error),
        });
    }

    tool_result_blocks
}
