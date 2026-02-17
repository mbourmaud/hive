//! Compressors for test runner output (cargo test, npm test, pytest, jest).

use regex::Regex;
use std::sync::LazyLock;

static CARGO_TEST_SUMMARY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"test result: (\w+)\. (\d+) passed; (\d+) failed").unwrap());

static CARGO_TEST_FAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^---- (.+) stdout ----$").unwrap());

static JS_TEST_SUMMARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Tests:\s+(?:(\d+) failed,\s+)?(\d+) passed,\s+(\d+) total").unwrap()
});

static PYTEST_SUMMARY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:(\d+) failed)?.*?(\d+) passed").unwrap());

/// Try to compress test output. Returns None if the output doesn't match.
pub fn try_compress(content: &str) -> Option<String> {
    if let Some(compressed) = try_compress_cargo_test(content) {
        return Some(compressed);
    }
    if let Some(compressed) = try_compress_js_test(content) {
        return Some(compressed);
    }
    if let Some(compressed) = try_compress_pytest(content) {
        return Some(compressed);
    }
    None
}

fn try_compress_cargo_test(content: &str) -> Option<String> {
    let caps = CARGO_TEST_SUMMARY.captures(content)?;

    let passed: u32 = caps[2].parse().unwrap_or(0);
    let failed: u32 = caps[3].parse().unwrap_or(0);
    let total = passed + failed;

    // Extract timing if available
    let time = extract_time(content);

    if failed == 0 {
        return Some(format!("{total} tests passed{time}"));
    }

    // Keep failure details
    let mut result = String::new();
    let mut in_failure_block = false;
    let mut failure_lines = 0;
    let max_failure_lines: usize = 10;

    for line in content.lines() {
        if let Some(caps) = CARGO_TEST_FAIL.captures(line) {
            in_failure_block = true;
            failure_lines = 0;
            result.push_str(&format!("FAIL {}:\n", &caps[1]));
        } else if in_failure_block {
            if line.starts_with("----") || line.starts_with("failures:") {
                in_failure_block = false;
            } else if failure_lines < max_failure_lines {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    result.push_str("  ");
                    result.push_str(trimmed);
                    result.push('\n');
                    failure_lines += 1;
                }
            }
        }
    }

    result.push_str(&format!(
        "{total} tests: {passed} passed, {failed} failed{time}"
    ));
    Some(result)
}

fn try_compress_js_test(content: &str) -> Option<String> {
    // Detect jest/vitest output
    if !content.contains("Tests:") || !content.contains("passed,") {
        return None;
    }

    let caps = JS_TEST_SUMMARY.captures(content)?;
    let failed: u32 = caps
        .get(1)
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0);
    let passed: u32 = caps[2].parse().unwrap_or(0);
    let total: u32 = caps[3].parse().unwrap_or(0);
    let time = extract_time(content);

    if failed == 0 {
        return Some(format!("{total} tests passed{time}"));
    }

    // Keep FAIL lines
    let mut result = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("FAIL") || trimmed.contains("Error:") || trimmed.contains("expect(")
        {
            result.push_str(trimmed);
            result.push('\n');
        }
    }
    result.push_str(&format!(
        "{total} tests: {passed} passed, {failed} failed{time}"
    ));
    Some(result)
}

fn try_compress_pytest(content: &str) -> Option<String> {
    // Detect pytest output — look for the "= N passed =" summary line
    if !content.contains("passed") || !content.contains("====") {
        return None;
    }

    // Must have pytest-style summary
    let summary_line = content
        .lines()
        .find(|l| l.contains("====") && l.contains("passed"))?;
    let caps = PYTEST_SUMMARY.captures(summary_line)?;

    let failed: u32 = caps
        .get(1)
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0);
    let passed: u32 = caps[2].parse().unwrap_or(0);
    let total = passed + failed;
    let time = extract_time(content);

    if failed == 0 {
        return Some(format!("{total} tests passed{time}"));
    }

    let mut result = String::new();
    let mut in_failure = false;
    for line in content.lines() {
        if line.starts_with("FAILED ") || line.starts_with("E ") {
            result.push_str(line);
            result.push('\n');
            in_failure = true;
        } else if in_failure && line.starts_with("    ") {
            result.push_str(line);
            result.push('\n');
        } else {
            in_failure = false;
        }
    }
    result.push_str(&format!(
        "{total} tests: {passed} passed, {failed} failed{time}"
    ));
    Some(result)
}

fn extract_time(content: &str) -> String {
    // Look for common time patterns: "finished in 0.10s", "Time: 1.234s", "in 0.5s"
    for line in content.lines().rev() {
        if let Some(pos) = line.find("finished in ") {
            let rest = &line[pos + 12..];
            if let Some(end) = rest.find('s') {
                return format!(" ({}s)", &rest[..end]);
            }
        }
        if let Some(pos) = line.find("Time:") {
            let rest = line[pos + 5..].trim();
            if let Some(end) = rest.find('s') {
                return format!(" ({}s)", &rest[..end]);
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_compress_cargo_test_all_pass() {
        let input = "\
running 107 tests
test test_a ... ok
test test_b ... ok
test test_c ... ok

test result: ok. 107 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
";
        let compressed = try_compress(input).unwrap();
        assert_eq!(compressed, "107 tests passed (0.10s)");
    }

    #[test]
    fn test_compress_cargo_test_with_failures() {
        let input = "\
running 10 tests
test test_a ... ok
test test_b ... FAILED
test test_c ... ok

failures:

---- test_b stdout ----
thread 'test_b' panicked at src/lib.rs:42:
assertion `left == right` failed
  left: \"foo\"
 right: \"bar\"

failures:
    test_b

test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
";
        let compressed = try_compress(input).unwrap();
        assert!(compressed.contains("FAIL test_b:"));
        assert!(compressed.contains("assertion"));
        assert!(compressed.contains("10 tests: 9 passed, 1 failed"));
    }

    #[test]
    fn test_compress_jest_all_pass() {
        let input = "\
PASS src/app.test.ts
PASS src/utils.test.ts

Test Suites: 2 passed, 2 total
Tests:       15 passed, 15 total
Snapshots:   0 total
Time:        2.5s
";
        let compressed = try_compress(input).unwrap();
        assert_eq!(compressed, "15 tests passed (2.5s)");
    }

    #[test]
    fn test_no_match_for_non_test_output() {
        let input = "Hello world\nThis is just regular output\n";
        assert!(try_compress(input).is_none());
    }
}
