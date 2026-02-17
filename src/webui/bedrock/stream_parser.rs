//! Parse the AWS EventStream binary framing from Bedrock's
//! InvokeModelWithResponseStream response.
//!
//! Each frame: [prelude 8B] [headers ...] [payload ...] [message CRC 4B]
//!   prelude = total_length (4B) + headers_length (4B)
//!   prelude CRC (4B) follows the prelude
//!
//! The payload is a JSON object with the same event types as Anthropic SSE.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::broadcast;

use crate::webui::anthropic::client::event_processor::{process_event, EventAccumulator};
use crate::webui::anthropic::types::{Message, UsageStats};

/// Parse the Bedrock EventStream response and return the final message.
pub(super) async fn parse_event_stream(
    response: reqwest::Response,
    tx: &broadcast::Sender<String>,
    abort_flag: &Arc<AtomicBool>,
) -> Result<(Message, UsageStats, String)> {
    let mut acc = EventAccumulator::new();
    let mut raw_buffer: Vec<u8> = Vec::new();

    use futures_util::StreamExt;
    let mut byte_stream = response.bytes_stream();

    loop {
        let chunk = tokio::select! {
            biased;
            _ = abort_notified(abort_flag) => break,
            next = byte_stream.next() => match next {
                Some(c) => c.context("Reading EventStream chunk")?,
                None => break,
            },
        };
        raw_buffer.extend_from_slice(&chunk);

        // Try to parse complete frames from the buffer
        while let Some(consumed) = try_parse_frame(&raw_buffer, tx, &mut acc) {
            raw_buffer.drain(..consumed);
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

/// Try to parse one complete EventStream frame from the buffer.
/// Returns `Some(bytes_consumed)` if a frame was parsed, `None` if incomplete.
fn try_parse_frame(
    buffer: &[u8],
    tx: &broadcast::Sender<String>,
    acc: &mut EventAccumulator,
) -> Option<usize> {
    // Need at least 12 bytes: prelude (8) + prelude CRC (4)
    if buffer.len() < 12 {
        return None;
    }

    let total_length = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
    let headers_length = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]) as usize;

    // Wait for the full frame
    if buffer.len() < total_length {
        return None;
    }

    // Layout: prelude(8) + prelude_crc(4) + headers(headers_length) + payload(...) + msg_crc(4)
    let headers_start = 12;
    let headers_end = headers_start + headers_length;
    let payload_end = total_length - 4; // subtract message CRC

    // Extract event type from headers
    let event_type = parse_event_type_header(&buffer[headers_start..headers_end]);

    // Extract payload
    if payload_end > headers_end {
        let payload = &buffer[headers_end..payload_end];
        if let Ok(payload_str) = std::str::from_utf8(payload) {
            // Bedrock wraps the JSON inside a `bytes` field
            if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(payload_str) {
                if let Some(bytes_val) = wrapper.get("bytes").and_then(|b| b.as_str()) {
                    // Base64-decode the inner payload
                    if let Ok(decoded) = base64_decode(bytes_val) {
                        if let Ok(inner_str) = std::str::from_utf8(&decoded) {
                            let mapped_type = map_bedrock_event_type(&event_type, inner_str);
                            process_event(&mapped_type, inner_str, tx, acc);
                        }
                    }
                } else {
                    // Some events have the JSON directly
                    let mapped_type = map_bedrock_event_type(&event_type, payload_str);
                    process_event(&mapped_type, payload_str, tx, acc);
                }
            }
        }
    }

    Some(total_length)
}

/// Parse the `:event-type` header from the EventStream header block.
fn parse_event_type_header(header_bytes: &[u8]) -> String {
    let mut pos = 0;
    while pos < header_bytes.len() {
        // Header: name_length(1) + name(N) + type(1) + value_length(2) + value(M)
        if pos >= header_bytes.len() {
            break;
        }
        let name_len = header_bytes[pos] as usize;
        pos += 1;
        if pos + name_len > header_bytes.len() {
            break;
        }
        let name = std::str::from_utf8(&header_bytes[pos..pos + name_len]).unwrap_or("");
        pos += name_len;

        // Header value type (7 = string)
        if pos >= header_bytes.len() {
            break;
        }
        let value_type = header_bytes[pos];
        pos += 1;

        if value_type == 7 {
            // String type: 2-byte length + value
            if pos + 2 > header_bytes.len() {
                break;
            }
            let val_len = u16::from_be_bytes([header_bytes[pos], header_bytes[pos + 1]]) as usize;
            pos += 2;
            if pos + val_len > header_bytes.len() {
                break;
            }
            let val = std::str::from_utf8(&header_bytes[pos..pos + val_len]).unwrap_or("");
            pos += val_len;

            if name == ":event-type" {
                return val.to_string();
            }
        } else {
            // Skip other value types — we only care about :event-type
            break;
        }
    }
    String::new()
}

/// Map Bedrock event type names to the Anthropic SSE event type names.
fn map_bedrock_event_type(bedrock_type: &str, payload: &str) -> String {
    // Bedrock uses the same event type names as Anthropic for model responses
    if !bedrock_type.is_empty() {
        return bedrock_type.to_string();
    }
    // Fallback: try to infer type from JSON payload
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(payload) {
        if let Some(t) = val.get("type").and_then(|v| v.as_str()) {
            return t.to_string();
        }
    }
    String::new()
}

fn base64_decode(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(input)
}

/// Poll the abort flag at 50ms intervals.
async fn abort_notified(flag: &AtomicBool) {
    loop {
        if flag.load(Ordering::Relaxed) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
