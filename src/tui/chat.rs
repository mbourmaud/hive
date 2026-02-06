use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use ratatui::Frame;

use super::markdown::render_markdown;
use super::messages::{ChatMessage, MessageRole};
use super::theme::Theme;

pub struct ChatPanel {
    pub messages: Vec<ChatMessage>,
    pub scroll_offset: u16,
    pub auto_scroll: bool,
    content_height: u16,
}

impl ChatPanel {
    pub fn new() -> Self {
        Self {
            messages: vec![ChatMessage::system(
                "Welcome to Hive TUI. Type a message and press Ctrl+S to send.",
            )],
            scroll_offset: 0,
            auto_scroll: true,
            content_height: 0,
        }
    }

    pub fn add_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn append_to_last_assistant(&mut self, text: &str) {
        if let Some(last) = self.messages.last_mut() {
            if last.role == MessageRole::Assistant {
                last.content.push_str(text);
                if self.auto_scroll {
                    self.scroll_to_bottom();
                }
                return;
            }
        }
        self.add_message(ChatMessage::assistant(text));
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
        if self.scroll_offset >= self.content_height.saturating_sub(1) {
            self.auto_scroll = true;
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.content_height.saturating_sub(1);
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style(focused))
            .title(" Chat ");

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut all_lines: Vec<Line<'static>> = Vec::new();

        for msg in &self.messages {
            // Role header
            let (role_label, role_style) = match msg.role {
                MessageRole::User => (
                    "You",
                    Style::default()
                        .fg(theme.user_msg)
                        .add_modifier(Modifier::BOLD),
                ),
                MessageRole::Assistant => (
                    "Claude",
                    Style::default()
                        .fg(theme.assistant_msg)
                        .add_modifier(Modifier::BOLD),
                ),
                MessageRole::System => (
                    "System",
                    Style::default()
                        .fg(theme.system_msg)
                        .add_modifier(Modifier::BOLD),
                ),
                MessageRole::Error => (
                    "Error",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                ),
            };

            all_lines.push(Line::from(vec![Span::styled(
                format!("[{}]", role_label),
                role_style,
            )]));

            // Content
            match msg.role {
                MessageRole::Assistant => {
                    let rendered = render_markdown(&msg.content);
                    all_lines.extend(rendered.lines);
                }
                _ => {
                    for line in msg.content.lines() {
                        all_lines.push(Line::from(line.to_string()));
                    }
                }
            }

            // Separator
            all_lines.push(Line::from(""));
        }

        self.content_height = all_lines.len() as u16;

        // Clamp scroll
        let visible_height = inner.height;
        if self.auto_scroll {
            self.scroll_offset = self.content_height.saturating_sub(visible_height);
        }
        if self.scroll_offset > self.content_height.saturating_sub(visible_height) {
            self.scroll_offset = self.content_height.saturating_sub(visible_height);
        }

        let paragraph = Paragraph::new(Text::from(all_lines))
            .scroll((self.scroll_offset, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);

        // Scrollbar
        if self.content_height > visible_height {
            let mut scrollbar_state = ScrollbarState::new(self.content_height as usize)
                .position(self.scroll_offset as usize)
                .viewport_content_length(visible_height as usize);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }
}

impl Default for ChatPanel {
    fn default() -> Self {
        Self::new()
    }
}
