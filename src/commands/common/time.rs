//! Time parsing and formatting utilities.

use chrono::{DateTime, Utc};

/// Seconds in an hour
pub const SECONDS_PER_HOUR: i64 = 3600;

/// Seconds in a minute
pub const SECONDS_PER_MINUTE: i64 = 60;

/// Parse an ISO8601/RFC3339 timestamp string into a DateTime.
pub fn parse_timestamp(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Calculate the duration between two timestamp strings.
pub fn duration_between(start: &str, end: &str) -> Option<chrono::Duration> {
    let start_dt = parse_timestamp(start)?;
    let end_dt = parse_timestamp(end)?;
    Some(end_dt.signed_duration_since(start_dt))
}

/// Format a duration as a human-readable string (e.g., "1h 23m" or "5m 30s").
pub fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / SECONDS_PER_HOUR;
    let minutes = (total_seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let seconds = total_seconds % SECONDS_PER_MINUTE;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Calculate and format elapsed time since a timestamp.
pub fn elapsed_since(start: &str) -> Option<String> {
    let start_dt = parse_timestamp(start)?;
    let duration = Utc::now().signed_duration_since(start_dt);
    Some(format_duration(duration))
}
