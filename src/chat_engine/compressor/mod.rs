//! Tool output compressor — reduces token consumption via heuristic pattern matching.
//!
//! Applied after tool execution, before results enter conversation history.
//! Never compresses error output. Preserves paths and line numbers.

mod generic;
mod git;
mod tests;

/// Minimum content length to consider compression (chars).
const MIN_COMPRESS_LENGTH: usize = 500;

/// Compress tool output to reduce token consumption.
/// Returns the original content unchanged if no compression applies.
pub fn compress_tool_output(content: &str, is_error: bool) -> String {
    if is_error || content.len() < MIN_COMPRESS_LENGTH {
        return content.to_string();
    }

    // Try pattern-specific compressors in priority order
    if let Some(compressed) = git::try_compress(content) {
        return compressed;
    }
    if let Some(compressed) = tests::try_compress(content) {
        return compressed;
    }
    if let Some(compressed) = generic::try_compress(content) {
        return compressed;
    }

    content.to_string()
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_passthrough_small_output() {
        let small = "hello world";
        assert_eq!(compress_tool_output(small, false), small);
    }

    #[test]
    fn test_passthrough_errors() {
        let error_output = "E".repeat(1000);
        assert_eq!(compress_tool_output(&error_output, true), error_output);
    }

    #[test]
    fn test_passthrough_below_threshold() {
        let content = "x".repeat(499);
        assert_eq!(compress_tool_output(&content, false), content);
    }
}
