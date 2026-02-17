//! Built-in tools for searching and listing past chat sessions.
//!
//! - `SessionSearch`: keyword search across session titles and first user messages.
//! - `RecentSessions`: list the N most recent sessions.

use std::path::Path;

use anyhow::{bail, Result};

use crate::chat_engine::persistence;

/// Search past sessions by keyword.
pub async fn execute_search(input: &serde_json::Value, _cwd: &Path) -> Result<String> {
    let query = input
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();

    if query.is_empty() {
        bail!("Missing required parameter: query");
    }

    let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let sessions = persistence::list_persisted_sessions();
    let mut matches: Vec<(String, persistence::SessionMeta, String)> = Vec::new();

    for (id, meta) in &sessions {
        let title_lower = meta.title.to_lowercase();
        let first_msg = first_user_message(id);
        let first_msg_lower = first_msg.to_lowercase();

        if title_lower.contains(&query) || first_msg_lower.contains(&query) {
            matches.push((id.clone(), meta.clone(), first_msg));
        }
    }

    // Sort by updated_at descending
    matches.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
    matches.truncate(limit);

    if matches.is_empty() {
        return Ok(format!("No sessions found matching \"{query}\"."));
    }

    let mut out = format!(
        "Found {} session(s) matching \"{query}\":\n\n",
        matches.len()
    );
    for (id, meta, first_msg) in &matches {
        let short_id = &id[..8.min(id.len())];
        let date = meta
            .updated_at
            .split('T')
            .next()
            .unwrap_or(&meta.updated_at);
        let preview = truncate_str(first_msg, 120);
        out.push_str(&format!(
            "- **{title}** (`{short_id}…` · {model} · {date})\n  > {preview}\n\n",
            title = meta.title,
            model = meta.model,
        ));
    }

    Ok(out)
}

/// List the N most recent sessions.
pub async fn execute_recent(input: &serde_json::Value, _cwd: &Path) -> Result<String> {
    let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let mut sessions = persistence::list_persisted_sessions();
    sessions.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
    sessions.truncate(limit);

    if sessions.is_empty() {
        return Ok("No sessions found.".to_string());
    }

    let mut out = format!("Last {} session(s):\n\n", sessions.len());
    for (id, meta) in &sessions {
        let short_id = &id[..8.min(id.len())];
        let date = meta
            .updated_at
            .split('T')
            .next()
            .unwrap_or(&meta.updated_at);
        let first_msg = first_user_message(id);
        let preview = truncate_str(&first_msg, 120);
        out.push_str(&format!(
            "- **{title}** (`{short_id}…` · {model} · {date})\n  > {preview}\n\n",
            title = meta.title,
            model = meta.model,
        ));
    }

    Ok(out)
}

/// Extract the first user message text from a session's persisted messages.
fn first_user_message(session_id: &str) -> String {
    let messages = persistence::load_messages(session_id);
    for msg in messages {
        if msg.role == "user" {
            return match msg.content {
                crate::webui::anthropic::types::MessageContent::Text(t) => t,
                crate::webui::anthropic::types::MessageContent::Blocks(blocks) => blocks
                    .iter()
                    .find_map(|b| match b {
                        crate::webui::anthropic::types::ContentBlock::Text { text } => {
                            Some(text.clone())
                        }
                        _ => None,
                    })
                    .unwrap_or_default(),
            };
        }
    }
    String::new()
}

fn truncate_str(s: &str, max_len: usize) -> String {
    let clean: String = s.lines().collect::<Vec<_>>().join(" ");
    if clean.len() <= max_len {
        clean
    } else {
        format!("{}…", &clean[..max_len])
    }
}
