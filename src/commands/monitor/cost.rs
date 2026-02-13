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
pub(crate) fn parse_cost_from_log_at(project_root: &Path, drone_name: &str) -> CostSummary {
    let log_path = project_root
        .join(".hive/drones")
        .join(drone_name)
        .join("activity.log");
    parse_cost_from_log_path(&log_path)
}

/// Parse cost/token info from a drone's activity.log (stream-json format).
/// Reads only the last 8KB of the file for efficiency (cost data is cumulative).
pub(crate) fn parse_cost_from_log(drone_name: &str) -> CostSummary {
    let log_path = PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("activity.log");
    parse_cost_from_log_path(&log_path)
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

/// Format a token count as human-readable (e.g., "12.3k", "1.2M").
pub(crate) fn format_token_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_token_count_small() {
        assert_eq!(format_token_count(0), "0");
        assert_eq!(format_token_count(500), "500");
        assert_eq!(format_token_count(999), "999");
    }

    #[test]
    fn test_format_token_count_thousands() {
        assert_eq!(format_token_count(1000), "1.0k");
        assert_eq!(format_token_count(12345), "12.3k");
        assert_eq!(format_token_count(999999), "1000.0k");
    }

    #[test]
    fn test_format_token_count_millions() {
        assert_eq!(format_token_count(1_000_000), "1.0M");
        assert_eq!(format_token_count(2_500_000), "2.5M");
    }

    #[test]
    fn test_cost_summary_default() {
        let cs = CostSummary::default();
        assert_eq!(cs.total_cost_usd, 0.0);
        assert_eq!(cs.input_tokens, 0);
        assert_eq!(cs.output_tokens, 0);
    }
}
