use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub default_model: String,
    pub models: Vec<ModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub display_name: String,
    pub provider: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            default_model: "claude-sonnet-4-5-20250929".to_string(),
            models: vec![
                ModelConfig {
                    id: "claude-sonnet-4-5-20250929".to_string(),
                    display_name: "Sonnet".to_string(),
                    provider: "anthropic".to_string(),
                },
                ModelConfig {
                    id: "claude-opus-4-6".to_string(),
                    display_name: "Opus".to_string(),
                    provider: "anthropic".to_string(),
                },
                ModelConfig {
                    id: "claude-haiku-4-5-20251001".to_string(),
                    display_name: "Haiku".to_string(),
                    provider: "anthropic".to_string(),
                },
            ],
        }
    }
}

impl ProviderConfig {
    /// Load provider config from .hive/config.json or use defaults
    pub fn load() -> Self {
        // Try to read default_model from .hive/config.json
        let config_path = find_config_path();
        if let Some(path) = config_path {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(model) = v.get("default_model").and_then(|m| m.as_str()) {
                        return ProviderConfig {
                            default_model: resolve_model_id(model),
                            ..Default::default()
                        };
                    }
                }
            }
        }
        Self::default()
    }

    /// Get the display name for a model ID
    pub fn display_name<'a>(&'a self, model_id: &'a str) -> &'a str {
        self.models
            .iter()
            .find(|m| m.id == model_id)
            .map(|m| m.display_name.as_str())
            .unwrap_or(model_id)
    }
}

/// Resolve short model names to full IDs
pub fn resolve_model_id(name: &str) -> String {
    match name {
        "sonnet" => "claude-sonnet-4-5-20250929".to_string(),
        "opus" => "claude-opus-4-6".to_string(),
        "haiku" => "claude-haiku-4-5-20251001".to_string(),
        other => other.to_string(),
    }
}

fn find_config_path() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let config = dir.join(".hive").join("config.json");
        if config.exists() {
            return Some(config);
        }
        if !dir.pop() {
            return None;
        }
    }
}
