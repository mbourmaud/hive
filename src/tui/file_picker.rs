use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};
use ratatui::Frame;

use super::theme::Theme;

pub struct FilePicker {
    pub visible: bool,
    pub query: String,
    pub selected: usize,
    files: Vec<String>,
    filtered: Vec<(usize, i64)>,
}

impl FilePicker {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            selected: 0,
            files: Vec::new(),
            filtered: Vec::new(),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
        self.load_files();
        self.update_filter();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.query.clear();
    }

    fn load_files(&mut self) {
        self.files.clear();
        let _ = self.walk_dir(".", 0);
    }

    fn walk_dir(&mut self, path: &str, depth: usize) -> std::io::Result<()> {
        if depth > 5 {
            return Ok(());
        }
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden and common ignored dirs
            if file_name.starts_with('.')
                || file_name == "node_modules"
                || file_name == "target"
                || file_name == "dist"
                || file_name == "build"
            {
                continue;
            }

            let path_str = entry.path().to_string_lossy().to_string();
            if entry.file_type()?.is_dir() {
                let _ = self.walk_dir(&path_str, depth + 1);
            } else {
                // Strip leading "./"
                let clean = path_str.strip_prefix("./").unwrap_or(&path_str);
                self.files.push(clean.to_string());
            }
        }
        Ok(())
    }

    pub fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = self
                .files
                .iter()
                .enumerate()
                .map(|(i, _)| (i, 0))
                .take(50)
                .collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(usize, i64)> = self
                .files
                .iter()
                .enumerate()
                .filter_map(|(i, f)| matcher.fuzzy_match(f, &self.query).map(|score| (i, score)))
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            scored.truncate(50);
            self.filtered = scored;
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

    pub fn selected_file(&self) -> Option<&str> {
        self.filtered
            .get(self.selected)
            .and_then(|&(idx, _)| self.files.get(idx))
            .map(|s| s.as_str())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style(true))
            .title(format!(" Files: {} ", self.query))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(display_idx, &(file_idx, _score))| {
                let file = &self.files[file_idx];
                let is_selected = display_idx == self.selected;

                let style = if is_selected {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default().fg(theme.fg)
                };

                ListItem::new(Line::from(Span::styled(file.clone(), style)))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner);
    }
}

impl Default for FilePicker {
    fn default() -> Self {
        Self::new()
    }
}
