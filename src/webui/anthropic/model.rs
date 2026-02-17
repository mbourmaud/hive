pub fn resolve_model(short: &str) -> &str {
    match short.to_lowercase().as_str() {
        "sonnet" | "claude-sonnet" | "sonnet-4.5" => "claude-sonnet-4-5-20250929",
        "opus" | "claude-opus" | "opus-4" => "claude-opus-4-20250514",
        "opus-4.6" | "claude-opus-4.6" => "claude-opus-4-6-20260213",
        "haiku" | "claude-haiku" | "haiku-4.5" => "claude-haiku-4-5-20251001",
        other if other.starts_with("claude-") => short,
        _ => "claude-sonnet-4-5-20250929",
    }
}

/// Maximum output tokens allowed by the Anthropic API for a given model.
/// With extended thinking enabled, Sonnet 4.5 supports up to 128K output.
/// Opus models are capped at 32K. Haiku at 16K.
pub fn max_output_tokens(model_id: &str, thinking_enabled: bool) -> u32 {
    if model_id.contains("opus") {
        32_000
    } else if model_id.contains("sonnet") && thinking_enabled {
        128_000
    } else {
        16_384
    }
}
