use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::webui::anthropic::types::Message;

/// Metadata persisted to .hive/sessions/{id}/meta.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub cwd: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub title: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Cumulative input tokens (persisted for context usage display)
    #[serde(default)]
    pub total_input_tokens: u64,
    /// Cumulative output tokens (persisted for context usage display)
    #[serde(default)]
    pub total_output_tokens: u64,
}

fn sessions_dir() -> PathBuf {
    PathBuf::from(".hive/sessions")
}

pub fn session_dir(id: &str) -> PathBuf {
    sessions_dir().join(id)
}

fn ensure_session_dir(id: &str) -> std::io::Result<PathBuf> {
    let dir = session_dir(id);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn append_event(id: &str, line: &str) {
    if let Ok(dir) = ensure_session_dir(id) {
        let path = dir.join("events.ndjson");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            use std::io::Write;
            let _ = writeln!(f, "{}", line);
        }
    }
}

pub fn save_messages(id: &str, messages: &[Message]) {
    if let Ok(dir) = ensure_session_dir(id) {
        let path = dir.join("messages.json");
        if let Ok(json) = serde_json::to_string_pretty(messages) {
            let _ = std::fs::write(path, json);
        }
    }
}

pub fn load_messages(id: &str) -> Vec<Message> {
    let path = session_dir(id).join("messages.json");
    if let Ok(data) = std::fs::read_to_string(path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn write_meta(meta: &SessionMeta) {
    if let Ok(dir) = ensure_session_dir(&meta.id) {
        let path = dir.join("meta.json");
        if let Ok(json) = serde_json::to_string_pretty(meta) {
            let _ = std::fs::write(path, json);
        }
    }
}

pub fn read_meta(id: &str) -> Option<SessionMeta> {
    let path = session_dir(id).join("meta.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn extract_title(text: &str) -> String {
    let cleaned: String = text
        .lines()
        .next()
        .unwrap_or(text)
        .trim()
        .chars()
        .take(60)
        .collect();
    if cleaned.is_empty() {
        "Untitled".to_string()
    } else {
        cleaned
    }
}

pub fn update_meta_status(id: &str, status: &str) {
    if let Some(mut meta) = read_meta(id) {
        meta.status = status.to_string();
        meta.updated_at = chrono::Utc::now().to_rfc3339();
        write_meta(&meta);
    }
}

pub fn update_meta_tokens(id: &str, input_tokens: u64, output_tokens: u64) {
    if let Some(mut meta) = read_meta(id) {
        meta.total_input_tokens = input_tokens;
        meta.total_output_tokens = output_tokens;
        write_meta(&meta);
    }
}

/// List session directories on disk for merging with in-memory sessions.
pub fn list_persisted_sessions() -> Vec<(String, SessionMeta)> {
    let dir = sessions_dir();
    let mut results = Vec::new();
    if !dir.exists() {
        return results;
    }
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let id = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };
            if let Some(meta) = read_meta(&id) {
                results.push((id, meta));
            }
        }
    }
    results
}

/// Delete a session's directory from disk.
pub fn delete_session_dir(session_id: &str) {
    let dir = session_dir(session_id);
    let _ = std::fs::remove_dir_all(dir);
}
