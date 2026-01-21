use crate::types::HiveConfig;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get worktree base directory with priority: ENV > local > global > default
pub fn get_worktree_base() -> Result<PathBuf> {
    // 1. Check environment variable
    if let Ok(env_base) = std::env::var("HIVE_WORKTREE_BASE") {
        return Ok(PathBuf::from(env_base));
    }

    // 2. Check local config
    if let Ok(local_config) = load_local_config() {
        if let Some(worktree_base) = local_config.worktree_base {
            return Ok(PathBuf::from(worktree_base));
        }
    }

    // 3. Check global config
    if let Ok(global_config) = load_global_config() {
        if let Some(worktree_base) = global_config.worktree_base {
            return Ok(PathBuf::from(worktree_base));
        }
    }

    // 4. Use default
    let home = dirs::home_dir().context("Failed to get home directory")?;
    Ok(home.join(".hive").join("worktrees"))
}

/// Get model with priority: ENV > local > global > default
pub fn get_model() -> String {
    // 1. Check environment variable
    if let Ok(env_model) = std::env::var("HIVE_MODEL") {
        return env_model;
    }

    // 2. Check local config
    if let Ok(local_config) = load_local_config() {
        if let Some(model) = local_config.default_model {
            return model;
        }
    }

    // 3. Check global config
    if let Ok(global_config) = load_global_config() {
        if let Some(model) = global_config.default_model {
            return model;
        }
    }

    // 4. Use default
    "sonnet".to_string()
}

/// Load local config from .hive/config.json
pub fn load_local_config() -> Result<HiveConfig> {
    let config_path = PathBuf::from(".hive").join("config.json");
    let contents = std::fs::read_to_string(&config_path)
        .context("Failed to read local config")?;
    let config: HiveConfig = serde_json::from_str(&contents)
        .context("Failed to parse local config")?;
    Ok(config)
}

/// Load global config from ~/.config/hive/config.json
pub fn load_global_config() -> Result<HiveConfig> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("hive");
    let config_path = config_dir.join("config.json");
    let contents = std::fs::read_to_string(&config_path)
        .context("Failed to read global config")?;
    let config: HiveConfig = serde_json::from_str(&contents)
        .context("Failed to parse global config")?;
    Ok(config)
}

/// Save local config to .hive/config.json
pub fn save_local_config(config: &HiveConfig) -> Result<()> {
    let config_path = PathBuf::from(".hive").join("config.json");
    let contents = serde_json::to_string_pretty(config)
        .context("Failed to serialize config")?;
    std::fs::write(&config_path, contents)
        .context("Failed to write local config")?;
    Ok(())
}

/// Save global config to ~/.config/hive/config.json
pub fn save_global_config(config: &HiveConfig) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("hive");
    std::fs::create_dir_all(&config_dir)
        .context("Failed to create config directory")?;
    let config_path = config_dir.join("config.json");
    let contents = serde_json::to_string_pretty(config)
        .context("Failed to serialize config")?;
    std::fs::write(&config_path, contents)
        .context("Failed to write global config")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_model_default() {
        // Save current env
        let saved = env::var("HIVE_MODEL").ok();
        env::remove_var("HIVE_MODEL");
        let model = get_model();
        assert_eq!(model, "sonnet");
        // Restore env
        if let Some(val) = saved {
            env::set_var("HIVE_MODEL", val);
        }
    }

    #[test]
    fn test_get_model_from_env() {
        // Save current env
        let saved = env::var("HIVE_MODEL").ok();
        env::set_var("HIVE_MODEL", "opus");
        let model = get_model();
        assert_eq!(model, "opus");
        // Restore env
        env::remove_var("HIVE_MODEL");
        if let Some(val) = saved {
            env::set_var("HIVE_MODEL", val);
        }
    }

    #[test]
    fn test_config_serialization() {
        let config = HiveConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: HiveConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.version, parsed.version);
    }
}
