use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::app::{App, Focus};
use super::dialogs;

pub fn render(frame: &mut Frame, app: &mut App<'_>) {
    let outer = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(outer);

    let content_area = vertical[0];
    let footer_area = vertical[1];

    if app.sidebar_visible {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(content_area);

        render_sidebar(frame, app, horizontal[0]);
        render_chat_panel(frame, app, horizontal[1]);
    } else {
        render_chat_panel(frame, app, content_area);
    }

    render_footer(frame, app, footer_area);

    // Overlays (rendered last, on top)
    if app.commands.visible {
        app.commands.render(frame, &app.theme);
    }

    if app.file_picker.visible {
        let area = dialogs::centered_rect(50, 50, frame.area());
        app.file_picker.render(frame, area, &app.theme);
    }

    if app.sessions.visible {
        app.sessions.render(frame, &app.theme);
    }

    if let Some(dialog) = &app.permission_dialog {
        dialog.render(frame, &app.theme);
    }
}

fn render_sidebar(frame: &mut Frame, app: &App<'_>, area: Rect) {
    let focused = matches!(app.focus, Focus::Sidebar);
    super::sidebar::render(frame, &app.sidebar, focused, area);
}

fn render_chat_panel(frame: &mut Frame, app: &mut App<'_>, area: Rect) {
    let chat_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(5)])
        .split(area);

    let messages_area = chat_layout[0];
    let input_area = chat_layout[1];

    render_chat_messages(frame, app, messages_area);
    render_input(frame, app, input_area);
}

fn render_chat_messages(frame: &mut Frame, app: &mut App<'_>, area: Rect) {
    let focused = matches!(app.focus, Focus::Chat);
    app.chat.render(frame, area, &app.theme, focused);
}

fn render_input(frame: &mut Frame, app: &mut App<'_>, area: Rect) {
    let focused = matches!(app.focus, Focus::Input);
    app.input.set_focus_style(focused);
    app.input.render(frame, area);
}

fn render_footer(frame: &mut Frame, app: &App<'_>, area: Rect) {
    let theme = &app.theme;

    // Show status message if available
    if let Some((msg, _)) = &app.status_message {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style(false))
            .title(" Status ")
            .style(Style::default().bg(theme.bg));

        let footer = Paragraph::new(Line::from(vec![Span::styled(
            msg.as_str(),
            theme.warning_style(),
        )]))
        .block(block);
        frame.render_widget(footer, area);
        return;
    }

    let hints = match app.focus {
        Focus::Sidebar => "q:Quit j/k:Nav Enter:Expand x:Stop c:Clean l:Logs ^B:Sidebar Tab:Focus",
        Focus::Chat => {
            "q:Quit i:Input PgUp/Dn:Scroll ^B:Sidebar ^T:Theme ^N:New ^L:Sessions Tab:Focus"
        }
        Focus::Input => "Esc:Back ^S:Send ^B:Sidebar /:Commands @:Files ^T:Theme",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style(false))
        .title(" Keys ")
        .style(Style::default().bg(theme.bg));

    let footer = Paragraph::new(Span::styled(hints, theme.muted_style())).block(block);
    frame.render_widget(footer, area);
}
