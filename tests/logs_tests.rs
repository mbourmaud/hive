use std::fs;
use std::io::Write;
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
        "hive-test-logs-{}-{}-{}",
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

    // Create test drone directory with activity log
    let drone_dir = temp_dir.join(".hive/drones/test-drone");
    fs::create_dir_all(&drone_dir).unwrap();

    let log_path = drone_dir.join("activity.log");
    let mut log_file = fs::File::create(&log_path).unwrap();
    writeln!(log_file, "[10:00:00] 🔨 Début TEST-001").unwrap();
    writeln!(log_file, "[10:00:05] 💾 Commit TEST-001").unwrap();
    writeln!(log_file, "[10:00:10] ✅ TEST-001 terminée").unwrap();
    writeln!(log_file, "[10:00:15] 🔨 Début TEST-002").unwrap();

    temp_dir
}

fn cleanup(path: &PathBuf) {
    if path.exists() {
        fs::remove_dir_all(path).ok();
    }
}

#[test]
fn test_logs_show_team_conversation() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("show");

    let output = Command::new(&binary)
        .args(["logs", "test-drone"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("stdout: {}", stdout);

    assert!(output.status.success());
    // Now always shows team conversation
    assert!(stdout.contains("Team Conversation"));

    cleanup(&temp_dir);
}

#[test]
fn test_logs_nonexistent_drone() {
    let binary = get_binary_path();
    let temp_dir = setup_test_env("noexist");

    let output = Command::new(&binary)
        .args(["logs", "nonexistent"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stderr: {}", stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("not found"));

    cleanup(&temp_dir);
}
