use ratatui::style::Color;

/// Agent color palette
const AGENT_COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Blue,
    Color::Green,
    Color::Red,
    Color::LightCyan,
    Color::LightMagenta,
];

pub fn agent_color(index: usize) -> Color {
    AGENT_COLORS[index % AGENT_COLORS.len()]
}

/// A parsed message ready for display
pub struct DisplayMessage {
    pub timestamp: String,
    pub from: String,
    pub to: String,
    pub text: String,
    pub is_lead: bool,
}

/// Parse JSON message content into a clean display string
pub fn parse_message_text(raw: &str) -> String {
    if !raw.starts_with('{') {
        return raw.to_string();
    }
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) else {
        return raw.to_string();
    };
    let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match msg_type {
        "idle_notification" => "[idle]".to_string(),
        "shutdown_request" => {
            let content = parsed
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("shutting down");
            format!("[shutdown] {}", content)
        }
        "shutdown_response" | "shutdown_approved" => {
            let approved = parsed
                .get("approve")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if approved {
                "[shutdown approved]".to_string()
            } else {
                "[shutdown rejected]".to_string()
            }
        }
        _ => parsed
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or(raw)
            .to_string(),
    }
}
