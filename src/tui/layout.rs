use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Manages the TUI layout calculations
pub struct AppLayout {
    /// Cached layout chunks
    #[allow(dead_code)]
    cached_chunks: Option<(Rect, Vec<Rect>)>,
}

impl AppLayout {
    pub fn new() -> Self {
        Self {
            cached_chunks: None,
        }
    }

    /// Calculate the main layout with sidebar and footer
    /// Returns: (main_area, [sidebar, content, footer])
    pub fn calculate(&mut self, area: Rect, sidebar_visible: bool) -> (Rect, Rect, Rect) {
        // Vertical split: main area (top) and footer (bottom)
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main area
                Constraint::Length(3), // Footer
            ])
            .split(area);

        let main_area = vertical_chunks[0];
        let footer = vertical_chunks[1];

        // Horizontal split in main area: sidebar (if visible) and content
        let content = if sidebar_visible {
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20), // Sidebar (20%)
                    Constraint::Percentage(80), // Content (80%)
                ])
                .split(main_area);

            horizontal_chunks[1]
        } else {
            main_area
        };

        let sidebar = if sidebar_visible {
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20), // Sidebar (20%)
                    Constraint::Percentage(80), // Content (80%)
                ])
                .split(main_area);
            horizontal_chunks[0]
        } else {
            Rect::default()
        };

        (sidebar, content, footer)
    }

    /// Calculate the content area layout (chat messages + input)
    /// Returns: (messages_area, input_area)
    pub fn calculate_content(&self, content_area: Rect) -> (Rect, Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Messages area
                Constraint::Length(5), // Input area (multiline)
            ])
            .split(content_area);

        (chunks[0], chunks[1])
    }
}
