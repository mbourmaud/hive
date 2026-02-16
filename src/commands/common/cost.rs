use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

// ============================================================================
// Cost Tracking
// ============================================================================

/// How many bytes to read from the tail of the activity log.
/// Cost/usage data is cumulative, so we only need the most recent entries.
const TAIL_READ_BYTES: u64 = 8192;

/// Parsed cost summary from activity log
#[derive(Debug, Clone, Default)]
pub(crate) struct CostSummary {
    pub total_cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
}

/// Parse cost/token info from a drone's activity.log at a specific project root.
/// Falls back to cost.ndjson (native team mode) if activity.log doesn't exist.
pub(crate) fn parse_cost_from_log_at(project_root: &Path, drone_name: &str) -> CostSummary {
    let drone_dir = project_root.join(".hive/drones").join(drone_name);
    let log_path = drone_dir.join("activity.log");
    let summary = parse_cost_from_log_path(&log_path);
    if summary.total_cost_usd > 0.0 || summary.input_tokens > 0 {
        return summary;
    }
    // Fallback: native team cost.ndjson
    parse_cost_from_ndjson(&drone_dir.join("cost.ndjson"))
}

/// Parse cost/token info from a drone's activity.log (stream-json format).
/// Falls back to cost.ndjson (native team mode) if activity.log doesn't exist.
pub(crate) fn parse_cost_from_log(drone_name: &str) -> CostSummary {
    let drone_dir = PathBuf::from(".hive/drones").join(drone_name);
    let log_path = drone_dir.join("activity.log");
    let summary = parse_cost_from_log_path(&log_path);
    if summary.total_cost_usd > 0.0 || summary.input_tokens > 0 {
        return summary;
    }
    // Fallback: native team cost.ndjson
    parse_cost_from_ndjson(&drone_dir.join("cost.ndjson"))
}

fn parse_cost_from_log_path(log_path: &Path) -> CostSummary {
    let mut file = match fs::File::open(log_path) {
        Ok(f) => f,
        Err(_) => return CostSummary::default(),
    };

    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);

    // Read only the tail of the file
    let contents = if file_size > TAIL_READ_BYTES {
        if file.seek(SeekFrom::End(-(TAIL_READ_BYTES as i64))).is_err() {
            return CostSummary::default();
        }
        let mut buf = String::new();
        if file.read_to_string(&mut buf).is_err() {
            return CostSummary::default();
        }
        // Skip the first (partial) line since we seeked into the middle
        if let Some(idx) = buf.find('\n') {
            buf[idx + 1..].to_string()
        } else {
            buf
        }
    } else {
        let mut buf = String::new();
        if file.read_to_string(&mut buf).is_err() {
            return CostSummary::default();
        }
        buf
    };

    let mut summary = CostSummary::default();

    // Scan lines for cumulative cost data (take latest values)
    for line in contents.lines() {
        let parsed: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Look for cost_usd at top level (stream-json result events)
        if let Some(cost) = parsed.get("cost_usd").and_then(|v| v.as_f64()) {
            summary.total_cost_usd = cost; // cumulative — take latest
        }

        // Look for usage stats
        if let Some(usage) = parsed.get("usage") {
            if let Some(input) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                summary.input_tokens = input;
            }
            if let Some(output) = usage.get("output_tokens").and_then(|v| v.as_u64()) {
                summary.output_tokens = output;
            }
            if let Some(cache_read) = usage
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_u64())
            {
                summary.cache_read_tokens = cache_read;
            }
            if let Some(cache_create) = usage
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_u64())
            {
                summary.cache_creation_tokens = cache_create;
            }
        }
    }

    summary
}

/// Pricing constants (per million tokens, Sonnet 4.5 as default).
const INPUT_PRICE_PER_M: f64 = 3.0;
const OUTPUT_PRICE_PER_M: f64 = 15.0;
const CACHE_READ_PRICE_PER_M: f64 = 0.30;
const CACHE_CREATE_PRICE_PER_M: f64 = 3.75;

/// Parse cost from native team cost.ndjson.
/// Each line has incremental usage from one agentic loop call.
/// We take the latest line (most recent cumulative snapshot from worker).
fn parse_cost_from_ndjson(path: &Path) -> CostSummary {
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return CostSummary::default(),
    };

    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;
    let mut total_cache_create: u64 = 0;

    for line in contents.lines() {
        let parsed: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Each line is a snapshot of one worker's session at that point.
        // Sum across all workers (each worker writes its cumulative totals).
        if let Some(v) = parsed.get("input_tokens").and_then(|v| v.as_u64()) {
            total_input += v;
        }
        if let Some(v) = parsed.get("output_tokens").and_then(|v| v.as_u64()) {
            total_output += v;
        }
        if let Some(v) = parsed.get("cache_read").and_then(|v| v.as_u64()) {
            total_cache_read += v;
        }
        if let Some(v) = parsed.get("cache_create").and_then(|v| v.as_u64()) {
            total_cache_create += v;
        }
    }

    let cost = (total_input as f64 * INPUT_PRICE_PER_M
        + total_output as f64 * OUTPUT_PRICE_PER_M
        + total_cache_read as f64 * CACHE_READ_PRICE_PER_M
        + total_cache_create as f64 * CACHE_CREATE_PRICE_PER_M)
        / 1_000_000.0;

    CostSummary {
        total_cost_usd: cost,
        input_tokens: total_input,
        output_tokens: total_output,
        cache_read_tokens: total_cache_read,
        cache_creation_tokens: total_cache_create,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_summary_default() {
        let cs = CostSummary::default();
        assert_eq!(cs.total_cost_usd, 0.0);
        assert_eq!(cs.input_tokens, 0);
        assert_eq!(cs.output_tokens, 0);
    }
}
