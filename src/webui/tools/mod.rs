pub mod bash;
pub mod definitions;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod output;
pub mod read;
pub mod sandbox;
pub mod write;

use std::path::Path;

/// Result of executing a tool: content string and whether it was an error.
pub struct ToolExecutionResult {
    pub content: String,
    pub is_error: bool,
}

/// Execute a built-in tool by name. Returns None if the tool name is not recognized.
pub async fn execute_tool(
    name: &str,
    input: &serde_json::Value,
    cwd: &Path,
) -> Option<ToolExecutionResult> {
    let result = match name {
        "Read" => read::execute(input, cwd).await,
        "Write" => write::execute(input, cwd).await,
        "Edit" => edit::execute(input, cwd).await,
        "Bash" => bash::execute(input, cwd).await,
        "Grep" => grep::execute(input, cwd).await,
        "Glob" => glob::execute(input, cwd).await,
        _ => return None,
    };

    Some(match result {
        Ok(content) => ToolExecutionResult {
            content,
            is_error: false,
        },
        Err(e) => ToolExecutionResult {
            content: format!("{e:#}"),
            is_error: true,
        },
    })
}
