use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};
use std::path::Path;

pub struct FilePicker {
    pub visible: bool,
    pub query: String,
    pub selected: usize,
    pub list_state: ListState,
    files: Vec<String>,
    filtered: Vec<(i64, String)>,
    matcher: SkimMatcherV2,
}

impl Default for FilePicker {
    fn default() -> Self {
        Self::new()
    }
}

impl FilePicker {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            selected: 0,
            list_state: ListState::default(),
            files: Vec::new(),
            filtered: Vec::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;

        // Scan project files (limited to avoid slowness)
        self.files = scan_project_files(".", 500);
        self.update_filter();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.query.clear();
        self.filtered.clear();
    }

    pub fn add_char(&mut self, c: char) {
        self.query.push(c);
        self.update_filter();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.update_filter();
    }

    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = self.files.iter().take(20).map(|f| (0, f.clone())).collect();
        } else {
            let mut scored: Vec<(i64, String)> = self
                .files
                .iter()
                .filter_map(|f| {
                    self.matcher
                        .fuzzy_match(f, &self.query)
                        .map(|score| (score, f.clone()))
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            self.filtered = scored.into_iter().take(20).collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
        self.list_state.select(Some(self.selected));
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn confirm(&self) -> Option<String> {
        self.filtered.get(self.selected).map(|(_, f)| f.clone())
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let height = (self.filtered.len() as u16 + 3).min(15);
        let width = 50u16.min(area.width.saturating_sub(2));

        let popup = Rect {
            x: area.x + 1,
            y: area.y.saturating_sub(height + 1),
            width,
            height,
        };

        f.render_widget(Clear, popup);

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .map(|(_, path)| {
                ListItem::new(Line::from(Span::styled(
                    path.clone(),
                    Style::default().fg(Color::White),
                )))
            })
            .collect();

        let title = format!(" @ {} ", self.query);
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(title),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, popup, &mut self.list_state);
    }
}

/// Scan project files, respecting .gitignore patterns via git ls-files
fn scan_project_files(dir: &str, max: usize) -> Vec<String> {
    // Try git ls-files first (respects .gitignore)
    if let Ok(output) = std::process::Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .current_dir(dir)
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.lines().take(max).map(String::from).collect();
        }
    }

    // Fallback: simple walkdir
    let mut files = Vec::new();
    collect_files(Path::new(dir), &mut files, max, 0);
    files
}

fn collect_files(dir: &Path, files: &mut Vec<String>, max: usize, depth: usize) {
    if depth > 5 || files.len() >= max {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if files.len() >= max {
                return;
            }
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Skip hidden dirs and common non-useful dirs
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }

            if path.is_file() {
                if let Ok(rel) = path.strip_prefix(".") {
                    files.push(rel.to_string_lossy().to_string());
                } else {
                    files.push(path.to_string_lossy().to_string());
                }
            } else if path.is_dir() {
                collect_files(&path, files, max, depth + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_picker_new() {
        let picker = FilePicker::new();
        assert!(!picker.visible);
        assert!(picker.query.is_empty());
    }

    #[test]
    fn test_scan_project_files() {
        // Should not panic and return some files
        let files = scan_project_files(".", 10);
        // We're in a project dir so there should be at least some files
        assert!(!files.is_empty());
    }

    #[test]
    fn test_fuzzy_matching() {
        let matcher = SkimMatcherV2::default();
        let score = matcher.fuzzy_match("src/tui/app.rs", "tuiapp");
        assert!(score.is_some());
    }
}
