use std::path::PathBuf;
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};

/// Format a raw NDJSON activity.log line into a human-readable summary.
/// Returns None for lines that should be skipped (tool results, unparseable).
pub fn format_log_line(raw: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let typ = v.get("type")?.as_str()?;

    match typ {
        "system" => Some("[init] Session started".to_string()),
        "result" => {
            let subtype = v
                .get("subtype")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");
            let result_text = v
                .get("result")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();
            if result_text.is_empty() {
                Some(format!("[done] {subtype}"))
            } else {
                Some(format!("[done] {subtype} — {result_text}"))
            }
        }
        "user" => {
            let content = v.get("message").and_then(|m| m.get("content"));
            if let Some(arr) = content.and_then(|c| c.as_array()) {
                if arr
                    .iter()
                    .any(|item| item.get("type").and_then(|t| t.as_str()) == Some("tool_result"))
                {
                    return None;
                }
            }
            let text = content
                .and_then(|c| {
                    if let Some(s) = c.as_str() {
                        Some(s.to_string())
                    } else if let Some(arr) = c.as_array() {
                        arr.iter()
                            .filter_map(|item| {
                                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                    item.get("text").and_then(|t| t.as_str()).map(String::from)
                                } else {
                                    None
                                }
                            })
                            .next()
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let truncated: String = text.chars().take(200).collect();
            if truncated.is_empty() {
                None
            } else {
                Some(format!("[user] {truncated}"))
            }
        }
        "assistant" => {
            let content = v
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())?;

            let mut parts: Vec<String> = Vec::new();
            for item in content {
                let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match item_type {
                    "tool_use" => {
                        let name = item
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        let input = item.get("input");
                        let detail = input
                            .and_then(|i| {
                                i.get("file_path")
                                    .or_else(|| i.get("command"))
                                    .or_else(|| i.get("pattern"))
                                    .and_then(|v| v.as_str())
                            })
                            .map(|s| s.chars().take(80).collect::<String>());
                        if let Some(d) = detail {
                            parts.push(format!("[tool] {name} {d}"));
                        } else {
                            parts.push(format!("[tool] {name}"));
                        }
                    }
                    "text" => {
                        let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        if !text.trim().is_empty() {
                            let truncated: String = text.chars().take(200).collect();
                            parts.push(format!("[assistant] {truncated}"));
                        }
                    }
                    _ => {}
                }
            }

            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
            }
        }
        _ => None,
    }
}

pub fn stream_log_file(
    log_path: PathBuf,
    raw: bool,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let stream = async_stream::stream! {
        let mut offset: u64 = 0;

        if let Ok(contents) = tokio::fs::read_to_string(&log_path).await {
            let lines: Vec<&str> = contents.lines().collect();
            let start = if lines.len() > 50 { lines.len() - 50 } else { 0 };
            for line in &lines[start..] {
                if raw {
                    yield Ok(Event::default().data(line));
                } else if let Some(formatted) = format_log_line(line) {
                    for sub in formatted.lines() {
                        yield Ok(Event::default().data(sub));
                    }
                }
            }
            offset = contents.len() as u64;
        }

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Ok(contents) = tokio::fs::read_to_string(&log_path).await {
                let len = contents.len() as u64;
                if len > offset {
                    let new_data = &contents[offset as usize..];
                    for line in new_data.lines() {
                        if !line.is_empty() {
                            if raw {
                                yield Ok(Event::default().data(line));
                            } else if let Some(formatted) = format_log_line(line) {
                                for sub in formatted.lines() {
                                    yield Ok(Event::default().data(sub));
                                }
                            }
                        }
                    }
                    offset = len;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
