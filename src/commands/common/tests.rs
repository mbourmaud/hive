use super::*;

#[test]
fn test_format_duration_seconds() {
    let duration = chrono::Duration::seconds(45);
    assert_eq!(format_duration(duration), "45s");
}

#[test]
fn test_format_duration_minutes() {
    let duration = chrono::Duration::seconds(125);
    assert_eq!(format_duration(duration), "2m 5s");
}

#[test]
fn test_format_duration_hours() {
    let duration = chrono::Duration::seconds(3725);
    assert_eq!(format_duration(duration), "1h 2m");
}

#[test]
fn test_truncate_with_ellipsis_short() {
    assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
}

#[test]
fn test_truncate_with_ellipsis_long() {
    assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
}

#[test]
fn test_wrap_text_single_line() {
    let result = wrap_text("hello world", 20);
    assert_eq!(result, vec!["hello world"]);
}

#[test]
fn test_wrap_text_multiple_lines() {
    let result = wrap_text("hello world foo bar", 10);
    assert_eq!(result, vec!["hello", "world foo", "bar"]);
}

#[test]
fn test_wrap_text_empty() {
    let result = wrap_text("", 10);
    assert_eq!(result, vec![""]);
}

#[test]
fn test_parse_timestamp_with_timezone() {
    // Test with actual timestamp format from status.json
    let timestamp = "2026-02-09T09:43:19.919277+00:00";
    let result = parse_timestamp(timestamp);
    assert!(
        result.is_some(),
        "Should parse RFC3339 timestamp with timezone"
    );
}

#[test]
fn test_elapsed_since_returns_value() {
    // Test with a timestamp 5 minutes ago
    let five_mins_ago = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
    let result = elapsed_since(&five_mins_ago);
    assert!(result.is_some(), "Should calculate elapsed time");
    let elapsed = result.unwrap();
    // Should show something like "5m 0s" or similar
    assert!(
        elapsed.contains("m") || elapsed.contains("s"),
        "Should format as time string"
    );
}

#[test]
fn test_elapsed_since_with_malformed_timestamp() {
    let result = elapsed_since("not-a-timestamp");
    assert!(result.is_none(), "Should return None for invalid timestamp");
}
