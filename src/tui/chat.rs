use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

use super::markdown::render_markdown;
use super::messages::Message;

/// Chat panel state managing message history and scrolling
pub struct ChatState {
    messages: Vec<Message>,
    scroll_offset: usize,
    auto_scroll: bool,
    viewport_height: usize,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            viewport_height: 0,
        }
    }

    /// Add a new message to the chat
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Scroll up by N lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll down by N lines
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
        // Re-enable auto-scroll if we reach the bottom
        if self.is_at_bottom() {
            self.auto_scroll = true;
        }
    }

    /// Scroll to the top (Home key)
    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = 0;
    }

    /// Scroll to the bottom (End key)
    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        let total_lines = self.count_total_lines();
        if total_lines > self.viewport_height {
            self.scroll_offset = total_lines - self.viewport_height;
        } else {
            self.scroll_offset = 0;
        }
    }

    /// Page up (PageUp key)
    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_height.saturating_sub(1));
    }

    /// Page down (PageDown key)
    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_height.saturating_sub(1));
    }

    /// Check if scrolled to bottom
    fn is_at_bottom(&self) -> bool {
        let total_lines = self.count_total_lines();
        if total_lines <= self.viewport_height {
            true
        } else {
            self.scroll_offset >= total_lines - self.viewport_height
        }
    }

    /// Count total lines needed to render all messages
    fn count_total_lines(&self) -> usize {
        self.messages
            .iter()
            .map(|msg| self.message_to_lines(msg).len())
            .sum()
    }

    /// Convert a message to rendered lines
    fn message_to_lines(&self, message: &Message) -> Vec<Line<'static>> {
        match message {
            Message::User { content, timestamp } => {
                let mut lines = Vec::new();
                // Header with timestamp
                lines.push(Line::from(vec![
                    Span::styled(
                        "You",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{}", timestamp.format("%H:%M:%S")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                // Content (plain text for user messages)
                for line in content.lines() {
                    lines.push(Line::from(Span::raw(line.to_string())));
                }
                lines.push(Line::from("")); // Spacing
                lines
            }
            Message::Assistant { content, timestamp } => {
                let mut lines = Vec::new();
                // Header
                lines.push(Line::from(vec![
                    Span::styled(
                        "Assistant",
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{}", timestamp.format("%H:%M:%S")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                // Content (markdown rendered)
                lines.extend(render_markdown(content));
                lines.push(Line::from("")); // Spacing
                lines
            }
            Message::ToolUse {
                tool_name,
                args_summary,
                timestamp,
            } => {
                vec![
                    Line::from(vec![
                        Span::styled(
                            "Tool Use",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("{}", timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(tool_name.clone(), Style::default().fg(Color::Yellow)),
                        Span::raw(": "),
                        Span::styled(args_summary.clone(), Style::default().fg(Color::Gray)),
                    ]),
                    Line::from(""),
                ]
            }
            Message::ToolResult {
                success,
                output_summary,
                timestamp,
            } => {
                let status_style = if *success {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };
                let status_text = if *success { "Success" } else { "Failed" };

                vec![
                    Line::from(vec![
                        Span::styled(
                            "Tool Result",
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("{}", timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(status_text, status_style),
                        Span::raw(": "),
                        Span::styled(output_summary.clone(), Style::default().fg(Color::Gray)),
                    ]),
                    Line::from(""),
                ]
            }
            Message::Error { content, timestamp } => {
                vec![
                    Line::from(vec![
                        Span::styled(
                            "Error",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("{}", timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                    Line::from(vec![Span::styled(
                        format!("  {}", content),
                        Style::default().fg(Color::Red),
                    )]),
                    Line::from(""),
                ]
            }
            Message::System { content, timestamp } => {
                vec![
                    Line::from(vec![
                        Span::styled(
                            "System",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("{}", timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                    Line::from(vec![Span::raw(format!("  {}", content))]),
                    Line::from(""),
                ]
            }
        }
    }

    /// Get scrollbar state
    pub fn scrollbar_state(&self) -> ScrollbarState {
        let total_lines = self.count_total_lines();
        ScrollbarState::default()
            .content_length(total_lines)
            .viewport_content_length(self.viewport_height)
            .position(self.scroll_offset)
    }
}

impl Default for ChatState {
    fn default() -> Self {
        Self::new()
    }
}

/// Chat panel widget
pub struct ChatPanel<'a> {
    block: Option<Block<'a>>,
}

impl<'a> ChatPanel<'a> {
    pub fn new() -> Self {
        Self { block: None }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'a> Default for ChatPanel<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> StatefulWidget for ChatPanel<'a> {
    type State = ChatState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Update viewport height
        let inner_area = self.block.as_ref().map_or(area, |b| b.inner(area));
        state.viewport_height = inner_area.height as usize;

        // Collect all lines from all messages
        let mut all_lines: Vec<Line<'static>> = Vec::new();
        for message in &state.messages {
            all_lines.extend(state.message_to_lines(message));
        }

        // Apply scroll offset
        let visible_lines: Vec<Line<'static>> = all_lines
            .into_iter()
            .skip(state.scroll_offset)
            .take(state.viewport_height)
            .collect();

        // Render paragraph
        let paragraph = Paragraph::new(visible_lines).block(
            self.block
                .unwrap_or_else(|| Block::default().borders(Borders::ALL).title("Chat")),
        );

        paragraph.render(area, buf);

        // Render scrollbar if needed
        if state.count_total_lines() > state.viewport_height {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = state.scrollbar_state();
            scrollbar.render(area, buf, &mut scrollbar_state);
        }
    }
}
