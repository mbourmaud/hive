//! Two-tier tool loading: Core tools always sent, Deferred (MCP) tools loaded on demand.
//!
//! This saves ~6,000 tokens/turn by not sending 30+ MCP tool definitions
//! until Claude actually needs them.

use crate::webui::anthropic::types::ToolDefinition;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolTier {
    /// Always included: built-in tools + ToolSearch + SessionSearch + RecentSessions.
    Core,
    /// MCP tools — only included when deferred tier is activated.
    Deferred,
}

/// Classify a tool by name. MCP tools (containing "__") are Deferred.
pub fn classify_tool(name: &str) -> ToolTier {
    if name.contains("__") {
        ToolTier::Deferred
    } else {
        ToolTier::Core
    }
}

/// Filter tools: always include Core, include Deferred only if the flag is set.
pub fn filter_by_tier(tools: &[ToolDefinition], include_deferred: bool) -> Vec<ToolDefinition> {
    tools
        .iter()
        .filter(|t| include_deferred || classify_tool(&t.name) == ToolTier::Core)
        .cloned()
        .collect()
}

/// Detect whether the user's text suggests MCP tools are needed.
/// Checks for MCP server name mentions and generic browser/navigation patterns.
pub fn should_activate_deferred(user_text: &str, mcp_server_names: &[String]) -> bool {
    let lower = user_text.to_lowercase();

    // Check explicit MCP server names (e.g. "playwright", "context7", "chrome-devtools")
    for name in mcp_server_names {
        if lower.contains(&name.to_lowercase()) {
            return true;
        }
    }

    // Generic patterns that imply browser/MCP tool usage
    const PATTERNS: &[&str] = &[
        "browser",
        "screenshot",
        "navigate",
        "chrome",
        "mcp",
        "playwright",
        "context7",
        "web page",
        "webpage",
        "click on",
        "open the page",
        "devtools",
    ];

    PATTERNS.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_core_tools() {
        assert_eq!(classify_tool("Read"), ToolTier::Core);
        assert_eq!(classify_tool("Bash"), ToolTier::Core);
        assert_eq!(classify_tool("SessionSearch"), ToolTier::Core);
        assert_eq!(classify_tool("ToolSearch"), ToolTier::Core);
    }

    #[test]
    fn test_classify_mcp_tools() {
        assert_eq!(classify_tool("mcp__playwright__click"), ToolTier::Deferred);
        assert_eq!(classify_tool("mcp__context7__query"), ToolTier::Deferred);
    }

    #[test]
    fn test_filter_excludes_deferred_when_inactive() {
        let tools = vec![
            ToolDefinition {
                name: "Read".into(),
                description: String::new(),
                input_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "mcp__playwright__click".into(),
                description: String::new(),
                input_schema: serde_json::json!({}),
            },
        ];
        let filtered = filter_by_tier(&tools, false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Read");
    }

    #[test]
    fn test_filter_includes_deferred_when_active() {
        let tools = vec![
            ToolDefinition {
                name: "Read".into(),
                description: String::new(),
                input_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "mcp__playwright__click".into(),
                description: String::new(),
                input_schema: serde_json::json!({}),
            },
        ];
        let filtered = filter_by_tier(&tools, true);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_should_activate_on_server_name() {
        let servers = vec!["playwright".to_string(), "context7".to_string()];
        assert!(should_activate_deferred("Use playwright to test", &servers));
        assert!(should_activate_deferred("Check context7 docs", &servers));
        assert!(!should_activate_deferred("Fix the login bug", &servers));
    }

    #[test]
    fn test_should_activate_on_generic_patterns() {
        let servers: Vec<String> = vec![];
        assert!(should_activate_deferred(
            "Take a screenshot of the page",
            &servers
        ));
        assert!(should_activate_deferred(
            "Navigate to the homepage",
            &servers
        ));
        assert!(should_activate_deferred("Open chrome devtools", &servers));
        assert!(!should_activate_deferred(
            "Refactor the parser module",
            &servers
        ));
    }
}
