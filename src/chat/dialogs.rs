use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme;

/// Active dialog type
#[derive(Debug, Clone)]
pub enum Dialog {
    /// Session picker
    SessionList {
        sessions: Vec<SessionListItem>,
        selected: usize,
    },
    /// Model picker
    ModelPicker {
        models: Vec<ModelOption>,
        selected: usize,
    },
    /// Help screen
    Help,
}

#[derive(Debug, Clone)]
pub struct SessionListItem {
    pub id: String,
    pub title: String,
    pub model: String,
    pub message_count: usize,
    pub cost: f64,
    pub is_current: bool,
}

#[derive(Debug, Clone)]
pub struct ModelOption {
    pub id: String,
    pub display_name: String,
    pub description: String,
}

pub fn default_models() -> Vec<ModelOption> {
    vec![
        ModelOption {
            id: "claude-sonnet-4-5-20250929".to_string(),
            display_name: "Sonnet".to_string(),
            description: "Fast and capable".to_string(),
        },
        ModelOption {
            id: "claude-opus-4-6".to_string(),
            display_name: "Opus".to_string(),
            description: "Most capable".to_string(),
        },
        ModelOption {
            id: "claude-haiku-4-5-20251001".to_string(),
            display_name: "Haiku".to_string(),
            description: "Fastest and cheapest".to_string(),
        },
    ]
}

/// Result of dialog key handling
pub enum DialogAction {
    /// Dialog consumed the key, keep it open
    Continue,
    /// Close the dialog without action
    Close,
    /// Session selected
    SelectSession(String),
    /// Model selected
    SelectModel(String),
}

impl Dialog {
    /// Handle a key event in the dialog
    pub fn handle_key(&mut self, key: KeyEvent) -> DialogAction {
        if key.code == KeyCode::Esc {
            return DialogAction::Close;
        }

        match self {
            Dialog::SessionList { sessions, selected } => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                    DialogAction::Continue
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *selected + 1 < sessions.len() {
                        *selected += 1;
                    }
                    DialogAction::Continue
                }
                KeyCode::Enter => {
                    if let Some(session) = sessions.get(*selected) {
                        DialogAction::SelectSession(session.id.clone())
                    } else {
                        DialogAction::Close
                    }
                }
                _ => DialogAction::Continue,
            },
            Dialog::ModelPicker { models, selected } => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                    DialogAction::Continue
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *selected + 1 < models.len() {
                        *selected += 1;
                    }
                    DialogAction::Continue
                }
                KeyCode::Enter => {
                    if let Some(model) = models.get(*selected) {
                        DialogAction::SelectModel(model.id.clone())
                    } else {
                        DialogAction::Close
                    }
                }
                _ => DialogAction::Continue,
            },
            Dialog::Help => {
                // Any key closes help
                DialogAction::Close
            }
        }
    }

    /// Render the dialog as an overlay
    pub fn render(&self, f: &mut Frame, area: Rect) {
        // Calculate centered overlay area
        let dialog_width = area.width.clamp(30, 60);
        let dialog_height = match self {
            Dialog::SessionList { sessions, .. } => {
                (sessions.len() as u16 + 4).min(area.height - 4)
            }
            Dialog::ModelPicker { models, .. } => {
                (models.len() as u16 * 2 + 4).min(area.height - 4)
            }
            Dialog::Help => area.height.min(25),
        };

        let x = (area.width.saturating_sub(dialog_width)) / 2;
        let y = (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Clear the area behind the dialog
        f.render_widget(Clear, dialog_area);

        match self {
            Dialog::SessionList { sessions, selected } => {
                self.render_session_list(f, dialog_area, sessions, *selected);
            }
            Dialog::ModelPicker { models, selected } => {
                self.render_model_picker(f, dialog_area, models, *selected);
            }
            Dialog::Help => {
                self.render_help(f, dialog_area);
            }
        }
    }

    fn render_session_list(
        &self,
        f: &mut Frame,
        area: Rect,
        sessions: &[SessionListItem],
        selected: usize,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::PRIMARY))
            .title(" Sessions (\u{2191}\u{2193} Enter Esc) ")
            .title_style(
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();
        for (i, session) in sessions.iter().enumerate() {
            let style = if i == selected {
                Style::default().fg(Color::Black).bg(theme::PRIMARY)
            } else if session.is_current {
                Style::default().fg(theme::SECONDARY)
            } else {
                Style::default().fg(Color::White)
            };

            let marker = if session.is_current {
                "\u{25cf} "
            } else {
                "  "
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "{}{} \u{2014} {} msgs, ${:.2}",
                    marker, session.title, session.message_count, session.cost
                ),
                style,
            )));
        }

        if sessions.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No sessions found.",
                Style::default().fg(theme::DIM),
            )));
        }

        let paragraph = Paragraph::new(lines);
        f.render_widget(paragraph, inner);
    }

    fn render_model_picker(
        &self,
        f: &mut Frame,
        area: Rect,
        models: &[ModelOption],
        selected: usize,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::PRIMARY))
            .title(" Model Picker (\u{2191}\u{2193} Enter Esc) ")
            .title_style(
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();
        for (i, model) in models.iter().enumerate() {
            let style = if i == selected {
                Style::default().fg(Color::Black).bg(theme::PRIMARY)
            } else {
                Style::default().fg(Color::White)
            };

            lines.push(Line::from(Span::styled(
                format!("  {}", model.display_name),
                style.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                format!("    {}", model.description),
                if i == selected {
                    Style::default().fg(Color::Black).bg(theme::PRIMARY)
                } else {
                    Style::default().fg(theme::DIM)
                },
            )));
        }

        let paragraph = Paragraph::new(lines);
        f.render_widget(paragraph, inner);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::PRIMARY))
            .title(" Help (any key to close) ")
            .title_style(
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        f.render_widget(block, area);

        let help_text = vec![
            Line::from(Span::styled(
                "Commands",
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  /new         ", Style::default().fg(theme::SECONDARY)),
                Span::raw("Start a new session"),
            ]),
            Line::from(vec![
                Span::styled("  /sessions    ", Style::default().fg(theme::SECONDARY)),
                Span::raw("List sessions"),
            ]),
            Line::from(vec![
                Span::styled("  /model <m>   ", Style::default().fg(theme::SECONDARY)),
                Span::raw("Switch model"),
            ]),
            Line::from(vec![
                Span::styled("  /status      ", Style::default().fg(theme::SECONDARY)),
                Span::raw("Show drones"),
            ]),
            Line::from(vec![
                Span::styled("  /help        ", Style::default().fg(theme::SECONDARY)),
                Span::raw("This help"),
            ]),
            Line::from(vec![
                Span::styled("  /clear       ", Style::default().fg(theme::SECONDARY)),
                Span::raw("Clear messages"),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                "Keybinds",
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Ctrl+X       ", Style::default().fg(theme::SECONDARY)),
                Span::raw("Leader key"),
            ]),
            Line::from(vec![
                Span::styled("  <leader>n    ", Style::default().fg(theme::SECONDARY)),
                Span::raw("New session"),
            ]),
            Line::from(vec![
                Span::styled("  <leader>b    ", Style::default().fg(theme::SECONDARY)),
                Span::raw("Toggle sidebar"),
            ]),
            Line::from(vec![
                Span::styled("  <leader>m    ", Style::default().fg(theme::SECONDARY)),
                Span::raw("Model picker"),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+C       ", Style::default().fg(theme::SECONDARY)),
                Span::raw("Interrupt / Quit"),
            ]),
        ];

        let paragraph = Paragraph::new(help_text);
        f.render_widget(paragraph, inner);
    }
}
