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
