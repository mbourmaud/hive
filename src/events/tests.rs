use super::*;
use std::io::Write;

#[test]
fn test_parse_task_create_event() {
    let json = r#"{"event":"TaskCreate","ts":"2025-01-15T10:00:00Z","subject":"Implement auth","description":"Add JWT auth"}"#;
    let event: HiveEvent = serde_json::from_str(json).unwrap();
    match event {
        HiveEvent::TaskCreate {
            ts,
            subject,
            description,
        } => {
            assert_eq!(ts, "2025-01-15T10:00:00Z");
            assert_eq!(subject, "Implement auth");
            assert_eq!(description, "Add JWT auth");
        }
        _ => panic!("Expected TaskCreate"),
    }
}

#[test]
fn test_parse_task_update_event() {
    let json = r#"{"event":"TaskUpdate","ts":"2025-01-15T10:01:00Z","task_id":"1","status":"in_progress","owner":"researcher"}"#;
    let event: HiveEvent = serde_json::from_str(json).unwrap();
    match event {
        HiveEvent::TaskUpdate {
            ts,
            task_id,
            status,
            owner,
        } => {
            assert_eq!(ts, "2025-01-15T10:01:00Z");
            assert_eq!(task_id, "1");
            assert_eq!(status, "in_progress");
            assert_eq!(owner, Some("researcher".to_string()));
        }
        _ => panic!("Expected TaskUpdate"),
    }
}

#[test]
fn test_parse_stop_event() {
    let json = r#"{"event":"Stop","ts":"2025-01-15T10:05:00Z"}"#;
    let event: HiveEvent = serde_json::from_str(json).unwrap();
    match event {
        HiveEvent::Stop { ts } => assert_eq!(ts, "2025-01-15T10:05:00Z"),
        _ => panic!("Expected Stop"),
    }
}

#[test]
fn test_event_reader_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let events_path = dir.path().join("events.ndjson");

    // Write some events
    let mut file = fs::File::create(&events_path).unwrap();
    writeln!(
        file,
        r#"{{"event":"Start","ts":"2025-01-15T10:00:00Z","model":"opus"}}"#
    )
    .unwrap();
    writeln!(file, r#"{{"event":"TaskCreate","ts":"2025-01-15T10:01:00Z","subject":"Story 1","description":"desc"}}"#).unwrap();

    let mut reader = EventReader {
        offset: 0,
        path: events_path.clone(),
    };

    let events = reader.read_new();
    assert_eq!(events.len(), 2);

    // No new events
    let events = reader.read_new();
    assert!(events.is_empty());

    // Append more
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&events_path)
        .unwrap();
    writeln!(file, r#"{{"event":"Stop","ts":"2025-01-15T10:05:00Z"}}"#).unwrap();

    let events = reader.read_new();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], HiveEvent::Stop { .. }));
}

#[test]
fn test_event_reader_nonexistent() {
    let mut reader = EventReader {
        offset: 0,
        path: PathBuf::from("/nonexistent/events.ndjson"),
    };
    assert!(!reader.exists());
    assert!(reader.read_new().is_empty());
}

#[test]
fn test_event_reader_skips_invalid_lines() {
    let dir = tempfile::tempdir().unwrap();
    let events_path = dir.path().join("events.ndjson");

    let mut file = fs::File::create(&events_path).unwrap();
    writeln!(
        file,
        r#"{{"event":"Start","ts":"2025-01-15T10:00:00Z","model":"opus"}}"#
    )
    .unwrap();
    writeln!(file, "not valid json").unwrap();
    writeln!(file, r#"{{"event":"Stop","ts":"2025-01-15T10:05:00Z"}}"#).unwrap();

    let mut reader = EventReader {
        offset: 0,
        path: events_path,
    };

    let events = reader.read_new();
    assert_eq!(events.len(), 2); // skipped the invalid line
}

#[test]
fn test_event_timestamp() {
    let event = HiveEvent::Start {
        ts: "2025-01-15T10:00:00Z".to_string(),
        model: "opus".to_string(),
    };
    assert_eq!(event.timestamp(), "2025-01-15T10:00:00Z");
}

#[test]
fn test_parse_todo_snapshot() {
    let json = r#"{"event":"TodoSnapshot","ts":"2025-01-15T10:00:00Z","todos":[{"content":"Build auth","status":"completed","activeForm":"Building auth"},{"content":"Write tests","status":"in_progress","activeForm":"Writing tests"},{"content":"Deploy","status":"pending"}]}"#;
    let event: HiveEvent = serde_json::from_str(json).unwrap();
    match event {
        HiveEvent::TodoSnapshot { ts, todos } => {
            assert_eq!(ts, "2025-01-15T10:00:00Z");
            assert_eq!(todos.len(), 3);
            assert_eq!(todos[0].content, "Build auth");
            assert_eq!(todos[0].status, "completed");
            assert_eq!(todos[1].status, "in_progress");
            assert_eq!(todos[2].status, "pending");
            assert_eq!(todos[0].active_form, Some("Building auth".to_string()));
        }
        _ => panic!("Expected TodoSnapshot"),
    }
}
