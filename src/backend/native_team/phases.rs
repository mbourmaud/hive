use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::backend::SpawnConfig;
use crate::webui::anthropic::types::{ContentBlock, Message, MessageContent};
use crate::webui::auth::credentials::Credentials;
use crate::webui::chat::handlers::agentic::{run_agentic_loop, AgenticLoopParams};
use crate::webui::chat::session::{Effort, SessionStore};
use crate::webui::provider;
use crate::webui::tools::definitions::builtin_tool_definitions;

const MAX_VERIFY_ATTEMPTS: usize = 3;

/// Run the verification phase with fix-and-retry loop.
pub async fn run_verify_phase(
    config: &SpawnConfig,
    creds: &Credentials,
    store: SessionStore,
) -> bool {
    for attempt in 0..MAX_VERIFY_ATTEMPTS {
        let prompt = if attempt == 0 {
            super::prompts::build_verifier_prompt(config)
        } else {
            let failures = "Previous verification attempt failed. Please re-check.";
            super::prompts::build_fix_prompt(failures, config)
        };

        let result = run_single_agent(&prompt, "verifier", config, creds, store.clone()).await;
        if result.contains("VERIFY_PASS") {
            return true;
        }
    }

    false
}

/// Run the PR creation phase.
pub async fn run_pr_phase(
    config: &SpawnConfig,
    creds: &Credentials,
    store: SessionStore,
    verification_passed: bool,
) {
    let prompt = super::prompts::build_pr_prompt(config, verification_passed);
    run_single_agent(&prompt, "pr-agent", config, creds, store).await;
}

/// Run a single agent (verifier/PR) and return the last assistant text.
async fn run_single_agent(
    system_prompt: &str,
    name: &str,
    config: &SpawnConfig,
    creds: &Credentials,
    store: SessionStore,
) -> String {
    let model_id = provider::resolve_model(&config.model, creds);
    let (tx, _rx) = tokio::sync::broadcast::channel::<String>(256);
    let abort = Arc::new(AtomicBool::new(false));

    let messages = vec![Message {
        role: "user".to_string(),
        content: MessageContent::Text(
            "Execute your task as described in the system prompt.".to_string(),
        ),
    }];

    let params = AgenticLoopParams {
        creds,
        model: &model_id,
        messages,
        system_prompt: Some(system_prompt.to_string()),
        tools: Some(builtin_tool_definitions()),
        cwd: &config.working_dir,
        tx: &tx,
        session_id: name,
        abort_flag: &abort,
        store,
        effort: Effort::Medium,
        max_turns: Some(25),
        mcp_pool: None,
        deferred_tools_active: false,
    };

    match run_agentic_loop(params).await {
        Ok(msgs) => extract_last_text(&msgs),
        Err(e) => format!("VERIFY_FAIL: {e:#}"),
    }
}

/// Extract the last assistant text from conversation messages.
pub fn extract_last_text(messages: &[Message]) -> String {
    for msg in messages.iter().rev() {
        if msg.role != "assistant" {
            continue;
        }
        match &msg.content {
            MessageContent::Text(t) => return t.clone(),
            MessageContent::Blocks(blocks) => {
                let text: String = blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !text.is_empty() {
                    return text;
                }
            }
        }
    }
    String::new()
}
