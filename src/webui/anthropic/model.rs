pub fn resolve_model(short: &str) -> &'static str {
    match short.to_lowercase().as_str() {
        "sonnet" | "claude-sonnet" => "claude-sonnet-4-5-20250929",
        "opus" | "claude-opus" => "claude-opus-4-20250514",
        "haiku" | "claude-haiku" => "claude-haiku-4-5-20251001",
        _ => "claude-sonnet-4-5-20250929",
    }
}
