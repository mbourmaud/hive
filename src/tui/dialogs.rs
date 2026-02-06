use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::theme::Theme;

#[derive(Debug, Clone)]
pub struct DialogOption {
    pub label: String,
    pub key: char,
    pub style: Style,
}

#[derive(Debug, Clone)]
pub struct ModalDialog {
    pub title: String,
    pub body: String,
    pub options: Vec<DialogOption>,
}

impl ModalDialog {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            options: Vec::new(),
        }
    }

    pub fn with_option(mut self, label: impl Into<String>, key: char, style: Style) -> Self {
        self.options.push(DialogOption {
            label: label.into(),
            key,
            style,
        });
        self
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        let area = centered_rect(60, 40, frame.area());

        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style(true))
            .title(format!(" {} ", self.title))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();
        for body_line in self.body.lines() {
            lines.push(Line::from(body_line.to_string()));
        }

        if !self.options.is_empty() {
            lines.push(Line::from(""));
            let option_spans: Vec<Span> = self
                .options
                .iter()
                .flat_map(|opt| {
                    vec![
                        Span::styled(
                            format!("[{}]", opt.key),
                            opt.style.add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!(" {} ", opt.label)),
                        Span::raw("  "),
                    ]
                })
                .collect();
            lines.push(Line::from(option_spans));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
    }

    pub fn handle_key(&self, c: char) -> Option<char> {
        self.options.iter().find(|o| o.key == c).map(|o| o.key)
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = area.width * percent_x / 100;
    let height = area.height * percent_y / 100;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
