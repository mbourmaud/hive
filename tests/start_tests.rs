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
        "hive-test-start-{}-{}-{}",
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

    // Configure git user for commits
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

    // Create initial commit
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

    // Create a test PRD
    let prd = "{
        \"id\": \"test-prd\",
        \"title\": \"Test PRD\",
        \"description\": \"A test PRD\",
        \"version\": \"1.0.0\",
        \"created_at\": \"2024-01-01T00:00:00Z\",
        \"plan\": \"# Test Plan\\n\\nThis is a test plan for the test drone.\",
        \"tasks\": [{\"title\": \"Test task\", \"description\": \"A test task\"}]
    }";

    fs::write(temp_dir.join(".hive/prds/prd-test-drone.json"), prd).unwrap();

    temp_dir
}

fn cleanup(path: &PathBuf) {
    if path.exists() {
        // Clean up worktrees first
        let _ = Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(path)
            .output();

        fs::remove_dir_all(path).ok();
    }
}

#[test]
fn test_start_local_mode() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("local");

    let output = Command::new(&binary)
        .args(["start", "test-drone", "--local", "--dry-run"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(output.status.success());
    assert!(stdout.contains("test-drone"));
    assert!(stdout.contains("Dry run"));

    // Verify status.json was created
    assert!(temp_dir
        .join(".hive/drones/test-drone/status.json")
        .exists());

    cleanup(&temp_dir);
}

#[test]
fn test_start_no_prd() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("noprd");

    // Remove PRD
    fs::remove_file(temp_dir.join(".hive/prds/prd-test-drone.json")).unwrap();

    let output = Command::new(&binary)
        .args(["start", "nonexistent-drone", "--local", "--dry-run"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stderr: {}", stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("No plan found"));

    cleanup(&temp_dir);
}

#[test]
fn test_start_creates_status() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("status");

    let output = Command::new(&binary)
        .args(["start", "test-drone", "--local", "--dry-run"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify status.json content
    let status_path = temp_dir.join(".hive/drones/test-drone/status.json");
    assert!(status_path.exists());

    let status_content = fs::read_to_string(&status_path).unwrap();
    assert!(status_content.contains("test-drone"));
    assert!(status_content.contains("starting"));

    cleanup(&temp_dir);
}
