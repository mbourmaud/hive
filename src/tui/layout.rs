use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub sidebar: Option<Rect>,
    pub main_panel: Rect,
    pub footer: Rect,
}

impl AppLayout {
    pub fn compute(area: Rect, sidebar_visible: bool) -> Self {
        // Split vertically: content area + footer (1 row)
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let content_area = vertical[0];
        let footer = vertical[1];

        if sidebar_visible {
            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                .split(content_area);

            Self {
                sidebar: Some(horizontal[0]),
                main_panel: horizontal[1],
                footer,
            }
        } else {
            Self {
                sidebar: None,
                main_panel: content_area,
                footer,
            }
        }
    }
}
