use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
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
        render_sidebar(f, sidebar);
    }

    // Render main content area
    render_content(f, content, app);

    // Render footer
    render_footer(f, footer, app.sidebar_visible);
}

/// Render the sidebar
fn render_sidebar(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Sidebar ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  📋 Conversations",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  📁 Files",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  ⚙️  Settings",
            Style::default().fg(Color::DarkGray),
        )),
    ];

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
    let block = Block::default()
        .title(" Chat ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let lines: Vec<Line> = if app.messages.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Welcome to Hive Unified TUI!",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Type a message below and press Ctrl+Enter to submit."),
        ]
    } else {
        app.messages
            .iter()
            .flat_map(|msg| render_message(msg))
            .collect()
    };

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Render a single message
fn render_message(message: &crate::tui::messages::Message) -> Vec<Line<'static>> {
    use crate::tui::messages::Message;

    match message {
        Message::User(text) => vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "You: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(text.clone()),
            ]),
        ],
        Message::Assistant(text) => vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Claude: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(text.clone()),
            ]),
        ],
        Message::ToolUse { tool, args } => {
            let args_display = if args.len() > 50 {
                format!("{}...", &args[..47])
            } else {
                args.clone()
            };
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("🔧 ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        tool.clone(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(Span::styled(
                    args_display,
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        Message::ToolResult { success, result } => {
            let (icon, color) = if *success {
                ("✓", Color::Green)
            } else {
                ("✗", Color::Red)
            };
            let result_display = if result.len() > 80 {
                format!("{} {}...", icon, &result[..77])
            } else {
                format!("{} {}", icon, result)
            };
            vec![Line::from(Span::styled(
                result_display,
                Style::default().fg(color),
            ))]
        }
        Message::Error(err) => vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "❌ Error: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(err.clone()),
            ]),
        ],
    }
}

/// Render the input area
fn render_input(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title(" Input (Ctrl+Enter to submit) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    app.input_state.textarea.set_block(block);
    f.render_widget(&app.input_state.textarea, area);
}

/// Render the footer with keybinding hints
fn render_footer(f: &mut Frame, area: Rect, sidebar_visible: bool) {
    let sidebar_hint = if sidebar_visible {
        "Hide Sidebar"
    } else {
        "Show Sidebar"
    };

    let hints = vec![
        Span::styled(" q ", Style::default().fg(Color::Black).bg(Color::Gray)),
        Span::raw(" Quit  "),
        Span::styled(
            " Ctrl+B ",
            Style::default().fg(Color::Black).bg(Color::Gray),
        ),
        Span::raw(format!(" {}  ", sidebar_hint)),
        Span::styled(
            " Ctrl+Enter ",
            Style::default().fg(Color::Black).bg(Color::Gray),
        ),
        Span::raw(" Submit  "),
        Span::styled(" ↑↓ ", Style::default().fg(Color::Black).bg(Color::Gray)),
        Span::raw(" History "),
    ];

    let footer = Paragraph::new(Line::from(hints))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));

    f.render_widget(footer, area);
}
