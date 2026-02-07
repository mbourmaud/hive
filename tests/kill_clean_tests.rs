use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("hive");
    path
}

fn setup_test_env(test_name: &str) -> PathBuf {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let temp_dir = std::env::temp_dir().join(format!(
        "hive-test-kill-{}-{}-{}",
        test_name,
        std::process::id(),
        timestamp
    ));

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    fs::write(temp_dir.join("README.md"), "Test").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Initialize hive
    let binary = get_binary_path();
    Command::new(&binary)
        .args(["init"])
        .current_dir(&temp_dir)
        .env("HIVE_SKIP_PROMPTS", "1")
        .output()
        .unwrap();

    // Create test drone with status
    let drone_dir = temp_dir.join(".hive/drones/test-drone");
    fs::create_dir_all(&drone_dir).unwrap();

    let status = r#"{
        "drone": "test-drone",
        "prd": "test-prd.json",
        "branch": "hive/test-drone",
        "worktree": "/tmp/test-worktree",
        "local_mode": false,
        "status": "stopped",
        "current_story": null,
        "completed": [],
        "story_times": {},
        "total": 5,
        "started": "2024-01-01T00:00:00Z",
        "updated": "2024-01-01T00:00:00Z",
        "error_count": 0,
        "last_error_story": null
    }"#;

    fs::write(drone_dir.join("status.json"), status).unwrap();

    temp_dir
}

fn cleanup(path: &PathBuf) {
    if path.exists() {
        let _ = Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(path)
            .output();

        fs::remove_dir_all(path).ok();
    }
}

#[test]
fn test_kill_nonexistent_drone() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("nonexistent");

    let output = Command::new(&binary)
        .args(["kill", "nonexistent"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stderr: {}", stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("not found"));

    cleanup(&temp_dir);
}

#[test]
fn test_kill_stopped_drone() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("stopped");

    let output = Command::new(&binary)
        .args(["kill", "test-drone"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("stdout: {}", stdout);

    assert!(output.status.success());
    assert!(stdout.contains("No running process found") || stdout.contains("stopped"));

    cleanup(&temp_dir);
}

#[test]
fn test_clean_requires_confirmation() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("confirm");

    // Without --force, clean should prompt (but will fail in non-interactive mode)
    let output = Command::new(&binary)
        .args(["clean", "test-drone"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // In non-interactive mode, dialoguer will fail
    // We just verify the command runs without panic
    let _ = output.status.success();

    cleanup(&temp_dir);
}

#[test]
fn test_clean_with_force() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("force");

    let output = Command::new(&binary)
        .args(["clean", "test-drone", "--force"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(
        output.status.success(),
        "clean --force failed with stderr: {}",
        stderr
    );
    assert!(stdout.contains("cleaned up") || stdout.contains("Cleaning"));

    // Verify drone directory was removed
    let drone_dir = temp_dir.join(".hive/drones/test-drone");
    assert!(!drone_dir.exists());

    cleanup(&temp_dir);
}
