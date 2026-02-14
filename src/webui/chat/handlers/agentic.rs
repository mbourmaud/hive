use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::webui::anthropic::{
    self,
    types::{ContentBlock, Message, MessageContent, MessagesRequest, ThinkingConfig},
};
use crate::webui::auth::credentials;
use crate::webui::mcp_client::pool::McpPool;
use crate::webui::tools;

use super::super::context;
use super::super::session::{Effort, SessionStore};

/// Parameters for the agentic loop, grouped to avoid too-many-arguments.
pub(crate) struct AgenticLoopParams<'a> {
    pub creds: &'a credentials::Credentials,
    pub model: &'a str,
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<anthropic::types::ToolDefinition>>,
    pub cwd: &'a std::path::Path,
    pub tx: &'a broadcast::Sender<String>,
    pub session_id: &'a str,
    pub abort_flag: &'a Arc<std::sync::atomic::AtomicBool>,
    pub store: SessionStore,
    pub effort: Effort,
    pub max_turns: Option<usize>,
    pub mcp_pool: Option<Arc<tokio::sync::Mutex<McpPool>>>,
}

/// The agentic loop: stream API response, execute tools, repeat until end_turn.
pub(crate) async fn run_agentic_loop(
    params: AgenticLoopParams<'_>,
) -> anyhow::Result<Vec<Message>> {
    let AgenticLoopParams {
        creds,
        model,
        mut messages,
        system_prompt,
        tools: session_tools,
        cwd,
        tx,
        session_id,
        abort_flag,
        store,
        effort,
        max_turns,
        mcp_pool,
    } = params;
    let max_tool_turns = max_turns.unwrap_or(25);

    // Build thinking config from effort level
    let thinking = if effort.thinking_enabled() {
        Some(ThinkingConfig {
            thinking_type: "enabled".to_string(),
            budget_tokens: effort.thinking_budget(),
        })
    } else {
        None
    };

    // When thinking is enabled, max_tokens must be > budget
    let base_max_tokens: u32 = if effort.thinking_enabled() {
        effort.thinking_budget() + 16384
    } else {
        16384
    };

    for _turn in 0..max_tool_turns {
        if abort_flag.load(Ordering::Relaxed) {
            break;
        }

        // Context window management: truncate if needed
        let estimated = context::estimate_total_tokens(&messages);
        let api_messages = if effort.thinking_enabled() {
            context::truncate_messages(&messages, estimated)
        } else {
            let stripped = strip_thinking_from_history(&messages);
            let stripped_estimated = context::estimate_total_tokens(&stripped);
            context::truncate_messages(&stripped, stripped_estimated)
        };

        let request = MessagesRequest {
            model: model.to_string(),
            max_tokens: base_max_tokens,
            messages: api_messages,
            system: system_prompt.clone(),
            stream: true,
            metadata: None,
            tools: session_tools.clone(),
            tool_choice: None,
            thinking: thinking.clone(),
            temperature: if effort.thinking_enabled() {
                None
            } else {
                Some(1.0)
            },
        };

        let (assistant_msg, usage, stop_reason) =
            anthropic::client::stream_messages(creds, &request, tx, session_id, abort_flag).await?;

        messages.push(assistant_msg.clone());

        broadcast_usage(tx, session_id, &usage, &store).await;

        if stop_reason != "tool_use" || abort_flag.load(Ordering::Relaxed) {
            break;
        }

        let tool_uses = extract_tool_uses(&assistant_msg);
        if tool_uses.is_empty() {
            break;
        }

        let tool_results = execute_tools(&tool_uses, abort_flag, &mcp_pool, cwd, tx).await;
        let tool_result_message = Message {
            role: "user".to_string(),
            content: MessageContent::Blocks(tool_results),
        };
        messages.push(tool_result_message);
    }

    Ok(messages)
}

async fn broadcast_usage(
    tx: &broadcast::Sender<String>,
    session_id: &str,
    usage: &anthropic::types::UsageStats,
    store: &SessionStore,
) {
    // Update messages and token counters in the store
    {
        let mut sessions = store.lock().await;
        if let Some(s) = sessions.get_mut(session_id) {
            s.total_input_tokens += usage.input_tokens;
            s.total_output_tokens += usage.output_tokens;
        }
    }

    let sessions = store.lock().await;
    if let Some(s) = sessions.get(session_id) {
        let usage_event = serde_json::json!({
            "type": "usage",
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "total_input": s.total_input_tokens,
            "total_output": s.total_output_tokens,
            "cache_creation_input_tokens": usage.cache_creation_input_tokens,
            "cache_read_input_tokens": usage.cache_read_input_tokens
        });
        let _ = tx.send(usage_event.to_string());
    }
}

async fn execute_tools(
    tool_uses: &[(String, String, serde_json::Value)],
    abort_flag: &Arc<std::sync::atomic::AtomicBool>,
    mcp_pool: &Option<Arc<tokio::sync::Mutex<McpPool>>>,
    cwd: &std::path::Path,
    tx: &broadcast::Sender<String>,
) -> Vec<ContentBlock> {
    let mut tool_result_blocks: Vec<ContentBlock> = Vec::new();

    for (tool_id, tool_name, tool_input) in tool_uses {
        if abort_flag.load(Ordering::Relaxed) {
            break;
        }

        let result = if tool_name.contains("__") {
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
            match tools::execute_tool(tool_name, tool_input, cwd).await {
                Some(r) => r,
                None => tools::ToolExecutionResult {
                    content: format!("Unknown tool: {tool_name}"),
                    is_error: true,
                },
            }
        };

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

        tool_result_blocks.push(ContentBlock::ToolResult {
            tool_use_id: tool_id.clone(),
            content: result.content,
            is_error: Some(result.is_error),
        });
    }

    tool_result_blocks
}

/// Extract (id, name, input) tuples from tool_use blocks in an assistant message.
fn extract_tool_uses(msg: &Message) -> Vec<(String, String, serde_json::Value)> {
    match &msg.content {
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.clone(), name.clone(), input.clone()))
                }
                _ => None,
            })
            .collect(),
        MessageContent::Text(_) => Vec::new(),
    }
}

/// Remove thinking blocks from conversation history before sending to the API.
fn strip_thinking_from_history(messages: &[Message]) -> Vec<Message> {
    messages
        .iter()
        .map(|msg| match &msg.content {
            MessageContent::Blocks(blocks) => {
                let filtered: Vec<ContentBlock> = blocks
                    .iter()
                    .filter(|b| !matches!(b, ContentBlock::Thinking { .. }))
                    .cloned()
                    .collect();
                if filtered.is_empty() {
                    Message {
                        role: msg.role.clone(),
                        content: MessageContent::Blocks(vec![ContentBlock::Text {
                            text: String::new(),
                        }]),
                    }
                } else {
                    Message {
                        role: msg.role.clone(),
                        content: MessageContent::Blocks(filtered),
                    }
                }
            }
            _ => msg.clone(),
        })
        .collect()
}
