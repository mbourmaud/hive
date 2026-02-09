use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::app::{ChatMessage, MessageRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub title: String,
    pub model: String,
    pub claude_session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_cost_usd: f64,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

impl PersistedMessage {
    pub fn from_chat_message(msg: &ChatMessage) -> Self {
        Self {
            role: match msg.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::System => "system".to_string(),
                MessageRole::Error => "error".to_string(),
            },
            content: msg.content.clone(),
            timestamp: msg.timestamp.to_rfc3339(),
        }
    }

    pub fn to_chat_message(&self) -> ChatMessage {
        ChatMessage {
            role: match self.role.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                _ => MessageRole::Error,
            },
            content: self.content.clone(),
            timestamp: chrono::DateTime::parse_from_rfc3339(&self.timestamp)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

pub struct SessionManager {
    sessions_dir: PathBuf,
    current_session: Option<SessionMetadata>,
}

fn find_hive_dir() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let hive = dir.join(".hive");
        if hive.is_dir() {
            return Ok(hive);
        }
        if !dir.pop() {
            let hive = std::env::current_dir()?.join(".hive");
            std::fs::create_dir_all(&hive)?;
            return Ok(hive);
        }
    }
}

impl SessionManager {
    pub fn new() -> Result<Self> {
        let hive_dir = find_hive_dir()?;
        let sessions_dir = hive_dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir)?;
        Ok(Self {
            sessions_dir,
            current_session: None,
        })
    }

    pub fn create_session(&mut self, model: &str) -> Result<SessionMetadata> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let metadata = SessionMetadata {
            id: id.clone(),
            title: "New Chat".to_string(),
            model: model.to_string(),
            claude_session_id: None,
            created_at: now.clone(),
            updated_at: now,
            input_tokens: 0,
            output_tokens: 0,
            total_cost_usd: 0.0,
            message_count: 0,
        };

        let session_dir = self.sessions_dir.join(&id);
        std::fs::create_dir_all(&session_dir)?;

        self.save_metadata(&metadata)?;
        self.save_messages(&id, &[])?;
        self.current_session = Some(metadata.clone());

        Ok(metadata)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        let mut sessions = Vec::new();
        let entries =
            std::fs::read_dir(&self.sessions_dir).context("Failed to read sessions directory")?;

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let meta_path = entry.path().join("session.json");
            if !meta_path.exists() {
                continue;
            }
            let data = std::fs::read_to_string(&meta_path)
                .with_context(|| format!("Failed to read {}", meta_path.display()))?;
            match serde_json::from_str::<SessionMetadata>(&data) {
                Ok(meta) => sessions.push(meta),
                Err(_) => continue,
            }
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    pub fn load_session(&mut self, id: &str) -> Result<(SessionMetadata, Vec<PersistedMessage>)> {
        let session_dir = self.sessions_dir.join(id);
        let meta_path = session_dir.join("session.json");
        let data = std::fs::read_to_string(&meta_path)
            .with_context(|| format!("Failed to read session metadata: {}", meta_path.display()))?;
        let metadata: SessionMetadata =
            serde_json::from_str(&data).context("Failed to parse session metadata")?;
        let messages = self.load_messages(id)?;
        self.current_session = Some(metadata.clone());
        Ok((metadata, messages))
    }

    pub fn resume_or_create(
        &mut self,
        model: &str,
    ) -> Result<(SessionMetadata, Vec<PersistedMessage>)> {
        let sessions = self.list_sessions()?;
        if let Some(most_recent) = sessions.into_iter().next() {
            self.load_session(&most_recent.id)
        } else {
            let metadata = self.create_session(model)?;
            Ok((metadata, Vec::new()))
        }
    }

    pub fn save_metadata(&self, metadata: &SessionMetadata) -> Result<()> {
        let session_dir = self.sessions_dir.join(&metadata.id);
        std::fs::create_dir_all(&session_dir)?;
        let meta_path = session_dir.join("session.json");
        let data = serde_json::to_string_pretty(metadata)
            .context("Failed to serialize session metadata")?;
        std::fs::write(&meta_path, data)
            .with_context(|| format!("Failed to write {}", meta_path.display()))?;
        Ok(())
    }

    pub fn append_message(&self, session_id: &str, message: &PersistedMessage) -> Result<()> {
        let mut messages = self.load_messages(session_id)?;
        messages.push(message.clone());
        self.save_messages(session_id, &messages)
    }

    pub fn save_messages(&self, session_id: &str, messages: &[PersistedMessage]) -> Result<()> {
        let session_dir = self.sessions_dir.join(session_id);
        let msg_path = session_dir.join("messages.json");
        let data =
            serde_json::to_string_pretty(messages).context("Failed to serialize messages")?;
        std::fs::write(&msg_path, data)
            .with_context(|| format!("Failed to write {}", msg_path.display()))?;
        Ok(())
    }

    pub fn load_messages(&self, session_id: &str) -> Result<Vec<PersistedMessage>> {
        let session_dir = self.sessions_dir.join(session_id);
        let msg_path = session_dir.join("messages.json");
        if !msg_path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read_to_string(&msg_path)
            .with_context(|| format!("Failed to read {}", msg_path.display()))?;
        let messages: Vec<PersistedMessage> =
            serde_json::from_str(&data).context("Failed to parse messages")?;
        Ok(messages)
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        let session_dir = self.sessions_dir.join(id);
        if session_dir.exists() {
            std::fs::remove_dir_all(&session_dir)
                .with_context(|| format!("Failed to delete session {}", id))?;
        }
        Ok(())
    }

    pub fn update_usage(
        &mut self,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
    ) -> Result<()> {
        if let Some(ref mut meta) = self.current_session {
            meta.input_tokens += input_tokens;
            meta.output_tokens += output_tokens;
            meta.total_cost_usd += cost_usd;
            meta.updated_at = chrono::Utc::now().to_rfc3339();
        }
        if let Some(meta) = self.current_session.as_ref() {
            self.save_metadata(meta)?;
        }
        Ok(())
    }

    pub fn auto_title(messages: &[PersistedMessage]) -> String {
        for msg in messages {
            if msg.role == "user" {
                let content = msg.content.trim();
                if content.len() <= 50 {
                    return content.to_string();
                }
                return format!("{}...", &content[..50]);
            }
        }
        "New Chat".to_string()
    }

    pub fn current(&self) -> Option<&SessionMetadata> {
        self.current_session.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_title_short() {
        let msgs = vec![PersistedMessage {
            role: "user".to_string(),
            content: "Hello world".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        }];
        assert_eq!(SessionManager::auto_title(&msgs), "Hello world");
    }

    #[test]
    fn test_auto_title_long() {
        let msgs = vec![PersistedMessage {
            role: "user".to_string(),
            content: "This is a very long message that should be truncated to a reasonable length for display".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        }];
        let title = SessionManager::auto_title(&msgs);
        assert!(title.len() <= 53); // 50 + "..."
    }

    #[test]
    fn test_auto_title_empty() {
        let msgs: Vec<PersistedMessage> = vec![];
        assert_eq!(SessionManager::auto_title(&msgs), "New Chat");
    }

    #[test]
    fn test_persisted_message_roundtrip() {
        let chat_msg = ChatMessage {
            role: MessageRole::User,
            content: "test message".to_string(),
            timestamp: chrono::Utc::now(),
        };
        let persisted = PersistedMessage::from_chat_message(&chat_msg);
        let restored = persisted.to_chat_message();
        assert_eq!(restored.role, MessageRole::User);
        assert_eq!(restored.content, "test message");
    }

    #[test]
    fn test_session_crud_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let hive_dir = temp.path().join(".hive");
        std::fs::create_dir_all(&hive_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let mut mgr = SessionManager::new().unwrap();

        // Create session
        let meta = mgr.create_session("sonnet").unwrap();
        assert_eq!(meta.title, "New Chat");
        assert_eq!(meta.model, "sonnet");

        // List sessions
        let sessions = mgr.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);

        // Append message
        let msg = PersistedMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        mgr.append_message(&meta.id, &msg).unwrap();

        // Load messages
        let msgs = mgr.load_messages(&meta.id).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Hello");

        // Delete session
        mgr.delete_session(&meta.id).unwrap();
        let sessions = mgr.list_sessions().unwrap();
        assert_eq!(sessions.len(), 0);

        // Restore original dir
        std::env::set_current_dir(original_dir).unwrap();
    }
}
