//! Shared output truncation utility for tool results.

/// Truncate output to a maximum byte size, cutting at the last newline boundary.
pub fn truncate_output(output: &str, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        return output.to_string();
    }

    let truncated = &output[..max_bytes];
    // Find the last newline to avoid cutting mid-line
    let end = truncated.rfind('\n').unwrap_or(max_bytes);
    let remaining = output.len() - end;
    format!(
        "{}\n\n... (truncated, {remaining} bytes omitted)",
        &output[..end]
    )
}
