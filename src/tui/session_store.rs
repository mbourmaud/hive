/// Session storage and management for the Hive TUI
/// Persists chat sessions to .hive/sessions/

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    pub last_message: Option<String>,
    #[serde(default)]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub timestamp: String,
}

pub struct SessionStore {
    sessions_dir: PathBuf,
}

impl SessionStore {
    pub fn new() -> Result<Self> {
        let sessions_dir = PathBuf::from(".hive/sessions");
        if !sessions_dir.exists() {
            fs::create_dir_all(&sessions_dir).context("Failed to create sessions directory")?;
        }
        Ok(Self { sessions_dir })
    }

    /// Create a new session
    pub fn create_session(&self) -> Result<Session> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let session = Session {
            id: id.clone(),
            title: "New Session".to_string(),
            created_at: now.clone(),
            updated_at: now,
            message_count: 0,
            last_message: None,
            is_active: true,
        };

        self.save_session(&session)?;
        Ok(session)
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();

        if !self.sessions_dir.exists() {
            return Ok(sessions);
        }

        for entry in fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(contents) = fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<Session>(&contents) {
                        sessions.push(session);
                    }
                }
            }
        }

        // Sort by updated_at descending (most recent first)
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    /// Get a specific session by ID
    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let path = self.session_path(id);
        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path)?;
        let session = serde_json::from_str(&contents)?;
        Ok(Some(session))
    }

    /// Save session metadata
    pub fn save_session(&self, session: &Session) -> Result<()> {
        let path = self.session_path(&session.id);
        let contents = serde_json::to_string_pretty(session)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Update session title (auto-generated from first message)
    pub fn update_title(&self, id: &str, title: String) -> Result<()> {
        if let Some(mut session) = self.get_session(id)? {
            session.title = title;
            session.updated_at = Utc::now().to_rfc3339();
            self.save_session(&session)?;
        }
        Ok(())
    }

    /// Add a message to session
    pub fn add_message(&self, id: &str, role: &str, content: &str) -> Result<()> {
        if let Some(mut session) = self.get_session(id)? {
            session.message_count += 1;
            session.last_message = Some(content.to_string());
            session.updated_at = Utc::now().to_rfc3339();

            // Auto-generate title from first user message
            if session.message_count == 1 && role == "user" && session.title == "New Session" {
                let title = generate_title_from_message(content);
                session.title = title;
            }

            self.save_session(&session)?;

            // Append message to messages file
            let msg = SessionMessage {
                role: role.to_string(),
                content: content.to_string(),
                timestamp: Utc::now().to_rfc3339(),
            };
            self.append_message_to_file(id, &msg)?;
        }
        Ok(())
    }

    /// Get all messages for a session
    pub fn get_messages(&self, id: &str) -> Result<Vec<SessionMessage>> {
        let messages_path = self.messages_path(id);
        if !messages_path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&messages_path)?;
        let messages: Vec<SessionMessage> = serde_json::from_str(&contents)?;
        Ok(messages)
    }

    /// Mark a session as active
    pub fn set_active(&self, id: &str) -> Result<()> {
        // First, deactivate all sessions
        let sessions = self.list_sessions()?;
        for mut session in sessions {
            if session.is_active {
                session.is_active = false;
                self.save_session(&session)?;
            }
        }

        // Activate the specified session
        if let Some(mut session) = self.get_session(id)? {
            session.is_active = true;
            self.save_session(&session)?;
        }

        Ok(())
    }

    /// Get the currently active session
    pub fn get_active_session(&self) -> Result<Option<Session>> {
        let sessions = self.list_sessions()?;
        Ok(sessions.into_iter().find(|s| s.is_active))
    }

    /// Delete a session
    pub fn delete_session(&self, id: &str) -> Result<()> {
        let session_path = self.session_path(id);
        let messages_path = self.messages_path(id);

        if session_path.exists() {
            fs::remove_file(&session_path)?;
        }
        if messages_path.exists() {
            fs::remove_file(&messages_path)?;
        }

        Ok(())
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}.json", id))
    }

    fn messages_path(&self, id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}_messages.json", id))
    }

    fn append_message_to_file(&self, id: &str, message: &SessionMessage) -> Result<()> {
        let messages_path = self.messages_path(id);
        let mut messages = if messages_path.exists() {
            let contents = fs::read_to_string(&messages_path)?;
            serde_json::from_str(&contents)?
        } else {
            Vec::new()
        };

        messages.push(message.clone());
        let contents = serde_json::to_string_pretty(&messages)?;
        fs::write(&messages_path, contents)?;
        Ok(())
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new().expect("Failed to create SessionStore")
    }
}

/// Generate a title from the first message (max 50 chars)
fn generate_title_from_message(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.len() <= 50 {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..47])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_create_session() {
        let temp_dir = env::temp_dir().join("hive-test-sessions");
        fs::create_dir_all(&temp_dir).unwrap();

        let store = SessionStore {
            sessions_dir: temp_dir.clone(),
        };

        let session = store.create_session().unwrap();
        assert_eq!(session.title, "New Session");
        assert_eq!(session.message_count, 0);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_add_message_generates_title() {
        let temp_dir = env::temp_dir().join("hive-test-title-gen");
        fs::create_dir_all(&temp_dir).unwrap();

        let store = SessionStore {
            sessions_dir: temp_dir.clone(),
        };

        let session = store.create_session().unwrap();
        store
            .add_message(&session.id, "user", "Hello, how are you?")
            .unwrap();

        let updated = store.get_session(&session.id).unwrap().unwrap();
        assert_eq!(updated.title, "Hello, how are you?");
        assert_eq!(updated.message_count, 1);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_list_sessions() {
        let temp_dir = env::temp_dir().join("hive-test-list-sessions");
        // Clean up any leftover files from previous runs
        fs::remove_dir_all(&temp_dir).ok();
        fs::create_dir_all(&temp_dir).unwrap();

        let store = SessionStore {
            sessions_dir: temp_dir.clone(),
        };

        let _session1 = store.create_session().unwrap();
        let _session2 = store.create_session().unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_generate_title() {
        let short = "Short message";
        assert_eq!(generate_title_from_message(short), "Short message");

        let long = "This is a very long message that exceeds the maximum title length";
        let title = generate_title_from_message(long);
        assert!(title.len() <= 50);
        assert!(title.ends_with("..."));
    }
}
