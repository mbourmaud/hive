use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::app::App;

/// Render the entire UI
pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();
    
    // Calculate layout
    let (sidebar, content, footer) = app.layout.calculate(size, app.sidebar_visible);
    
    // Render sidebar if visible
    if app.sidebar_visible {
        render_sidebar(f, sidebar, app);
    }

    // Render main content area
    render_content(f, content, app);

    // Render footer
    render_footer(f, footer, app);
}

/// Render the sidebar
fn render_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    // Hive ASCII logo
    let logo = vec![
        Line::from(Span::styled(
            "    __  __  _____   _______",
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "   / / / / /  _/ | / / ____/",
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  / /_/ /  / / | |/ / __/   ",
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " / __  / _/ /  |   / /___   ",
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "/_/ /_/ /___/  |__/_____/   ",
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_sidebar))
        .style(Style::default().bg(theme.bg_sidebar));

    let mut content = logo;
    content.push(Line::from(Span::styled(
        "  📋 Conversations",
        Style::default().fg(theme.fg_primary),
    )));
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "  📁 Files",
        Style::default().fg(theme.fg_muted),
    )));
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "  ⚙️  Settings",
        Style::default().fg(theme.fg_muted),
    )));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Render the main content area (chat messages + input)
fn render_content(f: &mut Frame, area: Rect, app: &mut App) {
    let (messages_area, input_area) = app.layout.calculate_content(area);

    // Render chat messages area
    render_messages(f, messages_area, app);

    // Render input area
    render_input(f, input_area, app);
}

/// Render the chat messages area
fn render_messages(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Chat ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_success))
        .style(Style::default().bg(theme.bg_primary));

    let welcome_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Welcome to Hive Unified TUI!",
            Style::default()
                .fg(theme.accent_warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  This is a unified chat interface for interacting with your Hive drones.",
            Style::default().fg(theme.fg_primary),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Start typing in the input box below to send a message.",
            Style::default().fg(theme.fg_secondary),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Keyboard Shortcuts:",
            Style::default()
                .fg(theme.fg_bright)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "    • Ctrl+Enter: Submit message",
            Style::default().fg(theme.fg_secondary),
        )),
        Line::from(Span::styled(
            "    • Ctrl+T: Toggle theme",
            Style::default().fg(theme.fg_secondary),
        )),
        Line::from(Span::styled(
            "    • Up/Down: Navigate input history",
            Style::default().fg(theme.fg_secondary),
        )),
        Line::from(Span::styled(
            "    • Ctrl+A/E: Move to start/end of line",
            Style::default().fg(theme.fg_secondary),
        )),
        Line::from(Span::styled(
            "    • Ctrl+K/U: Delete to end/start of line",
            Style::default().fg(theme.fg_secondary),
        )),
        Line::from(Span::styled(
            "    • Ctrl+B: Toggle sidebar",
            Style::default().fg(theme.fg_secondary),
        )),
        Line::from(Span::styled(
            "    • q or Ctrl+C: Quit",
            Style::default().fg(theme.fg_secondary),
        )),
    ];

    let paragraph = Paragraph::new(welcome_text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Render the input area
fn render_input(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Input (Ctrl+Enter to submit, / for commands, @ for files, ! for bash) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_input))
        .style(Style::default().bg(theme.bg_input));

    app.input_state.textarea.set_block(block);
    f.render_widget(&app.input_state.textarea, area);

    // Render command autocomplete popup if visible
    if app.input_state.command_autocomplete.visible {
        render_command_autocomplete(f, area, app);
    }

    // Render file picker popup if visible
    if app.input_state.file_picker.visible {
        render_file_picker(f, area, app);
    }
}

/// Render the footer with keybinding hints
fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let sidebar_hint = if app.sidebar_visible {
        "Hide Sidebar"
    } else {
        "Show Sidebar"
    };

    let hints = vec![
        Span::styled(
            " q ",
            Style::default().fg(theme.footer_key_fg).bg(theme.footer_key_bg),
        ),
        Span::raw(" Quit  "),
        Span::styled(
            " Ctrl+B ",
            Style::default().fg(theme.footer_key_fg).bg(theme.footer_key_bg),
        ),
        Span::raw(format!(" {}  ", sidebar_hint)),
        Span::styled(
            " Ctrl+T ",
            Style::default().fg(theme.footer_key_fg).bg(theme.footer_key_bg),
        ),
        Span::raw(" Theme  "),
        Span::styled(
            " Ctrl+Enter ",
            Style::default().fg(theme.footer_key_fg).bg(theme.footer_key_bg),
        ),
        Span::raw(" Submit "),
    ];

    let footer = Paragraph::new(Line::from(hints))
        .style(Style::default().fg(theme.footer_fg).bg(theme.footer_bg));

    f.render_widget(footer, area);
}

/// Render command autocomplete popup
fn render_command_autocomplete(f: &mut Frame, input_area: Rect, app: &mut App) {
    let theme = &app.theme;
    let autocomplete = &app.input_state.command_autocomplete;

    // Calculate popup position (above the input area)
    let height = autocomplete.commands.len().min(8) as u16 + 2; // +2 for borders
    let width = 40;
    let x = input_area.x + 2;
    let y = if input_area.y >= height {
        input_area.y - height
    } else {
        input_area.y + 1
    };

    let popup_area = Rect::new(x, y, width, height);

    // Build content lines
    let mut lines = Vec::new();
    for (i, cmd) in autocomplete.commands.iter().enumerate() {
        let is_selected = i == autocomplete.selected;
        let prefix = if is_selected { "► " } else { "  " };
        let (fg, modifier) = if is_selected {
            (theme.selection_fg, Modifier::BOLD)
        } else {
            (theme.fg_primary, Modifier::empty())
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(fg).add_modifier(modifier)),
            Span::styled(
                format!("/{}", cmd.name()),
                Style::default().fg(fg).add_modifier(modifier),
            ),
            Span::styled(
                format!(" - {}", cmd.description()),
                Style::default().fg(theme.fg_muted),
            ),
        ]));
    }

    let block = Block::default()
        .title(" Commands ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_popup));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_popup));

    // Clear the background
    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

/// Render file picker popup
fn render_file_picker(f: &mut Frame, _input_area: Rect, app: &mut App) {
    let theme = &app.theme;
    let picker = &app.input_state.file_picker;

    // Calculate popup position (centered overlay)
    let width = f.area().width.saturating_sub(20).min(80);
    let height = f.area().height.saturating_sub(10).min(20);
    let x = (f.area().width.saturating_sub(width)) / 2;
    let y = (f.area().height.saturating_sub(height)) / 2;

    let popup_area = Rect::new(x, y, width, height);

    // Build content lines
    let max_items = (height.saturating_sub(4)) as usize; // -4 for borders and query line
    let display_files = picker.get_display_files(max_items);

    let mut lines = Vec::new();

    // Add query line
    lines.push(Line::from(vec![
        Span::styled("Search: ", Style::default().fg(theme.accent_primary)),
        Span::styled(
            &picker.query,
            Style::default()
                .fg(theme.fg_bright)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // Add files
    let start_idx = if picker.selected >= max_items {
        picker.selected - max_items + 1
    } else {
        0
    };

    for (i, path) in display_files.iter().enumerate() {
        let actual_idx = start_idx + i;
        let is_selected = actual_idx == picker.selected;
        let prefix = if is_selected { "► " } else { "  " };
        let (fg, modifier) = if is_selected {
            (theme.selection_fg, Modifier::BOLD)
        } else {
            (theme.fg_primary, Modifier::empty())
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(fg).add_modifier(modifier)),
            Span::styled(
                path.to_string_lossy(),
                Style::default().fg(fg).add_modifier(modifier),
            ),
        ]));
    }

    let title = format!(
        " Files ({}/{}) ",
        picker.filtered_files.len(),
        picker.all_files.len()
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_popup));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_popup));

    // Clear the background and render
    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}
