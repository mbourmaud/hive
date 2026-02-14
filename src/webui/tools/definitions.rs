use crate::webui::anthropic::types::ToolDefinition;

/// Returns JSON schema definitions for all built-in tools.
pub fn builtin_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "Read".to_string(),
            description: "Read a file from the filesystem. Returns the file contents with line numbers.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "The line number to start reading from (1-based)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "The number of lines to read"
                    }
                },
                "required": ["file_path"]
            }),
        },
        ToolDefinition {
            name: "Write".to_string(),
            description: "Write content to a file, creating it if it doesn't exist or overwriting if it does.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                },
                "required": ["file_path", "content"]
            }),
        },
        ToolDefinition {
            name: "Edit".to_string(),
            description: "Perform exact string replacement in a file. The old_string must match exactly one location in the file.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "The exact text to find and replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "The replacement text"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default: false)"
                    }
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        },
        ToolDefinition {
            name: "Bash".to_string(),
            description: "Execute a bash command and return stdout and stderr. Commands run in the session's working directory.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in milliseconds (default: 120000, max: 600000)"
                    }
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "Grep".to_string(),
            description: "Search file contents using ripgrep. Returns matching files or content lines.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search in (defaults to cwd)"
                    },
                    "glob": {
                        "type": "string",
                        "description": "Glob pattern to filter files (e.g. \"*.rs\")"
                    },
                    "-i": {
                        "type": "boolean",
                        "description": "Case-insensitive search"
                    },
                    "output_mode": {
                        "type": "string",
                        "enum": ["content", "files_with_matches", "count"],
                        "description": "Output mode (default: files_with_matches)"
                    }
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "Glob".to_string(),
            description: "Find files matching a glob pattern. Returns matching file paths sorted by modification time.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The glob pattern to match files against (e.g. \"**/*.rs\")"
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory to search in (defaults to cwd)"
                    }
                },
                "required": ["pattern"]
            }),
        },
    ]
}
