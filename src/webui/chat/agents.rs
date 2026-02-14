use serde::Serialize;
use std::path::Path;

/// An agent profile loaded from a `.claude/agents/*.md` file.
#[derive(Debug, Clone, Serialize)]
pub struct AgentProfile {
    /// Filename without extension (used as identifier)
    pub slug: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    /// Body of the .md file (used as system prompt)
    pub system_prompt: String,
}

/// Discover agent profiles from `.claude/agents/` in cwd and home dir.
pub fn discover_agents(cwd: &Path) -> Vec<AgentProfile> {
    let mut agents = Vec::new();
    let mut seen_slugs = std::collections::HashSet::new();

    // Project-level agents take priority
    let project_dir = cwd.join(".claude").join("agents");
    load_agents_from_dir(&project_dir, &mut agents, &mut seen_slugs);

    // User-level agents
    if let Some(home) = dirs::home_dir() {
        let user_dir = home.join(".claude").join("agents");
        load_agents_from_dir(&user_dir, &mut agents, &mut seen_slugs);
    }

    agents
}

fn load_agents_from_dir(
    dir: &Path,
    agents: &mut Vec<AgentProfile>,
    seen: &mut std::collections::HashSet<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let slug = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        if seen.contains(&slug) {
            continue;
        }
        if let Some(profile) = parse_agent_file(&path, &slug) {
            seen.insert(slug);
            agents.push(profile);
        }
    }
}

/// Parse a single agent .md file with optional YAML frontmatter.
///
/// Expected format:
/// ```text
/// ---
/// name: "Frontend Developer"
/// description: "Specializes in React and TypeScript"
/// model: "sonnet"
/// allowed_tools: ["Read", "Write", "Edit", "Bash", "Grep", "Glob"]
/// ---
/// System prompt body here...
/// ```
fn parse_agent_file(path: &Path, slug: &str) -> Option<AgentProfile> {
    let content = std::fs::read_to_string(path).ok()?;

    let (frontmatter, body) = if let Some(rest) = content.strip_prefix("---") {
        // Find the closing ---
        if let Some(end) = rest.find("\n---") {
            let yaml = &rest[..end];
            let body = rest[end + 4..].trim_start().to_string();
            (Some(yaml.to_string()), body)
        } else {
            (None, content)
        }
    } else {
        (None, content)
    };

    let mut name = slug.replace('-', " ");
    // Capitalize first letter of each word
    name = name
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut description = String::new();
    let mut model = None;
    let mut allowed_tools = Vec::new();

    if let Some(yaml_str) = frontmatter {
        if let Ok(yaml) = serde_json::from_str::<serde_json::Value>(
            // Quick YAML-to-JSON: parse as serde_yaml if available, else basic key-value
            &yaml_to_json_basic(&yaml_str),
        ) {
            if let Some(n) = yaml.get("name").and_then(|v| v.as_str()) {
                name = n.to_string();
            }
            if let Some(d) = yaml.get("description").and_then(|v| v.as_str()) {
                description = d.to_string();
            }
            if let Some(m) = yaml.get("model").and_then(|v| v.as_str()) {
                model = Some(m.to_string());
            }
            if let Some(tools) = yaml.get("allowed_tools").and_then(|v| v.as_array()) {
                allowed_tools = tools
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
        }
    }

    Some(AgentProfile {
        slug: slug.to_string(),
        name,
        description,
        model,
        allowed_tools,
        system_prompt: body,
    })
}

/// Minimal YAML frontmatter → JSON converter.
/// Handles simple key: value and key: [array] syntax.
fn yaml_to_json_basic(yaml: &str) -> String {
    let mut map = serde_json::Map::new();

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim().trim_matches('"');
            let value = value.trim();

            if value.starts_with('[') {
                // Array value: ["a", "b", "c"]
                if let Ok(arr) = serde_json::from_str::<serde_json::Value>(value) {
                    map.insert(key.to_string(), arr);
                    continue;
                }
            }

            // String value (strip surrounding quotes)
            let val = value.trim_matches('"').trim_matches('\'');
            map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
        }
    }

    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}
