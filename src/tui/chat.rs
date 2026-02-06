use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::markdown::render_markdown;
use super::messages::{ChatMessage, MessageRole, ToolStatus};

pub struct ChatPanel {
    /// Scroll offset from bottom (0 = at bottom, showing newest)
    scroll_offset: u16,
    /// Whether auto-scroll is active (follows new messages)
    auto_scroll: bool,
    /// Total content height from last render
    last_content_height: u16,
    /// Visible area height from last render
    last_visible_height: u16,
}

impl Default for ChatPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatPanel {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            auto_scroll: true,
            last_content_height: 0,
            last_visible_height: 0,
        }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.auto_scroll = false;
        let max_scroll = self
            .last_content_height
            .saturating_sub(self.last_visible_height);
        self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        if amount >= self.scroll_offset {
            self.scroll_offset = 0;
            self.auto_scroll = true;
        } else {
            self.scroll_offset -= amount;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self
            .last_content_height
            .saturating_sub(self.last_visible_height);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn page_up(&mut self) {
        self.scroll_up(self.last_visible_height.saturating_sub(2));
    }

    pub fn page_down(&mut self) {
        self.scroll_down(self.last_visible_height.saturating_sub(2));
    }

    pub fn render(
        &mut self,
        f: &mut Frame,
        area: Rect,
        messages: &[ChatMessage],
        is_focused: bool,
    ) {
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let inner_height = area.height.saturating_sub(2); // account for borders

        // Build all lines from messages
        let mut all_lines: Vec<Line<'static>> = Vec::new();

        for msg in messages {
            match msg {
                ChatMessage::Text {
                    role,
                    content,
                    timestamp,
                    ..
                } => {
                    let (prefix, color) = match role {
                        MessageRole::User => ("You", Color::Cyan),
                        MessageRole::Assistant => ("Claude", Color::Green),
                        MessageRole::System => ("System", Color::DarkGray),
                    };

                    let time_str = timestamp.format("%H:%M").to_string();

                    // Header line with role + timestamp
                    all_lines.push(Line::from(vec![
                        Span::styled(
                            format!("{} ", prefix),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                    ]));

                    // Render content - use markdown for assistant, plain for others
                    match role {
                        MessageRole::Assistant => {
                            let md_lines = render_markdown(content);
                            all_lines.extend(md_lines);
                        }
                        _ => {
                            for line in content.lines() {
                                all_lines.push(Line::from(line.to_string()));
                            }
                        }
                    }

                    // Blank separator
                    all_lines.push(Line::from(""));
                }
                ChatMessage::ToolUse {
                    tool_name,
                    args_summary,
                    status,
                    ..
                } => {
                    let (icon, color) = match status {
                        ToolStatus::Running => ("\u{2699}", Color::Yellow),
                        ToolStatus::Success => ("\u{2713}", Color::Green),
                        ToolStatus::Error => ("\u{2717}", Color::Red),
                    };
                    all_lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                        Span::styled(
                            tool_name.clone(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" {}", args_summary),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
                ChatMessage::ToolResult {
                    tool_name,
                    success,
                    output_preview,
                    ..
                } => {
                    let (icon, color) = if *success {
                        ("\u{2713}", Color::Green)
                    } else {
                        ("\u{2717}", Color::Red)
                    };
                    all_lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                        Span::styled(
                            format!("{}: ", tool_name),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw(output_preview.clone()),
                    ]));
                    all_lines.push(Line::from(""));
                }
                ChatMessage::Error { message, .. } => {
                    all_lines.push(Line::from(Span::styled(
                        format!("\u{26a0} Error: {}", message),
                        Style::default().fg(Color::Red),
                    )));
                    all_lines.push(Line::from(""));
                }
            }
        }

        let content_height = all_lines.len() as u16;
        self.last_content_height = content_height;
        self.last_visible_height = inner_height;

        // Calculate scroll position
        let scroll = if self.auto_scroll {
            content_height.saturating_sub(inner_height)
        } else {
            content_height
                .saturating_sub(inner_height)
                .saturating_sub(self.scroll_offset)
        };

        let chat = Paragraph::new(all_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(" Chat "),
            )
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        f.render_widget(chat, area);

        // Scroll indicator
        if content_height > inner_height && !self.auto_scroll {
            let indicator = format!(" \u{2191}{} ", self.scroll_offset);
            let indicator_area = Rect {
                x: area.x + area.width - indicator.len() as u16 - 1,
                y: area.y,
                width: indicator.len() as u16,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(Span::styled(indicator, Style::default().fg(Color::Yellow))),
                indicator_area,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_panel_new() {
        let panel = ChatPanel::new();
        assert!(panel.auto_scroll);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_up_disables_auto_scroll() {
        let mut panel = ChatPanel::new();
        panel.last_content_height = 100;
        panel.last_visible_height = 20;
        panel.scroll_up(5);
        assert!(!panel.auto_scroll);
        assert_eq!(panel.scroll_offset, 5);
    }

    #[test]
    fn test_scroll_down_to_bottom_enables_auto_scroll() {
        let mut panel = ChatPanel::new();
        panel.auto_scroll = false;
        panel.scroll_offset = 3;
        panel.scroll_down(5); // More than offset
        assert!(panel.auto_scroll);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_to_top() {
        let mut panel = ChatPanel::new();
        panel.last_content_height = 100;
        panel.last_visible_height = 20;
        panel.scroll_to_top();
        assert_eq!(panel.scroll_offset, 80);
        assert!(!panel.auto_scroll);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut panel = ChatPanel::new();
        panel.auto_scroll = false;
        panel.scroll_offset = 50;
        panel.scroll_to_bottom();
        assert_eq!(panel.scroll_offset, 0);
        assert!(panel.auto_scroll);
    }
}
