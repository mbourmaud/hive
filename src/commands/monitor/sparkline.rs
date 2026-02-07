use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ============================================================================
// Activity Sparklines
// ============================================================================

/// Unicode block characters for sparkline rendering (8 levels).
pub(crate) const SPARK_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render a sparkline string from data values.
pub(crate) fn render_sparkline(data: &[u64]) -> String {
    if data.is_empty() {
        return String::new();
    }
    let max = *data.iter().max().unwrap_or(&1).max(&1);
    data.iter()
        .map(|&v| {
            let idx = if max == 0 {
                0
            } else {
                ((v as f64 / max as f64) * 7.0).round() as usize
            };
            SPARK_CHARS[idx.min(7)]
        })
        .collect()
}

/// Parse activity log file size into 8 one-minute buckets for sparkline display.
/// Uses file modification tracking — each call snapshots the file size.
pub(crate) fn update_activity_history(
    history: &mut HashMap<String, Vec<(Instant, u64)>>,
    drone_name: &str,
) {
    let log_path = PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("activity.log");

    let file_size = fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0);
    let now = Instant::now();

    let entry = history.entry(drone_name.to_string()).or_default();
    entry.push((now, file_size));

    // Keep only last 10 minutes of data
    let cutoff = now - Duration::from_secs(600);
    entry.retain(|(t, _)| *t >= cutoff);
}

/// Get sparkline data (8 buckets) from activity history.
pub(crate) fn get_sparkline_data(
    history: &HashMap<String, Vec<(Instant, u64)>>,
    drone_name: &str,
) -> Vec<u64> {
    let entry = match history.get(drone_name) {
        Some(e) if e.len() >= 2 => e,
        _ => return vec![0; 8],
    };

    let now = Instant::now();
    let bucket_duration = Duration::from_secs(60);
    let mut buckets = vec![0u64; 8];

    for i in 0..8 {
        let bucket_end = now - bucket_duration * i as u32;
        let bucket_start = bucket_end - bucket_duration;

        // Find file size delta in this bucket
        let sizes_in_bucket: Vec<u64> = entry
            .iter()
            .filter(|(t, _)| *t >= bucket_start && *t < bucket_end)
            .map(|(_, s)| *s)
            .collect();

        if sizes_in_bucket.len() >= 2 {
            let min = sizes_in_bucket.iter().min().copied().unwrap_or(0);
            let max = sizes_in_bucket.iter().max().copied().unwrap_or(0);
            buckets[7 - i] = max - min;
        }
    }

    buckets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_sparkline_empty() {
        assert_eq!(render_sparkline(&[]), "");
    }

    #[test]
    fn test_render_sparkline_all_zeros() {
        let result = render_sparkline(&[0, 0, 0, 0]);
        assert_eq!(result.chars().count(), 4);
        assert!(result.chars().all(|c| c == '▁'));
    }

    #[test]
    fn test_render_sparkline_ascending() {
        let result = render_sparkline(&[0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(result.chars().count(), 8);
        // First char should be lowest, last should be highest
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[0], '▁');
        assert_eq!(chars[7], '█');
    }

    #[test]
    fn test_render_sparkline_uniform() {
        let result = render_sparkline(&[5, 5, 5, 5]);
        assert_eq!(result.chars().count(), 4);
        // All same value → all same char (max index since 5/5=1.0 → index 7)
        let chars: Vec<char> = result.chars().collect();
        assert!(chars.iter().all(|&c| c == chars[0]));
    }
}
