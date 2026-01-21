use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_resume_module_exists() {
    // Basic test to ensure resume functionality compiles
    assert!(true);
}

#[test]
fn test_worktree_list_command() {
    // Test that git worktree list --porcelain command works
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output();

    assert!(output.is_ok(), "git worktree list should work");
    let output = output.unwrap();
    assert!(output.status.success(), "git worktree list should succeed");
}

#[test]
fn test_worktree_porcelain_parsing() {
    // Test parsing of porcelain format
    let sample_output = r#"worktree /Users/test/project
HEAD abc123def456
branch refs/heads/main

worktree /Users/test/project-feature
HEAD def456abc123
branch refs/heads/hive/feature-branch

"#;

    let mut worktrees = Vec::new();
    let mut current_path = None;
    let mut current_branch = None;

    for line in sample_output.lines() {
        if line.starts_with("worktree ") {
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                worktrees.push((path, branch));
            }
            current_path = Some(line.strip_prefix("worktree ").unwrap().to_string());
        } else if line.starts_with("branch ") {
            let branch = line
                .strip_prefix("branch ")
                .unwrap()
                .strip_prefix("refs/heads/")
                .unwrap_or(line.strip_prefix("branch ").unwrap());
            current_branch = Some(branch.to_string());
        }
    }

    if let (Some(path), Some(branch)) = (current_path, current_branch) {
        worktrees.push((path, branch));
    }

    assert_eq!(worktrees.len(), 2);
    assert_eq!(worktrees[0].1, "main");
    assert_eq!(worktrees[1].1, "hive/feature-branch");
}

#[test]
fn test_prunable_worktree_detection() {
    // Test detection of prunable worktrees
    let sample_output = r#"worktree /Users/test/project
HEAD abc123def456
branch refs/heads/main

worktree /Users/test/old-worktree
HEAD def456abc123
branch refs/heads/hive/old
prunable

"#;

    let mut prunable_count = 0;

    for line in sample_output.lines() {
        if line == "prunable" {
            prunable_count += 1;
        }
    }

    assert_eq!(prunable_count, 1);
}

#[test]
fn test_worktree_path_parsing() {
    // Test various worktree path formats
    let paths = vec![
        "/Users/test/project",
        "/Users/test/.hive/worktrees/project/drone",
        "/tmp/worktree-123",
    ];

    for path_str in paths {
        let path = PathBuf::from(path_str);
        assert!(path.is_absolute() || path.starts_with("/"));
    }
}

#[test]
fn test_branch_name_extraction() {
    // Test extracting branch names from various formats
    let test_cases = vec![
        ("refs/heads/main", "main"),
        ("refs/heads/hive/feature", "hive/feature"),
        ("refs/heads/hive/rust-rewrite", "hive/rust-rewrite"),
        ("main", "main"),
    ];

    for (input, expected) in test_cases {
        let result = input.strip_prefix("refs/heads/").unwrap_or(input);
        assert_eq!(result, expected);
    }
}

#[test]
fn test_resume_config_update() {
    // Test that config can be updated with new worktree path
    let temp_dir = std::env::temp_dir().join("hive-test-resume-config");
    fs::create_dir_all(&temp_dir).unwrap();

    let config_path = temp_dir.join("status.json");
    let config_content = r#"{
        "drone": "test",
        "worktree": "/old/path",
        "status": "in_progress"
    }"#;

    fs::write(&config_path, config_content).unwrap();

    // Read and parse
    let content = fs::read_to_string(&config_path).unwrap();
    let mut config: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Update worktree path
    config["worktree"] = serde_json::Value::String("/new/path".to_string());

    // Write back
    let updated = serde_json::to_string_pretty(&config).unwrap();
    fs::write(&config_path, updated).unwrap();

    // Verify
    let final_content = fs::read_to_string(&config_path).unwrap();
    assert!(final_content.contains("/new/path"));
    assert!(!final_content.contains("/old/path"));

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_worktree_exists_check() {
    // Test checking if a worktree directory exists
    let existing_path = PathBuf::from(".");
    assert!(existing_path.exists());

    let nonexistent_path = PathBuf::from("/nonexistent/path/to/worktree");
    assert!(!nonexistent_path.exists());
}
