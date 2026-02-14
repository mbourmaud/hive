use axum::{extract::Query, Json};

use super::super::agents;

/// Build a system prompt that instructs Claude to use the available tools.
pub(super) fn build_default_system_prompt(cwd: &std::path::Path) -> String {
    let is_git = cwd.join(".git").is_dir();
    let platform = std::env::consts::OS;
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Load CLAUDE.md or opencode.md if present
    let mut context_files = String::new();
    for name in &["CLAUDE.md", "CLAUDE.local.md", "opencode.md", "OpenCode.md"] {
        let path = cwd.join(name);
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                context_files.push_str(&format!(
                    "\n<context_file path=\"{name}\">\n{content}\n</context_file>\n"
                ));
            }
        }
    }

    format!(
        r#"You are Hive, an interactive AI coding assistant with access to tools for reading, writing, and searching code.

You help users with software engineering tasks including reading files, writing code, debugging, searching codebases, and executing commands.

# Tools

You have access to these tools — use them to accomplish tasks:

- **Read**: Read files from the filesystem. Always read a file before modifying it.
- **Write**: Create or overwrite files.
- **Edit**: Make precise string replacements in files. Preferred over Write for modifying existing files.
- **Bash**: Execute shell commands. Use for git, build tools, tests, and other CLI operations.
- **Grep**: Search file contents using regex patterns (powered by ripgrep).
- **Glob**: Find files by name patterns.

# Guidelines

- When asked about files or code, use the Read tool to examine them — never guess at file contents.
- When asked to modify code, read the file first, then use Edit for precise changes.
- For searching, use Grep for content search and Glob for finding files by name.
- Use Bash for running tests, git operations, build commands, and other shell tasks.
- Be concise. Minimize output tokens. Answer with fewer than 4 lines unless the user asks for detail.
- Follow existing code conventions and patterns in the project.
- When multiple independent tool calls are needed, make them all at once for efficiency.

<env>
Working directory: {cwd}
Is git repo: {is_git}
Platform: {platform}
Date: {date}
</env>{context_files}"#,
        cwd = cwd.display(),
        is_git = is_git,
        platform = platform,
        date = date,
        context_files = context_files,
    )
}

/// Resolve slash commands: if user message starts with `/commandname`,
/// look up the command file and expand it.
pub(super) fn resolve_slash_command(text: &str, cwd: &std::path::Path) -> String {
    if !text.starts_with('/') {
        return text.to_string();
    }

    let parts: Vec<&str> = text.splitn(2, char::is_whitespace).collect();
    let command_name = parts[0].trim_start_matches('/');
    let arguments = parts.get(1).unwrap_or(&"").to_string();

    if command_name.is_empty() {
        return text.to_string();
    }

    // Search for command file in standard locations
    let search_dirs = [
        cwd.join(".claude").join("commands"),
        dirs::home_dir()
            .unwrap_or_default()
            .join(".claude")
            .join("commands"),
    ];

    for dir in &search_dirs {
        let md_path = dir.join(format!("{command_name}.md"));
        if md_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&md_path) {
                let expanded = content.replace("$ARGUMENTS", &arguments);
                return expanded;
            }
        }
    }

    // No command found — return original text
    text.to_string()
}

/// GET /api/chat/agents?cwd=...
pub async fn list_agents(Query(params): Query<AgentsQuery>) -> Json<Vec<agents::AgentProfile>> {
    let cwd = params
        .cwd
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let profiles = agents::discover_agents(&cwd);
    Json(profiles)
}

#[derive(Debug, serde::Deserialize)]
pub struct AgentsQuery {
    cwd: Option<String>,
}
