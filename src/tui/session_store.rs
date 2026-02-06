use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub messages: Vec<SavedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMessage {
    pub role: String,
    pub content: String,
}

pub struct SessionStore {
    sessions_dir: PathBuf,
}

impl SessionStore {
    pub fn new() -> Self {
        let sessions_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("hive")
            .join("tui-sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);
        Self { sessions_dir }
    }

    pub fn list_sessions(&self) -> Vec<SavedSession> {
        let mut sessions = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.sessions_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "json") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(data) = serde_json::from_str::<SessionData>(&content) {
                            sessions.push(SavedSession {
                                id: data.id,
                                title: data.title,
                                created_at: data.created_at,
                                message_count: data.messages.len(),
                            });
                        }
                    }
                }
            }
        }
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        sessions
    }

    pub fn save_session(&self, data: &SessionData) -> anyhow::Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", data.id));
        let json = serde_json::to_string_pretty(data)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_session(&self, id: &str) -> anyhow::Result<SessionData> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        let content = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }

    pub fn delete_session(&self, id: &str) -> anyhow::Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        std::fs::remove_file(path)?;
        Ok(())
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}
