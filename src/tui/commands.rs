use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};
use ratatui::Frame;

use super::theme::Theme;

#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub shortcut: Option<String>,
}

pub struct CommandPalette {
    pub visible: bool,
    pub query: String,
    pub selected: usize,
    pub commands: Vec<SlashCommand>,
    filtered: Vec<usize>,
}

impl CommandPalette {
    pub fn new() -> Self {
        let commands = vec![
            SlashCommand {
                name: "/new".into(),
                description: "Start a new chat session".into(),
                shortcut: Some("Ctrl+N".into()),
            },
            SlashCommand {
                name: "/sessions".into(),
                description: "Browse saved sessions".into(),
                shortcut: Some("Ctrl+L".into()),
            },
            SlashCommand {
                name: "/theme".into(),
                description: "Toggle dark/light theme".into(),
                shortcut: Some("Ctrl+T".into()),
            },
            SlashCommand {
                name: "/clear".into(),
                description: "Clear chat history".into(),
                shortcut: None,
            },
            SlashCommand {
                name: "/stop".into(),
                description: "Stop selected drone".into(),
                shortcut: None,
            },
            SlashCommand {
                name: "/clean".into(),
                description: "Clean selected drone".into(),
                shortcut: None,
            },
            SlashCommand {
                name: "/logs".into(),
                description: "View selected drone logs".into(),
                shortcut: None,
            },
            SlashCommand {
                name: "/refresh".into(),
                description: "Refresh drone status".into(),
                shortcut: None,
            },
            SlashCommand {
                name: "/quit".into(),
                description: "Exit the TUI".into(),
                shortcut: Some("Ctrl+C".into()),
            },
        ];

        let filtered: Vec<usize> = (0..commands.len()).collect();

        Self {
            visible: false,
            query: String::new(),
            selected: 0,
            commands,
            filtered,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
        self.update_filter();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.query.clear();
    }

    pub fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.commands.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(usize, i64)> = self
                .commands
                .iter()
                .enumerate()
                .filter_map(|(i, cmd)| {
                    matcher
                        .fuzzy_match(&cmd.name, &self.query)
                        .map(|score| (i, score))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = 0;
        }
    }

    pub fn type_char(&mut self, c: char) {
        self.query.push(c);
        self.update_filter();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.update_filter();
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self
                .selected
                .checked_sub(1)
                .unwrap_or(self.filtered.len() - 1);
        }
    }

    pub fn selected_command(&self) -> Option<&SlashCommand> {
        self.filtered
            .get(self.selected)
            .and_then(|&i| self.commands.get(i))
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        let area = super::dialogs::centered_rect(50, 50, frame.area());
        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style(true))
            .title(format!(" Commands: {} ", self.query))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(display_idx, &cmd_idx)| {
                let cmd = &self.commands[cmd_idx];
                let is_selected = display_idx == self.selected;

                let name_style = if is_selected {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                };

                let mut spans = vec![
                    Span::styled(format!("{:<12}", cmd.name), name_style),
                    Span::styled(&cmd.description, Style::default().fg(theme.fg)),
                ];

                if let Some(shortcut) = &cmd.shortcut {
                    spans.push(Span::styled(
                        format!("  ({})", shortcut),
                        theme.muted_style(),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner);
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}
