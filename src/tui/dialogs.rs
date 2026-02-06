use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone)]
pub enum DialogAction {
    Accept,
    Reject,
    AlwaysAllow,
}

#[derive(Debug)]
pub struct PermissionDialog {
    pub tool_name: String,
    pub args_summary: String,
    pub selected: usize, // 0=Accept, 1=Reject, 2=Always
}

impl PermissionDialog {
    pub fn new(tool_name: String, args_summary: String) -> Self {
        Self {
            tool_name,
            args_summary,
            selected: 0,
        }
    }

    pub fn next_option(&mut self) {
        self.selected = (self.selected + 1) % 3;
    }

    pub fn prev_option(&mut self) {
        self.selected = if self.selected == 0 {
            2
        } else {
            self.selected - 1
        };
    }

    pub fn confirm(&self) -> DialogAction {
        match self.selected {
            0 => DialogAction::Accept,
            1 => DialogAction::Reject,
            _ => DialogAction::AlwaysAllow,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        // Center the dialog
        let dialog_area = centered_rect(60, 50, area);

        // Clear the background
        f.render_widget(Clear, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Length(1), // Blank
                Constraint::Length(1), // Tool name
                Constraint::Min(3),    // Args
                Constraint::Length(1), // Blank
                Constraint::Length(1), // Options
            ])
            .split(dialog_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Permission Required ");
        f.render_widget(block, dialog_area);

        // Title
        let title = Paragraph::new(Line::from(vec![Span::styled(
            "Tool Approval Request",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        f.render_widget(title, chunks[0]);

        // Tool name
        let tool_line = Paragraph::new(Line::from(vec![
            Span::styled("Tool: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &self.tool_name,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        f.render_widget(tool_line, chunks[2]);

        // Args summary
        let args = Paragraph::new(self.args_summary.as_str())
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        f.render_widget(args, chunks[3]);

        // Options
        let options = [
            ("y", "Accept", 0),
            ("n", "Reject", 1),
            ("a", "Always Allow", 2),
        ];

        let option_spans: Vec<Span> = options
            .iter()
            .flat_map(|(key, label, idx)| {
                let style = if *idx == self.selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                vec![
                    Span::styled(format!(" [{}] {} ", key, label), style),
                    Span::raw("  "),
                ]
            })
            .collect();

        let options_line = Paragraph::new(Line::from(option_spans));
        f.render_widget(options_line, chunks[5]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_creation() {
        let dialog = PermissionDialog::new("Read".to_string(), "/tmp/file.txt".to_string());
        assert_eq!(dialog.tool_name, "Read");
        assert_eq!(dialog.args_summary, "/tmp/file.txt");
        assert_eq!(dialog.selected, 0);
    }

    #[test]
    fn test_dialog_next_option_cycles() {
        let mut dialog = PermissionDialog::new("Read".to_string(), "args".to_string());
        assert_eq!(dialog.selected, 0);
        dialog.next_option();
        assert_eq!(dialog.selected, 1);
        dialog.next_option();
        assert_eq!(dialog.selected, 2);
        dialog.next_option();
        assert_eq!(dialog.selected, 0);
    }

    #[test]
    fn test_dialog_prev_option_cycles() {
        let mut dialog = PermissionDialog::new("Read".to_string(), "args".to_string());
        assert_eq!(dialog.selected, 0);
        dialog.prev_option();
        assert_eq!(dialog.selected, 2);
        dialog.prev_option();
        assert_eq!(dialog.selected, 1);
        dialog.prev_option();
        assert_eq!(dialog.selected, 0);
    }

    #[test]
    fn test_dialog_confirm_accept() {
        let dialog = PermissionDialog::new("Read".to_string(), "args".to_string());
        assert!(matches!(dialog.confirm(), DialogAction::Accept));
    }

    #[test]
    fn test_dialog_confirm_reject() {
        let mut dialog = PermissionDialog::new("Read".to_string(), "args".to_string());
        dialog.selected = 1;
        assert!(matches!(dialog.confirm(), DialogAction::Reject));
    }

    #[test]
    fn test_dialog_confirm_always_allow() {
        let mut dialog = PermissionDialog::new("Read".to_string(), "args".to_string());
        dialog.selected = 2;
        assert!(matches!(dialog.confirm(), DialogAction::AlwaysAllow));
    }
}
