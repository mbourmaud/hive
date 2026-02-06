use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("hive");
    path
}

#[test]
fn test_version_flag() {
    let binary = get_binary_path();
    let output = Command::new(&binary)
        .arg("--version")
        .output()
        .expect("Failed to execute binary");

    assert!(
        output.status.success(),
        "Command should exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = env!("CARGO_PKG_VERSION");

    // Should output "hive X.Y.Z"
    assert!(
        stdout.contains("hive"),
        "Output should contain 'hive'"
    );
    assert!(
        stdout.contains(version),
        "Output should contain version {}",
        version
    );
    assert_eq!(
        stdout.trim(),
        format!("hive {}", version),
        "Output should be in format 'hive X.Y.Z'"
    );
}

#[test]
fn test_version_short_flag() {
    let binary = get_binary_path();
    let output = Command::new(&binary)
        .arg("-v")
        .output()
        .expect("Failed to execute binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Command should exit successfully. Stderr: {}",
        stderr
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = env!("CARGO_PKG_VERSION");

    // Should output "hive X.Y.Z"
    assert!(
        stdout.contains("hive"),
        "Output should contain 'hive'"
    );
    assert!(
        stdout.contains(version),
        "Output should contain version {}",
        version
    );
    assert_eq!(
        stdout.trim(),
        format!("hive {}", version),
        "Output should be in format 'hive X.Y.Z'"
    );
}

#[test]
fn test_version_subcommand() {
    let binary = get_binary_path();
    let output = Command::new(&binary)
        .arg("version")
        .output()
        .expect("Failed to execute binary");

    assert!(
        output.status.success(),
        "Command should exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = env!("CARGO_PKG_VERSION");

    // version subcommand has a different output format with emoji
    assert!(
        stdout.contains("Hive"),
        "Output should contain 'Hive'"
    );
    assert!(
        stdout.contains(version),
        "Output should contain version {}",
        version
    );
}
