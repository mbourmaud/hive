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
        "hive-test-status-{}-{}-{}",
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

    // Initialize hive
    let binary = get_binary_path();
    Command::new(&binary)
        .args(["init"])
        .current_dir(&temp_dir)
        .env("HIVE_SKIP_PROMPTS", "1")
        .output()
        .unwrap();

    // Create a test drone status
    let drones_dir = temp_dir.join(".hive/drones/test-drone");
    fs::create_dir_all(&drones_dir).unwrap();

    let status = r#"{
        "drone": "test-drone",
        "prd": "test-prd.json",
        "branch": "hive/test",
        "worktree": "/tmp/test-worktree",
        "local_mode": false,
        "status": "in_progress",
        "current_task": "TEST-001",
        "completed": ["TEST-000"],
        "story_times": {},
        "total": 5,
        "started": "2024-01-01T00:00:00Z",
        "updated": "2024-01-01T00:00:00Z",
        "error_count": 0,
        "last_error": null
    }"#;

    fs::write(drones_dir.join("status.json"), status).unwrap();

    temp_dir
}

fn cleanup(path: &PathBuf) {
    if path.exists() {
        fs::remove_dir_all(path).ok();
    }
}

#[test]
fn test_status_shows_drones() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("shows");

    let output = Command::new(&binary)
        .args(["list"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("stdout: {}", stdout);

    assert!(output.status.success());
    assert!(stdout.contains("test-drone"));
    assert!(stdout.contains("in_progress"));

    cleanup(&temp_dir);
}

#[test]
fn test_status_no_drones() {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let binary = get_binary_path();
    let temp_dir = std::env::temp_dir().join(format!(
        "hive-test-empty-{}-{}",
        std::process::id(),
        timestamp
    ));

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    // Initialize git and hive but no drones
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    Command::new(&binary)
        .args(["init"])
        .current_dir(&temp_dir)
        .env("HIVE_SKIP_PROMPTS", "1")
        .output()
        .unwrap();

    let output = Command::new(&binary)
        .args(["list"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("stdout: {}", stdout);

    assert!(output.status.success());
    assert!(stdout.contains("No drones found"));

    cleanup(&temp_dir);
}
