use super::*;
use tempfile::TempDir;

#[test]
fn test_write_hooks_config_creates_settings() {
    let dir = TempDir::new().unwrap();
    write_hooks_config(dir.path(), "test-drone").unwrap();

    let settings_path = dir.path().join(".claude").join("settings.json");
    assert!(settings_path.exists());

    let contents = fs::read_to_string(&settings_path).unwrap();
    let settings: serde_json::Value = serde_json::from_str(&contents).unwrap();

    assert!(settings.get("hooks").is_some());
    let hooks = settings.get("hooks").unwrap();
    assert!(hooks.get("PreToolUse").is_some());
    assert!(hooks.get("Stop").is_some());

    // PreToolUse has SendMessage + TodoWrite hooks
    let pre_tool = hooks.get("PreToolUse").unwrap().as_array().unwrap();
    assert_eq!(pre_tool.len(), 2);

    // Verify SendMessage matcher exists and contains drone name
    let send_matcher = &pre_tool[0];
    assert_eq!(send_matcher["matcher"].as_str().unwrap(), "SendMessage");
    let command = send_matcher["hooks"][0]
        .get("command")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(command.contains("test-drone"));
    assert!(command.contains("events.ndjson"));

    // Verify TodoWrite matcher exists and writes todos.json
    let todo_matcher = &pre_tool[1];
    assert_eq!(todo_matcher["matcher"].as_str().unwrap(), "TodoWrite");
    let todo_cmd = todo_matcher["hooks"][0]
        .get("command")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(todo_cmd.contains("todos.json"));

    // PostToolUse and SubagentStart/Stop hooks are removed
    assert!(hooks.get("PostToolUse").is_none());
    assert!(hooks.get("SubagentStart").is_none());
    assert!(hooks.get("SubagentStop").is_none());
}

#[test]
fn test_write_hooks_config_merges_existing() {
    let dir = TempDir::new().unwrap();
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();

    // Write existing settings
    let existing = serde_json::json!({
        "model": "opus",
        "permissions": {"allow": ["Bash"]}
    });
    fs::write(
        claude_dir.join("settings.json"),
        serde_json::to_string_pretty(&existing).unwrap(),
    )
    .unwrap();

    write_hooks_config(dir.path(), "merge-test").unwrap();

    let contents = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let settings: serde_json::Value = serde_json::from_str(&contents).unwrap();

    // Original keys preserved
    assert_eq!(settings.get("model").unwrap().as_str().unwrap(), "opus");
    assert!(settings.get("permissions").is_some());

    // Hooks added
    assert!(settings.get("hooks").is_some());
}

#[test]
fn test_write_hooks_config_async_and_timeout() {
    let dir = TempDir::new().unwrap();
    write_hooks_config(dir.path(), "timeout-test").unwrap();

    let contents = fs::read_to_string(dir.path().join(".claude").join("settings.json")).unwrap();
    let settings: serde_json::Value = serde_json::from_str(&contents).unwrap();

    // Check PreToolUse hooks have async + timeout
    let pre_tool = settings["hooks"]["PreToolUse"].as_array().unwrap();
    for entry in pre_tool {
        let hooks = entry["hooks"].as_array().unwrap();
        for hook in hooks {
            assert!(hook.get("async").unwrap().as_bool().unwrap());
            assert_eq!(hook.get("timeout").unwrap().as_u64().unwrap(), 5);
        }
    }

    // Check Stop hooks
    let stop_hooks = settings["hooks"]["Stop"].as_array().unwrap();
    for entry in stop_hooks {
        let hooks = entry["hooks"].as_array().unwrap();
        for hook in hooks {
            assert!(hook.get("async").unwrap().as_bool().unwrap());
            assert_eq!(hook.get("timeout").unwrap().as_u64().unwrap(), 5);
        }
    }
}
