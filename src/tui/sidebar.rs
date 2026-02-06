use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::monitor::{self, DroneInfo, StoryStatus};
use super::theme::Theme;

pub struct SidebarState {
    pub drones: Vec<DroneInfo>,
    pub selected_index: usize,
    pub expanded: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarState {
    pub fn new() -> Self {
        Self {
            drones: Vec::new(),
            selected_index: 0,
            expanded: false,
        }
    }

    pub fn refresh(&mut self) {
        self.drones = monitor::list_drones();
        if self.selected_index >= self.drones.len() && !self.drones.is_empty() {
            self.selected_index = self.drones.len() - 1;
        }
    }

    pub fn select_next(&mut self) {
        if !self.drones.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.drones.len() - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }
}

/// Build a progress bar string like [####....] 3/7
fn progress_bar(completed: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return format!("[{}] 0/0", ".".repeat(width));
    }
    let filled = (completed * width) / total;
    let empty = width - filled;
    format!(
        "[{}{}] {}/{}",
        "#".repeat(filled),
        ".".repeat(empty),
        completed,
        total
    )
}

fn render_logo(frame: &mut Frame, theme: &Theme, area: Rect) {
    let logo_lines = vec![
        Line::from(Span::styled(
            " //    HIVE",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " \\\\  drone ops",
            Style::default().fg(theme.muted),
        )),
    ];
    let logo = Paragraph::new(logo_lines);
    frame.render_widget(logo, area);
}

pub fn render(frame: &mut Frame, sidebar: &SidebarState, focused: bool, area: Rect) {
    let theme = Theme::dark();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style(focused))
        .title(" Drones ")
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner into logo area and drone list
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(inner);

    let logo_area = layout[0];
    let drones_area = layout[1];

    render_logo(frame, &theme, logo_area);

    if sidebar.drones.is_empty() {
        let content = Paragraph::new(Span::styled(
            "No drones running",
            Style::default().fg(theme.muted),
        ));
        frame.render_widget(content, drones_area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for (i, drone) in sidebar.drones.iter().enumerate() {
        let is_selected = i == sidebar.selected_index;
        let (icon, icon_color) = monitor::state_icon(&drone.status);

        let mut spans = vec![
            Span::styled(
                if is_selected { ">" } else { " " },
                Style::default().fg(theme.accent),
            ),
            Span::raw(" "),
            Span::styled(icon, Style::default().fg(icon_color)),
            Span::raw(" "),
        ];

        let name_style = if is_selected {
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        };
        spans.push(Span::styled(drone.name.clone(), name_style));

        lines.push(Line::from(spans));

        let bar = progress_bar(drone.completed, drone.total, 8);
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(bar, Style::default().fg(theme.muted)),
        ]));

        if is_selected && sidebar.expanded && !drone.stories.is_empty() {
            for story in &drone.stories {
                let (story_icon, story_color) = match story.status {
                    StoryStatus::Completed => ("\u{2713}", Color::Green),
                    StoryStatus::InProgress => ("\u{25cf}", Color::Cyan),
                    StoryStatus::Error => ("\u{2717}", Color::Red),
                    StoryStatus::Pending => ("\u{25cb}", Color::DarkGray),
                };

                let max_title = drones_area.width.saturating_sub(8) as usize;
                let title = if story.title.len() > max_title {
                    format!("{}...", &story.title[..max_title.saturating_sub(3)])
                } else {
                    story.title.clone()
                };

                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled(story_icon, Style::default().fg(story_color)),
                    Span::raw(" "),
                    Span::styled(title, Style::default().fg(theme.fg)),
                ]));
            }
        }

        if i < sidebar.drones.len() - 1 {
            lines.push(Line::from(""));
        }
    }

    let scroll_offset = calculate_scroll(
        &sidebar.drones,
        sidebar.selected_index,
        sidebar.expanded,
        drones_area.height as usize,
    );

    let content = Paragraph::new(lines).scroll((scroll_offset as u16, 0));
    frame.render_widget(content, drones_area);
}

/// Calculate vertical scroll offset to keep the selected drone visible.
fn calculate_scroll(
    drones: &[DroneInfo],
    selected: usize,
    expanded: bool,
    visible_height: usize,
) -> usize {
    let mut lines_before = 0usize;
    for (i, drone) in drones.iter().enumerate() {
        if i == selected {
            break;
        }
        lines_before += 2; // name + progress
        if i > 0 {
            lines_before += 1; // separator
        }
        let _ = drone;
    }

    let mut selected_lines = 2usize;
    if selected > 0 {
        lines_before += 1;
    }
    if expanded {
        if let Some(drone) = drones.get(selected) {
            selected_lines += drone.stories.len();
        }
    }

    let end = lines_before + selected_lines;
    if end > visible_height {
        end.saturating_sub(visible_height)
    } else {
        0
    }
}

pub fn handle_key(sidebar: &mut SidebarState, key: crossterm::event::KeyEvent) -> bool {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            sidebar.select_next();
            sidebar.expanded = false;
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            sidebar.select_prev();
            sidebar.expanded = false;
            true
        }
        KeyCode::Enter => {
            sidebar.toggle_expand();
            true
        }
        _ => false,
    }
}
