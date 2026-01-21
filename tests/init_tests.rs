use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test executable name
    path.pop(); // Remove 'deps' directory
    path.push("hive");
    path
}

fn setup_temp_repo() -> PathBuf {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let temp_dir =
        std::env::temp_dir().join(format!("hive-test-{}-{}", std::process::id(), timestamp));

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

    temp_dir
}

fn cleanup_temp_repo(path: &PathBuf) {
    if path.exists() {
        fs::remove_dir_all(path).ok();
    }
}

#[test]
fn test_init_creates_structure() {
    let binary = get_binary_path();
    let temp_dir = setup_temp_repo();

    // Run init command
    let output = Command::new(&binary)
        .args(["init"])
        .current_dir(&temp_dir)
        .env("HIVE_SKIP_PROMPTS", "1")
        .output()
        .unwrap();

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Verify structure was created
    assert!(temp_dir.join(".hive").exists());
    assert!(temp_dir.join(".hive/prds").exists());
    assert!(temp_dir.join(".hive/drones").exists());
    assert!(temp_dir.join(".hive/config.json").exists());

    // Verify .gitignore was updated
    let gitignore_content = fs::read_to_string(temp_dir.join(".gitignore")).unwrap();
    assert!(gitignore_content.contains(".hive/"));

    cleanup_temp_repo(&temp_dir);
}

#[test]
fn test_init_is_idempotent() {
    let binary = get_binary_path();
    let temp_dir = setup_temp_repo();

    // Run init twice
    for _ in 0..2 {
        let output = Command::new(&binary)
            .args(["init"])
            .current_dir(&temp_dir)
            .env("HIVE_SKIP_PROMPTS", "1")
            .output()
            .unwrap();

        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

        assert!(output.status.success());
    }

    // Verify structure exists
    assert!(temp_dir.join(".hive/config.json").exists());

    cleanup_temp_repo(&temp_dir);
}

#[test]
fn test_init_fails_without_git() {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let binary = get_binary_path();
    let temp_dir = std::env::temp_dir().join(format!(
        "hive-test-nogit-{}-{}",
        std::process::id(),
        timestamp
    ));

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    // Run init without git repo
    let output = Command::new(&binary)
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Should fail
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Not a git repository"));

    cleanup_temp_repo(&temp_dir);
}
