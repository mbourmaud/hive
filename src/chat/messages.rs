use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::app::{ChatMessage, MessageRole};
use super::theme;

pub struct MessageDisplay {
    /// Scroll offset (number of lines scrolled from top)
    scroll_offset: usize,
    /// Whether auto-scroll is enabled (scroll to bottom on new content)
    auto_scroll: bool,
    /// Whether the assistant is currently streaming
    is_streaming: bool,
    /// Total rendered line count (for scroll calculations)
    total_lines: usize,
}

impl Default for MessageDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageDisplay {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            auto_scroll: true,
            is_streaming: false,
            total_lines: 0,
        }
    }

    /// Render the message list into the given area
    pub fn render(&mut self, f: &mut Frame, area: Rect, messages: &[ChatMessage]) {
        let inner_width = area.width.saturating_sub(2) as usize;

        if messages.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .title(" Chat ")
                .title_style(Style::default().fg(theme::PRIMARY));

            let welcome = Paragraph::new("Welcome to Hive Chat. Type a message to begin.")
                .style(Style::default().fg(theme::DIM))
                .block(block);
            f.render_widget(welcome, area);
            return;
        }

        // Build all rendered lines
        let mut all_lines: Vec<Line<'static>> = Vec::new();

        for msg in messages {
            if msg.role == MessageRole::System {
                // Tool calls: show as dimmed inline
                all_lines.push(Line::from(Span::styled(
                    msg.content.clone(),
                    Style::default().fg(theme::DIM),
                )));
            } else {
                let (prefix, prefix_color) = match msg.role {
                    MessageRole::User => ("You", theme::USER_MSG),
                    MessageRole::Assistant => ("Claude", theme::ASSISTANT_MSG),
                    MessageRole::Error => ("Error", theme::ERROR_COLOR),
                    MessageRole::System => unreachable!(),
                };

                // Role header line
                all_lines.push(Line::from(Span::styled(
                    prefix.to_string(),
                    Style::default()
                        .fg(prefix_color)
                        .add_modifier(Modifier::BOLD),
                )));

                if msg.role == MessageRole::Error {
                    // Error content in red
                    for line_str in msg.content.lines() {
                        all_lines.push(Line::from(Span::styled(
                            line_str.to_string(),
                            Style::default().fg(theme::ERROR_COLOR),
                        )));
                    }
                } else {
                    // Content lines with basic markdown
                    let content_lines = render_markdown(&msg.content, inner_width);
                    all_lines.extend(content_lines);
                }
            }

            // Blank separator line
            all_lines.push(Line::raw(""));
        }

        // Add streaming indicator if active
        if self.is_streaming {
            all_lines.push(Line::from(Span::styled(
                "...".to_string(),
                Style::default().fg(theme::DIM),
            )));
        }

        self.total_lines = all_lines.len();

        // Calculate scroll
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
        let max_scroll = self.total_lines.saturating_sub(visible_height);

        if self.auto_scroll {
            self.scroll_offset = max_scroll;
        } else {
            self.scroll_offset = self.scroll_offset.min(max_scroll);
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .title(" Chat ")
            .title_style(Style::default().fg(theme::PRIMARY));

        let paragraph = Paragraph::new(all_lines)
            .block(block)
            .scroll((self.scroll_offset as u16, 0));

        f.render_widget(paragraph, area);

        // Show scroll indicator if not at bottom
        if self.scroll_offset < max_scroll {
            let remaining = max_scroll - self.scroll_offset;
            let indicator = format!(" {} more lines ", remaining);
            let indicator_len = indicator.len() as u16 + 1;
            let x = area.x + area.width.saturating_sub(indicator_len + 1);
            let y = area.y + area.height - 1;
            let w = indicator_len.min(area.x + area.width - x);
            if w > 0 {
                let indicator_widget = Paragraph::new(Line::from(Span::styled(
                    indicator,
                    Style::default().fg(theme::SECONDARY),
                )));
                f.render_widget(indicator_widget, Rect::new(x, y, w, 1));
            }
        }
    }

    /// Scroll up by given lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.auto_scroll = false;
    }

    /// Scroll down by given lines
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset += lines;
        // auto_scroll will be re-checked in render if we reach bottom
    }

    /// Page up (half page)
    pub fn page_up(&mut self, visible_height: u16) {
        let half = (visible_height / 2) as usize;
        self.scroll_up(half);
    }

    /// Page down (half page)
    pub fn page_down(&mut self, visible_height: u16) {
        let half = (visible_height / 2) as usize;
        self.scroll_down(half);
    }

    /// Scroll to bottom and re-enable auto-scroll
    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
    }

    /// Set streaming state
    pub fn set_streaming(&mut self, streaming: bool) {
        self.is_streaming = streaming;
    }

    /// Check if we're at the bottom
    pub fn is_at_bottom(&self) -> bool {
        self.auto_scroll
    }
}

/// Render markdown text into styled Lines
fn render_markdown(text: &str, _width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line_str in text.lines() {
        if line_str.starts_with("```") {
            in_code_block = !in_code_block;
            lines.push(Line::from(Span::styled(
                format!("  {}", line_str),
                Style::default().fg(theme::DIM),
            )));
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("  {}", line_str),
                Style::default().fg(ratatui::style::Color::Cyan),
            )));
            continue;
        }

        // Headers
        if let Some(header) = line_str.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                header.to_string(),
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(header) = line_str.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                header.to_string(),
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }

        // For inline formatting (bold, code), parse spans
        let spans = parse_inline_markdown(line_str);
        lines.push(Line::from(spans));
    }

    lines
}

/// Parse inline markdown for bold and code spans
fn parse_inline_markdown(text: &str) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Look for the next special marker
        if let Some(pos) = remaining.find("**") {
            // Add text before the marker
            if pos > 0 {
                spans.push(Span::raw(remaining[..pos].to_string()));
            }
            remaining = &remaining[pos + 2..];
            // Find closing **
            if let Some(end) = remaining.find("**") {
                spans.push(Span::styled(
                    remaining[..end].to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                remaining = &remaining[end + 2..];
            } else {
                // No closing **, output the ** as literal
                spans.push(Span::raw("**".to_string()));
            }
        } else if let Some(pos) = remaining.find('`') {
            // Add text before the backtick
            if pos > 0 {
                spans.push(Span::raw(remaining[..pos].to_string()));
            }
            remaining = &remaining[pos + 1..];
            // Find closing backtick
            if let Some(end) = remaining.find('`') {
                spans.push(Span::styled(
                    remaining[..end].to_string(),
                    Style::default().fg(ratatui::style::Color::Cyan),
                ));
                remaining = &remaining[end + 1..];
            } else {
                // No closing backtick, output as literal
                spans.push(Span::raw("`".to_string()));
            }
        } else {
            // No more special markers
            spans.push(Span::raw(remaining.to_string()));
            break;
        }
    }

    if spans.is_empty() {
        spans.push(Span::raw(String::new()));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inline_plain_text() {
        let spans = parse_inline_markdown("hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "hello world");
    }

    #[test]
    fn test_parse_inline_bold() {
        let spans = parse_inline_markdown("hello **bold** world");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "hello ");
        assert_eq!(spans[1].content, "bold");
        assert_eq!(spans[2].content, " world");
    }

    #[test]
    fn test_parse_inline_code() {
        let spans = parse_inline_markdown("use `code` here");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "use ");
        assert_eq!(spans[1].content, "code");
        assert_eq!(spans[2].content, " here");
    }

    #[test]
    fn test_parse_inline_unclosed_bold() {
        let spans = parse_inline_markdown("hello **unclosed");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "hello ");
        assert_eq!(spans[1].content, "**");
        assert_eq!(spans[2].content, "unclosed");
    }

    #[test]
    fn test_parse_inline_empty() {
        let spans = parse_inline_markdown("");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "");
    }

    #[test]
    fn test_render_markdown_code_block() {
        let text = "before\n```rust\nlet x = 1;\n```\nafter";
        let lines = render_markdown(text, 80);
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_render_markdown_headers() {
        let text = "# Title\n## Subtitle\nBody text";
        let lines = render_markdown(text, 80);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_message_display_scroll() {
        let mut display = MessageDisplay::new();
        assert!(display.is_at_bottom());

        display.scroll_up(5);
        assert!(!display.is_at_bottom());

        display.scroll_to_bottom();
        assert!(display.is_at_bottom());
    }
}
