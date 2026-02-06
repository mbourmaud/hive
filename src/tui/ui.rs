use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
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
            "  \u{1f41d} HIVE",
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            Style::default().fg(theme.accent_primary),
        )),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focused))
        .style(Style::default().bg(theme.bg_primary));

    let lines: Vec<Line> = if app.messages.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Welcome to Hive Unified TUI!",
                Style::default()
                    .fg(theme.accent_warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Type a message below and press Ctrl+Enter to submit.",
                Style::default().fg(theme.fg_secondary),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Shortcuts: Ctrl+T theme | Ctrl+B sidebar | / commands | @ files",
                Style::default().fg(theme.fg_muted),
            )),
        ]
    } else {
        app.messages
            .iter()
            .flat_map(|msg| render_message(msg, theme))
            .collect()
    };

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Render a single message
fn render_message(
    message: &super::messages::Message,
    theme: &super::theme::Theme,
) -> Vec<Line<'static>> {
    use super::messages::Message;

    match message {
        Message::User { content, timestamp } => vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "You: ",
                    Style::default()
                        .fg(theme.msg_user)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(content.clone()),
                Span::styled(
                    format!("  {}", timestamp.format("%H:%M:%S")),
                    Style::default().fg(theme.fg_muted),
                ),
            ]),
        ],
        Message::Assistant { content, timestamp } => vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Claude: ",
                    Style::default()
                        .fg(theme.msg_assistant)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(content.clone()),
                Span::styled(
                    format!("  {}", timestamp.format("%H:%M:%S")),
                    Style::default().fg(theme.fg_muted),
                ),
            ]),
        ],
        Message::ToolUse {
            tool_name,
            args_summary,
            timestamp,
        } => {
            let args_display = if args_summary.len() > 50 {
                format!("{}...", &args_summary[..47])
            } else {
                args_summary.clone()
            };
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("Tool: {}", tool_name),
                        Style::default()
                            .fg(theme.accent_warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {}", timestamp.format("%H:%M:%S")),
                        Style::default().fg(theme.fg_muted),
                    ),
                ]),
                Line::from(Span::styled(
                    args_display,
                    Style::default().fg(theme.fg_muted),
                )),
            ]
        }
        Message::ToolResult {
            success,
            output_summary,
            timestamp,
        } => {
            let (icon, color) = if *success {
                ("Success", theme.accent_success)
            } else {
                ("Failed", theme.accent_error)
            };
            let result_display = if output_summary.len() > 80 {
                format!("{}...", &output_summary[..77])
            } else {
                output_summary.clone()
            };
            vec![Line::from(vec![
                Span::styled(icon, Style::default().fg(color)),
                Span::raw(": "),
                Span::styled(result_display, Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    format!("  {}", timestamp.format("%H:%M:%S")),
                    Style::default().fg(theme.fg_muted),
                ),
            ])]
        }
        Message::Error { content, timestamp } => vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Error: ",
                    Style::default()
                        .fg(theme.accent_error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(content.clone()),
                Span::styled(
                    format!("  {}", timestamp.format("%H:%M:%S")),
                    Style::default().fg(theme.fg_muted),
                ),
            ]),
        ],
        Message::System { content, timestamp } => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("System  {}", timestamp.format("%H:%M:%S")),
                    Style::default()
                        .fg(theme.msg_system)
                        .add_modifier(Modifier::BOLD),
                )),
            ];
            for line in content.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(theme.fg_secondary),
                )));
            }
            lines
        }
    }
}

/// Render the input area
fn render_input(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Input (Ctrl+Enter to submit, / for commands, @ for files, ! for bash) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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
            Style::default()
                .fg(theme.footer_key_fg)
                .bg(theme.footer_key_bg),
        ),
        Span::raw(" Quit  "),
        Span::styled(
            " Ctrl+B ",
            Style::default()
                .fg(theme.footer_key_fg)
                .bg(theme.footer_key_bg),
        ),
        Span::raw(format!(" {}  ", sidebar_hint)),
        Span::styled(
            " Ctrl+T ",
            Style::default()
                .fg(theme.footer_key_fg)
                .bg(theme.footer_key_bg),
        ),
        Span::raw(" Theme  "),
        Span::styled(
            " Ctrl+Enter ",
            Style::default()
                .fg(theme.footer_key_fg)
                .bg(theme.footer_key_bg),
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
        .border_type(BorderType::Rounded)
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_popup));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_popup));

    // Clear the background and render
    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}
