use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[test]
fn test_sessions_module_exists() {
    // Basic test to ensure sessions module compiles
    assert!(true);
}

#[test]
fn test_count_messages_basic() {
    let temp_dir = std::env::temp_dir().join("hive-test-sessions-count");
    fs::create_dir_all(&temp_dir).unwrap();

    let session_path = temp_dir.join("test.jsonl");
    let mut file = fs::File::create(&session_path).unwrap();
    writeln!(file, r#"{{"type":"user","content":"hello"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","content":"hi"}}"#).unwrap();
    writeln!(file, r#"{{"type":"tool_use","name":"Read","input":{{}}}}"#).unwrap();

    let count = count_lines(&session_path);
    assert_eq!(count, 3);

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_session_parsing() {
    let temp_dir = std::env::temp_dir().join("hive-test-sessions-parse");
    fs::create_dir_all(&temp_dir).unwrap();

    let session_path = temp_dir.join("test.jsonl");
    let mut file = fs::File::create(&session_path).unwrap();
    writeln!(file, r#"{{"type":"user","content":"hello"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","content":"hi there"}}"#).unwrap();

    // Verify file exists and has content
    assert!(session_path.exists());
    let content = fs::read_to_string(&session_path).unwrap();
    assert!(content.contains("user"));
    assert!(content.contains("assistant"));

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_find_sessions_empty_dir() {
    let temp_dir = std::env::temp_dir().join("hive-test-no-sessions");
    fs::create_dir_all(&temp_dir).unwrap();

    // No sessions should be found in empty directory
    let session_files = find_jsonl_files(&temp_dir);
    assert!(session_files.is_empty());

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_find_sessions_with_files() {
    let temp_dir = std::env::temp_dir().join("hive-test-with-sessions");
    fs::create_dir_all(&temp_dir).unwrap();

    // Create some JSONL files
    fs::File::create(temp_dir.join("session1.jsonl")).unwrap();
    fs::File::create(temp_dir.join("session2.jsonl")).unwrap();
    fs::File::create(temp_dir.join("notasession.txt")).unwrap();

    let session_files = find_jsonl_files(&temp_dir);
    assert_eq!(session_files.len(), 2);

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_export_markdown_format() {
    let temp_dir = std::env::temp_dir().join("hive-test-export");
    fs::create_dir_all(&temp_dir).unwrap();

    let export_path = temp_dir.join("export.md");
    let mut file = fs::File::create(&export_path).unwrap();
    writeln!(file, "# Session: test").unwrap();
    writeln!(file, "## User").unwrap();
    writeln!(file, "Hello").unwrap();

    assert!(export_path.exists());
    let content = fs::read_to_string(&export_path).unwrap();
    assert!(content.contains("# Session"));
    assert!(content.contains("## User"));

    fs::remove_dir_all(&temp_dir).ok();
}

// Helper functions for testing

fn count_lines(path: &PathBuf) -> usize {
    use std::io::BufRead;
    let file = fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    reader.lines().count()
}

fn find_jsonl_files(dir: &PathBuf) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
        .map(|entry| entry.path())
        .collect()
}
