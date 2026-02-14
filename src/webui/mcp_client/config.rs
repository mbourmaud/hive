use std::collections::HashMap;
use std::path::Path;

use super::types::McpServerConfig;

/// Read MCP server configs from project-level .mcp.json and user-level settings.
pub fn load_mcp_configs(cwd: &Path) -> HashMap<String, McpServerConfig> {
    let mut configs = HashMap::new();

    // 1. Project-level: {cwd}/.mcp.json
    let project_mcp = cwd.join(".mcp.json");
    if let Ok(data) = std::fs::read_to_string(&project_mcp) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(servers) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
                for (name, config) in servers {
                    if let Ok(cfg) = serde_json::from_value::<McpServerConfig>(config.clone()) {
                        configs.insert(name.clone(), cfg);
                    }
                }
            }
        }
    }

    // 2. User-level: ~/.claude/settings.json
    if let Some(home) = dirs::home_dir() {
        let user_settings = home.join(".claude").join("settings.json");
        if let Ok(data) = std::fs::read_to_string(&user_settings) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(servers) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
                    for (name, config) in servers {
                        // Don't override project-level configs
                        if configs.contains_key(name) {
                            continue;
                        }
                        if let Ok(cfg) = serde_json::from_value::<McpServerConfig>(config.clone()) {
                            configs.insert(name.clone(), cfg);
                        }
                    }
                }
            }
        }
    }

    configs
}
