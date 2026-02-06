use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

/// Generic centered modal dialog widget
pub struct ModalDialog<'a> {
    title: String,
    content: Vec<Line<'a>>,
    footer: Option<Vec<Line<'a>>>,
    width_percent: u16,
    height_percent: u16,
}

impl<'a> ModalDialog<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: Vec::new(),
            footer: None,
            width_percent: 60,
            height_percent: 50,
        }
    }

    pub fn content(mut self, content: Vec<Line<'a>>) -> Self {
        self.content = content;
        self
    }

    pub fn footer(mut self, footer: Vec<Line<'a>>) -> Self {
        self.footer = Some(footer);
        self
    }

    pub fn width_percent(mut self, percent: u16) -> Self {
        self.width_percent = percent.min(100);
        self
    }

    pub fn height_percent(mut self, percent: u16) -> Self {
        self.height_percent = percent.min(100);
        self
    }

    /// Calculate centered rect
    fn centered_rect(&self, area: Rect) -> Rect {
        let popup_width = (area.width * self.width_percent) / 100;
        let popup_height = (area.height * self.height_percent) / 100;

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((area.height - popup_height) / 2),
                Constraint::Length(popup_height),
                Constraint::Min(0),
            ])
            .split(area);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((area.width - popup_width) / 2),
                Constraint::Length(popup_width),
                Constraint::Min(0),
            ])
            .split(vertical[1])[1]
    }
}

impl<'a> Widget for ModalDialog<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let dialog_area = self.centered_rect(area);

        // Clear the area behind the dialog
        Clear.render(dialog_area, buf);

        // Create the dialog block
        let block = Block::default()
            .title(self.title.clone())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        // Split inner area for content and footer
        let chunks = if self.footer.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(inner)
        };

        // Render content
        let content_paragraph = Paragraph::new(self.content).alignment(Alignment::Left);
        content_paragraph.render(chunks[0], buf);

        // Render footer if present
        if let Some(footer_lines) = self.footer {
            let footer_block = Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray));
            let footer_inner = footer_block.inner(chunks[1]);
            footer_block.render(chunks[1], buf);

            let footer_paragraph = Paragraph::new(footer_lines).alignment(Alignment::Center);
            footer_paragraph.render(footer_inner, buf);
        }
    }
}

/// Confirmation dialog helper
pub fn confirmation_dialog<'a>(
    title: impl Into<String>,
    message: impl Into<String>,
    yes_label: &str,
    no_label: &str,
) -> ModalDialog<'a> {
    let content = vec![
        Line::from(""),
        Line::from(Span::raw(message.into())),
        Line::from(""),
    ];

    let footer = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("[{}] ", yes_label),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("[{}]", no_label),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    ModalDialog::new(title)
        .content(content)
        .footer(footer)
        .width_percent(50)
        .height_percent(30)
}
