use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::broadcast;

use super::super::types::{Message, UsageStats};
use super::event_processor::{process_event, EventAccumulator};

/// Parse the SSE byte stream, broadcasting events and returning the final message.
pub(super) async fn parse_sse_stream(
    response: reqwest::Response,
    tx: &broadcast::Sender<String>,
    abort_flag: &Arc<AtomicBool>,
) -> Result<(Message, UsageStats, String)> {
    let mut acc = EventAccumulator::new();
    let mut buffer = String::new();

    use futures_util::StreamExt;
    let mut byte_stream = response.bytes_stream();

    loop {
        let chunk = tokio::select! {
            biased;
            _ = abort_notified(abort_flag) => break,
            next = byte_stream.next() => match next {
                Some(c) => c.context("Reading SSE chunk")?,
                None => break,
            },
        };
        let chunk_str = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_str);

        while let Some(event_end) = buffer.find("\n\n") {
            let event_block = buffer[..event_end].to_string();
            buffer = buffer[event_end + 2..].to_string();

            let mut event_type = String::new();
            let mut event_data = String::new();

            for line in event_block.lines() {
                if let Some(t) = line.strip_prefix("event: ") {
                    event_type = t.to_string();
                } else if let Some(d) = line.strip_prefix("data: ") {
                    event_data = d.to_string();
                }
            }

            if event_data.is_empty() {
                continue;
            }

            process_event(&event_type, &event_data, tx, &mut acc);
        }
    }

    if abort_flag.load(Ordering::Relaxed) {
        let abort_event = serde_json::json!({
            "type": "result",
            "subtype": "error",
            "result": "Aborted by user",
            "is_error": true
        });
        let _ = tx.send(abort_event.to_string());
    }

    Ok(acc.into_result())
}

/// Poll the abort flag at 50ms intervals, returning when it becomes `true`.
async fn abort_notified(flag: &AtomicBool) {
    loop {
        if flag.load(Ordering::Relaxed) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
