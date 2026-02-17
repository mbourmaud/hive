//! Generic compressors: build output, line deduplication, and large output truncation.

/// Maximum lines to keep after generic compression.
const MAX_OUTPUT_LINES: usize = 100;

/// Try to compress output using generic heuristics. Returns None if not applicable.
pub fn try_compress(content: &str) -> Option<String> {
    if let Some(compressed) = try_compress_build_output(content) {
        return Some(compressed);
    }

    let line_count = content.lines().count();
    if line_count > 200 {
        return Some(compress_large_output(content));
    }

    None
}

/// Compress cargo build / cargo check / npm build output.
/// Keeps errors and warnings, collapses "Compiling" / "Downloading" progress lines.
fn try_compress_build_output(content: &str) -> Option<String> {
    let compiling_count = content
        .lines()
        .filter(|l| l.trim_start().starts_with("Compiling"))
        .count();
    let downloading_count = content
        .lines()
        .filter(|l| l.trim_start().starts_with("Downloading"))
        .count();

    // Only compress if there are many progress lines
    if compiling_count + downloading_count < 5 {
        return None;
    }

    let mut result = String::new();
    let mut error_lines: Vec<&str> = Vec::new();
    let mut warning_lines: Vec<&str> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("Compiling")
            || trimmed.starts_with("Downloading")
            || trimmed.starts_with("Fetching")
        {
            continue;
        }
        if trimmed.starts_with("error") || trimmed.starts_with("Error") {
            error_lines.push(line);
        } else if trimmed.starts_with("warning") || trimmed.starts_with("Warning") {
            warning_lines.push(line);
        } else if !trimmed.is_empty() {
            // Keep other meaningful lines (e.g., "Finished", linker errors, etc.)
            result.push_str(line);
            result.push('\n');
        }
    }

    let mut summary = String::new();
    if compiling_count > 0 {
        summary.push_str(&format!("Compiled {compiling_count} crates"));
    }
    if downloading_count > 0 {
        if !summary.is_empty() {
            summary.push_str(", ");
        }
        summary.push_str(&format!("downloaded {downloading_count} crates"));
    }
    summary.push('\n');

    if !error_lines.is_empty() {
        summary.push_str(&format!("\n{} error(s):\n", error_lines.len()));
        for line in &error_lines {
            summary.push_str(line);
            summary.push('\n');
        }
    }
    if !warning_lines.is_empty() {
        summary.push_str(&format!("\n{} warning(s):\n", warning_lines.len()));
        for line in &warning_lines {
            summary.push_str(line);
            summary.push('\n');
        }
    }

    summary.push_str(&result);
    Some(summary.trim().to_string())
}

/// Compress large output by deduplicating consecutive identical lines and truncating.
fn compress_large_output(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut prev_line: Option<&str> = None;
    let mut repeat_count: usize = 0;

    for line in &lines {
        if Some(*line) == prev_line {
            repeat_count += 1;
        } else {
            flush_repeats(&mut result, prev_line, repeat_count);
            result.push(line.to_string());
            prev_line = Some(line);
            repeat_count = 0;
        }

        if result.len() >= MAX_OUTPUT_LINES {
            break;
        }
    }
    flush_repeats(&mut result, prev_line, repeat_count);

    let total_lines = lines.len();
    if total_lines > MAX_OUTPUT_LINES {
        result.push(format!(
            "\n... ({} more lines omitted, {} total)",
            total_lines - result.len(),
            total_lines
        ));
    }

    result.join("\n")
}

fn flush_repeats(result: &mut [String], prev_line: Option<&str>, repeat_count: usize) {
    if repeat_count > 0 {
        if let Some(line) = prev_line {
            // Remove the last added line (the first occurrence) and replace with dedup
            if let Some(last) = result.last_mut() {
                *last = format!("{line} [x{}]", repeat_count + 1);
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_compress_build_output() {
        let mut input = String::new();
        for i in 0..20 {
            input.push_str(&format!("   Compiling crate-{i} v0.1.0\n"));
        }
        input.push_str("    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.2s\n");

        let compressed = try_compress(&input).unwrap();
        assert!(compressed.contains("Compiled 20 crates"));
        assert!(compressed.contains("Finished"));
        assert!(!compressed.contains("Compiling crate-"));
    }

    #[test]
    fn test_compress_build_with_errors() {
        let mut input = String::new();
        for i in 0..10 {
            input.push_str(&format!("   Compiling crate-{i} v0.1.0\n"));
        }
        input.push_str("error[E0308]: mismatched types\n");
        input.push_str("warning: unused variable `x`\n");

        let compressed = try_compress(&input).unwrap();
        assert!(compressed.contains("Compiled 10 crates"));
        assert!(compressed.contains("1 error(s)"));
        assert!(compressed.contains("error[E0308]"));
        assert!(compressed.contains("1 warning(s)"));
    }

    #[test]
    fn test_compress_generic_dedup() {
        let mut input = String::new();
        for _ in 0..250 {
            input.push_str("Processing item...\n");
        }
        let compressed = try_compress(&input).unwrap();
        assert!(compressed.contains("[x"));
        assert!(compressed.contains("more lines omitted"));
        assert!(compressed.len() < input.len() / 2);
    }

    #[test]
    fn test_no_compress_small_output() {
        let input = "line 1\nline 2\nline 3\n";
        assert!(try_compress(input).is_none());
    }
}
