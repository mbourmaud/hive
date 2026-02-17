use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::webui::anthropic::{
    self,
    types::{ContentBlock, Message, MessageContent, MessagesRequest, ThinkingConfig},
};
use crate::webui::auth::credentials;
use crate::webui::mcp_client::pool::McpPool;
use crate::webui::provider;

use super::context;
use super::persistence;
use super::session::{Effort, SessionStore};
use super::tool_executor;
use super::tool_tier;

/// Parameters for the agentic loop, grouped to avoid too-many-arguments.
pub struct AgenticLoopParams<'a> {
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
    pub deferred_tools_active: bool,
}

/// The agentic loop: stream API response, execute tools, repeat until end_turn.
pub async fn run_agentic_loop(params: AgenticLoopParams<'_>) -> anyhow::Result<Vec<Message>> {
    let AgenticLoopParams {
        creds,
        model,
        mut messages,
        system_prompt,
        tools: all_session_tools,
        cwd,
        tx,
        session_id,
        abort_flag,
        store,
        effort,
        max_turns,
        mcp_pool,
        mut deferred_tools_active,
    } = params;
    let max_tool_turns = max_turns.unwrap_or(25);

    // Resolve model output limit, then fit thinking budget + output within it
    let model_limit = anthropic::model::max_output_tokens(model, effort.thinking_enabled());
    let output_reserve: u32 = 16_384;

    let (thinking, base_max_tokens) = if effort.thinking_enabled() {
        let budget = effort.thinking_budget().min(model_limit - output_reserve);
        let max_tokens = (budget + output_reserve).min(model_limit);
        let thinking = ThinkingConfig {
            thinking_type: "enabled".to_string(),
            budget_tokens: budget,
        };
        (Some(thinking), max_tokens)
    } else {
        (None, output_reserve.min(model_limit))
    };

    // Extract MCP server names for keyword detection
    let mcp_server_names: Vec<String> = all_session_tools
        .as_ref()
        .map(|tools| extract_mcp_server_names(tools))
        .unwrap_or_default();

    for _turn in 0..max_tool_turns {
        if abort_flag.load(Ordering::Relaxed) {
            break;
        }

        // Check if latest user message implies MCP tool usage
        if !deferred_tools_active {
            if let Some(user_text) = last_user_text(&messages) {
                if tool_tier::should_activate_deferred(&user_text, &mcp_server_names) {
                    deferred_tools_active = true;
                }
            }
        }

        // Filter tools by tier: Core always, Deferred only when activated
        let api_tools = all_session_tools
            .as_ref()
            .map(|tools| tool_tier::filter_by_tier(tools, deferred_tools_active));

        // Context window management: truncate if needed
        let estimated = context::estimate_total_tokens(&messages);
        let api_messages = if effort.thinking_enabled() {
            context::truncate_messages(&messages, estimated)
        } else {
            let stripped = strip_thinking_from_history(&messages);
            let stripped_estimated = context::estimate_total_tokens(&stripped);
            context::truncate_messages(&stripped, stripped_estimated)
        };

        // Inject fresh project context into system prompt (30s TTL cache)
        let effective_system = match system_prompt {
            Some(ref base) => {
                let ctx = super::project_context::gather_project_context(cwd).await;
                if ctx.is_empty() {
                    Some(base.clone())
                } else {
                    Some(format!("{base}{ctx}"))
                }
            }
            None => None,
        };

        let request = MessagesRequest {
            model: model.to_string(),
            max_tokens: base_max_tokens,
            messages: api_messages,
            system: effective_system,
            stream: true,
            metadata: None,
            tools: api_tools,
            tool_choice: None,
            thinking: thinking.clone(),
            temperature: if effort.thinking_enabled() {
                None
            } else {
                Some(1.0)
            },
        };

        let (assistant_msg, usage, stop_reason) =
            provider::stream_messages(creds, &request, tx, session_id, abort_flag).await?;

        messages.push(assistant_msg.clone());
        broadcast_usage(tx, session_id, &usage, &store).await;

        if stop_reason != "tool_use" || abort_flag.load(Ordering::Relaxed) {
            break;
        }

        let tool_uses = extract_tool_uses(&assistant_msg);
        if tool_uses.is_empty() {
            break;
        }

        // Pass full tool list so ToolSearch can enumerate all available tools
        let all_tools_ref = all_session_tools.as_deref().unwrap_or(&[]);
        let tool_results = tool_executor::execute_tools(
            &tool_uses,
            abort_flag,
            &mcp_pool,
            cwd,
            tx,
            all_tools_ref,
            &mut deferred_tools_active,
        )
        .await;

        let tool_result_message = Message {
            role: "user".to_string(),
            content: MessageContent::Blocks(tool_results),
        };
        messages.push(tool_result_message);
    }

    // Persist deferred activation state back to session
    if deferred_tools_active {
        let mut sessions = store.lock().await;
        if let Some(s) = sessions.get_mut(session_id) {
            s.deferred_tools_active = deferred_tools_active;
        }
    }

    Ok(messages)
}

async fn broadcast_usage(
    tx: &broadcast::Sender<String>,
    session_id: &str,
    usage: &anthropic::types::UsageStats,
    store: &SessionStore,
) {
    {
        let mut sessions = store.lock().await;
        if let Some(s) = sessions.get_mut(session_id) {
            s.total_input_tokens = usage.input_tokens;
            s.total_output_tokens += usage.output_tokens;
        }
    }

    let sessions = store.lock().await;
    if let Some(s) = sessions.get(session_id) {
        let total_in = s.total_input_tokens;
        let total_out = s.total_output_tokens;

        let usage_event = serde_json::json!({
            "type": "usage",
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "total_input": total_in,
            "total_output": total_out,
            "cache_creation_input_tokens": usage.cache_creation_input_tokens,
            "cache_read_input_tokens": usage.cache_read_input_tokens
        });
        let _ = tx.send(usage_event.to_string());
        drop(sessions);

        persistence::update_meta_tokens(session_id, total_in, total_out);
    }
}

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
                            text: ".".to_string(),
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

/// Extract the text of the last user message (for keyword detection).
fn last_user_text(messages: &[Message]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .and_then(|m| match &m.content {
            MessageContent::Text(t) => Some(t.clone()),
            MessageContent::Blocks(blocks) => blocks.iter().find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            }),
        })
}

/// Extract unique MCP server names from tool definitions (e.g. "playwright" from "mcp__playwright__click").
fn extract_mcp_server_names(tools: &[anthropic::types::ToolDefinition]) -> Vec<String> {
    let mut names: Vec<String> = tools
        .iter()
        .filter_map(|t| {
            let parts: Vec<&str> = t.name.splitn(3, "__").collect();
            if parts.len() >= 2 {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names.dedup();
    names
}
