use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub claude_session_id: Option<String>,
}

impl SessionMeta {
    pub fn new(title: &str) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        Self {
            id,
            title: title.to_string(),
            created_at: now,
            updated_at: now,
            message_count: 0,
            claude_session_id: None,
        }
    }
}

pub struct SessionStore {
    sessions_dir: PathBuf,
}

impl SessionStore {
    pub fn new() -> Result<Self> {
        let sessions_dir = Self::sessions_path()?;
        fs::create_dir_all(&sessions_dir)?;
        Ok(Self { sessions_dir })
    }

    fn sessions_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
        Ok(home.join(".hive").join("tui-sessions"))
    }

    pub fn list(&self) -> Result<Vec<SessionMeta>> {
        let mut sessions: Vec<SessionMeta> = Vec::new();
        if self.sessions_dir.exists() {
            for entry in fs::read_dir(&self.sessions_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(meta) = serde_json::from_str::<SessionMeta>(&content) {
                            sessions.push(meta);
                        }
                    }
                }
            }
        }
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    pub fn save(&self, meta: &SessionMeta) -> Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", meta.id));
        let content = serde_json::to_string_pretty(meta)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_meta_new() {
        let meta = SessionMeta::new("Test Session");
        assert_eq!(meta.title, "Test Session");
        assert_eq!(meta.message_count, 0);
        assert!(meta.claude_session_id.is_none());
        assert!(!meta.id.is_empty());
    }

    #[test]
    fn test_session_meta_serialization() {
        let meta = SessionMeta::new("Test");
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: SessionMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.title, "Test");
        assert_eq!(parsed.id, meta.id);
    }

    #[test]
    fn test_session_store_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore {
            sessions_dir: dir.path().to_path_buf(),
        };

        let meta = SessionMeta::new("Round Trip Test");
        store.save(&meta).unwrap();

        let sessions = store.list().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, "Round Trip Test");

        store.delete(&meta.id).unwrap();
        let sessions = store.list().unwrap();
        assert_eq!(sessions.len(), 0);
    }
}
