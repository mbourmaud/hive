use anyhow::Result;
use std::fs;
use std::path::Path;

/// Write `.claude/settings.json` in the worktree with hooks that stream events
/// to `.hive/drones/{name}/events.ndjson`.
pub fn write_hooks_config(worktree: &Path, drone_name: &str) -> Result<()> {
    let project_root = std::env::current_dir()?;
    write_hooks_config_at(worktree, drone_name, &project_root)
}

/// Inner implementation that accepts an explicit project root (testable without current_dir).
pub fn write_hooks_config_at(worktree: &Path, drone_name: &str, project_root: &Path) -> Result<()> {
    let claude_dir = worktree.join(".claude");
    fs::create_dir_all(&claude_dir)?;
    let settings_path = claude_dir.join("settings.json");

    // Load existing settings if present
    let mut settings: serde_json::Value = if settings_path.exists() {
        let contents = fs::read_to_string(&settings_path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&contents).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Use absolute path for drone directory — $CLAUDE_PROJECT_DIR is unreliable
    let drone_dir = project_root.join(".hive/drones").join(drone_name);
    fs::create_dir_all(&drone_dir)?;
    let events_file = drone_dir
        .join("events.ndjson")
        .to_string_lossy()
        .to_string();
    let messages_file = drone_dir
        .join("messages.ndjson")
        .to_string_lossy()
        .to_string();

    let todos_file = drone_dir.join("todos.json").to_string_lossy().to_string();

    // Hooks: SendMessage (audit), TodoWrite (task tracking), Stop (exit detection).
    // Agent Teams task files only contain agent names, so we capture TodoWrite
    // from the team lead to get real task descriptions and progress.
    let hooks = serde_json::json!({
        "PreToolUse": [
            {
                "matcher": "SendMessage",
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"INPUT=$(cat); echo "$INPUT" | jq -c '{{event:"Message",ts:(now|todate),recipient:(.tool_input.recipient // ""),summary:(.tool_input.summary // (.tool_input.content // "" | .[0:200]))}}' >> {} && \
    echo "$INPUT" | jq -c '{{timestamp:(now|todate),from:"{}",to:(.tool_input.recipient // ""),content:(.tool_input.content // ""),summary:(.tool_input.summary // "")}}' >> {}"#,
                        events_file, drone_name, messages_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            },
            {
                "matcher": "TodoWrite",
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"cat | jq -c '[.tool_input.todos[]? | {{content:.content,status:(.status // "pending"),activeForm:(.activeForm // null)}}]' > {}"#,
                        todos_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            }
        ],
        "Stop": [
            {
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        "echo '{{\"event\":\"Stop\",\"ts\":\"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'\"}}' >> {}",
                        events_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            }
        ]
    });

    // Merge hooks into settings (overwrite hooks key)
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("hooks".to_string(), hooks);
    }

    let json = serde_json::to_string_pretty(&settings)?;
    fs::write(&settings_path, json)?;

    Ok(())
}

/// Read the origin remote URL from a git repo. Returns empty string if unavailable.
pub fn get_git_remote_url(worktree_path: &Path) -> String {
    std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(worktree_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Detect project languages from marker files in the worktree.
pub fn detect_project_languages(worktree: &Path) -> Vec<String> {
    const MARKERS: &[(&[&str], &str)] = &[
        (&["Cargo.toml"], "rust"),
        (&["package.json"], "node"),
        (&["go.mod"], "go"),
        (&["pyproject.toml", "requirements.txt"], "python"),
    ];

    MARKERS
        .iter()
        .filter(|(files, _)| files.iter().any(|f| worktree.join(f).exists()))
        .map(|(_, lang)| lang.to_string())
        .collect()
}
