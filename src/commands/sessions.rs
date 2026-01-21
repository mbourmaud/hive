use anyhow::{bail, Context, Result};
use colored::Colorize;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMessage {
    #[serde(rename = "type")]
    message_type: String,
    #[serde(flatten)]
    content: serde_json::Value,
}

#[derive(Debug, Clone)]
struct Session {
    path: PathBuf,
    name: String,
    timestamp: String,
    message_count: usize,
}

#[derive(Debug, Clone)]
struct ConversationItem {
    message_type: String,
    content: String,
    tool_name: Option<String>,
    tool_input: Option<String>,
    tool_output: Option<String>,
    collapsed: bool,
}

struct SessionViewerState {
    sessions: Vec<Session>,
    selected_session: usize,
    viewing_session: Option<usize>,
    conversation: Vec<ConversationItem>,
    conversation_scroll: usize,
    conversation_state: ListState,
    search_mode: bool,
    search_query: String,
    search_results: Vec<usize>,
    current_search_result: usize,
}

pub fn run(name: String, latest: bool) -> Result<()> {
    let drone_dir = PathBuf::from(".hive/drones").join(&name);

    if !drone_dir.exists() {
        bail!("Drone '{}' not found", name);
    }

    let status_path = drone_dir.join("status.json");
    if !status_path.exists() {
        bail!("Drone status not found for '{}'", name);
    }

    let status_content = fs::read_to_string(&status_path)?;
    let status: serde_json::Value = serde_json::from_str(&status_content)?;

    let worktree_path = status["worktree"]
        .as_str()
        .context("Failed to read worktree path from status")?;

    let sessions = find_sessions(worktree_path)?;

    if sessions.is_empty() {
        println!("{}", "No Claude sessions found for this drone".yellow());
        return Ok(());
    }

    if latest {
        if let Some(latest_session) = sessions.last() {
            view_session_directly(&latest_session.path)?;
        }
        return Ok(());
    }

    run_tui(sessions)?;

    Ok(())
}

fn find_sessions(worktree_path: &str) -> Result<Vec<Session>> {
    let home = dirs::home_dir().context("Failed to get home directory")?;
    let claude_projects = home.join(".claude").join("projects");

    if !claude_projects.exists() {
        return Ok(Vec::new());
    }

    let worktree_normalized = worktree_path.replace(['/', '\\'], "-");
    let session_pattern = format!("-{}", worktree_normalized.trim_start_matches('-'));

    let mut sessions = Vec::new();

    for entry in fs::read_dir(&claude_projects)? {
        let entry = entry?;
        let dir_name = entry.file_name();
        let dir_name_str = dir_name.to_string_lossy();

        if dir_name_str.contains(&session_pattern) {
            let session_dir = entry.path();

            for session_file in fs::read_dir(&session_dir)? {
                let session_file = session_file?;
                let file_path = session_file.path();

                if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                    let metadata = fs::metadata(&file_path)?;
                    let modified = metadata.modified()?;
                    let timestamp = format!("{:?}", modified);

                    let message_count = count_messages(&file_path)?;

                    sessions.push(Session {
                        path: file_path.clone(),
                        name: file_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string(),
                        timestamp,
                        message_count,
                    });
                }
            }
        }
    }

    sessions.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(sessions)
}

fn count_messages(path: &Path) -> Result<usize> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().count())
}

fn view_session_directly(session_path: &Path) -> Result<()> {
    let conversation = parse_session(session_path)?;

    println!("{}", "Session Content".bright_cyan().bold());
    println!();

    for item in conversation {
        match item.message_type.as_str() {
            "user" => {
                println!("{}", "User:".bright_green().bold());
                println!("{}", item.content);
                println!();
            }
            "assistant" => {
                println!("{}", "Assistant:".bright_blue().bold());
                println!("{}", item.content);
                println!();
            }
            "tool_use" => {
                if let Some(name) = &item.tool_name {
                    println!("{} {}", "Tool:".bright_yellow().bold(), name.bright_yellow());
                    if let Some(input) = &item.tool_input {
                        println!("Input: {}", input);
                    }
                    println!();
                }
            }
            "tool_result" => {
                if let Some(output) = &item.tool_output {
                    println!("{}", "Tool Result:".bright_magenta().bold());
                    println!("{}", output);
                    println!();
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn parse_session(session_path: &Path) -> Result<Vec<ConversationItem>> {
    let file = fs::File::open(session_path)?;
    let reader = BufReader::new(file);
    let mut items = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let msg: SessionMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue,
        };

        match msg.message_type.as_str() {
            "user" | "assistant" => {
                let content = msg.content.get("content")
                    .and_then(|c| {
                        if c.is_string() {
                            c.as_str().map(|s| s.to_string())
                        } else if c.is_array() {
                            Some(extract_text_from_content_array(c))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "No content".to_string());

                items.push(ConversationItem {
                    message_type: msg.message_type.clone(),
                    content,
                    tool_name: None,
                    tool_input: None,
                    tool_output: None,
                    collapsed: false,
                });
            }
            "tool_use" => {
                let tool_name = msg.content.get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let tool_input = msg.content.get("input")
                    .map(|i| serde_json::to_string_pretty(i).unwrap_or_default());

                items.push(ConversationItem {
                    message_type: msg.message_type.clone(),
                    content: format!("Tool: {}", tool_name),
                    tool_name: Some(tool_name),
                    tool_input,
                    tool_output: None,
                    collapsed: true,
                });
            }
            "tool_result" => {
                let tool_output = msg.content.get("content")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());

                items.push(ConversationItem {
                    message_type: msg.message_type.clone(),
                    content: "Tool Result".to_string(),
                    tool_name: None,
                    tool_input: None,
                    tool_output,
                    collapsed: true,
                });
            }
            _ => {}
        }
    }

    Ok(items)
}

fn extract_text_from_content_array(content: &serde_json::Value) -> String {
    content.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    if item.get("type")?.as_str()? == "text" {
                        item.get("text")?.as_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn run_tui(sessions: Vec<Session>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = SessionViewerState {
        sessions: sessions.clone(),
        selected_session: sessions.len().saturating_sub(1),
        viewing_session: None,
        conversation: Vec::new(),
        conversation_scroll: 0,
        conversation_state: ListState::default(),
        search_mode: false,
        search_query: String::new(),
        search_results: Vec::new(),
        current_search_result: 0,
    };

    let result = run_app(&mut terminal, &mut state);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut SessionViewerState,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, state))?;

        if let Event::Key(key) = event::read()? {
            if state.search_mode {
                match key.code {
                    KeyCode::Char(c) => {
                        state.search_query.push(c);
                        perform_search(state);
                    }
                    KeyCode::Backspace => {
                        state.search_query.pop();
                        perform_search(state);
                    }
                    KeyCode::Enter | KeyCode::Esc => {
                        state.search_mode = false;
                    }
                    _ => {}
                }
            } else if state.viewing_session.is_some() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        state.viewing_session = None;
                        state.conversation.clear();
                        state.conversation_scroll = 0;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        state.conversation_scroll = state.conversation_scroll.saturating_add(1);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.conversation_scroll = state.conversation_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        state.conversation_scroll = state.conversation_scroll.saturating_add(10);
                    }
                    KeyCode::PageUp => {
                        state.conversation_scroll = state.conversation_scroll.saturating_sub(10);
                    }
                    KeyCode::Char('e') => {
                        if let Some(idx) = state.viewing_session {
                            export_to_markdown(&state.sessions[idx], &state.conversation)?;
                        }
                    }
                    KeyCode::Char('/') => {
                        state.search_mode = true;
                        state.search_query.clear();
                    }
                    KeyCode::Char('n') => {
                        if !state.search_results.is_empty() {
                            state.current_search_result =
                                (state.current_search_result + 1) % state.search_results.len();
                            state.conversation_scroll = state.search_results[state.current_search_result];
                        }
                    }
                    KeyCode::Char('N') => {
                        if !state.search_results.is_empty() {
                            state.current_search_result =
                                if state.current_search_result == 0 {
                                    state.search_results.len() - 1
                                } else {
                                    state.current_search_result - 1
                                };
                            state.conversation_scroll = state.search_results[state.current_search_result];
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = state.conversation_state.selected() {
                            if selected < state.conversation.len() {
                                state.conversation[selected].collapsed = !state.conversation[selected].collapsed;
                            }
                        }
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => {
                        if state.selected_session < state.sessions.len().saturating_sub(1) {
                            state.selected_session += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.selected_session = state.selected_session.saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        let session = &state.sessions[state.selected_session];
                        state.conversation = parse_session(&session.path)?;
                        state.viewing_session = Some(state.selected_session);
                        state.conversation_scroll = 0;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn perform_search(state: &mut SessionViewerState) {
    if state.search_query.is_empty() {
        state.search_results.clear();
        return;
    }

    state.search_results = state.conversation
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            item.content.to_lowercase().contains(&state.search_query.to_lowercase())
        })
        .map(|(i, _)| i)
        .collect();

    state.current_search_result = 0;
    if !state.search_results.is_empty() {
        state.conversation_scroll = state.search_results[0];
    }
}

fn export_to_markdown(session: &Session, conversation: &[ConversationItem]) -> Result<()> {
    let export_path = format!("session-{}.md", session.name.replace(".jsonl", ""));
    let mut content = String::new();

    content.push_str(&format!("# Session: {}\n\n", session.name));
    content.push_str(&format!("Timestamp: {}\n\n", session.timestamp));
    content.push_str("---\n\n");

    for item in conversation {
        match item.message_type.as_str() {
            "user" => {
                content.push_str("## User\n\n");
                content.push_str(&item.content);
                content.push_str("\n\n");
            }
            "assistant" => {
                content.push_str("## Assistant\n\n");
                content.push_str(&item.content);
                content.push_str("\n\n");
            }
            "tool_use" => {
                if let Some(name) = &item.tool_name {
                    content.push_str(&format!("### Tool: {}\n\n", name));
                    if let Some(input) = &item.tool_input {
                        content.push_str("```json\n");
                        content.push_str(input);
                        content.push_str("\n```\n\n");
                    }
                }
            }
            "tool_result" => {
                content.push_str("### Tool Result\n\n");
                if let Some(output) = &item.tool_output {
                    content.push_str("```\n");
                    content.push_str(output);
                    content.push_str("\n```\n\n");
                }
            }
            _ => {}
        }
    }

    fs::write(&export_path, content)?;
    println!("{} {}", "Exported to:".green(), export_path.bright_green());

    Ok(())
}

fn ui(f: &mut Frame, state: &mut SessionViewerState) {
    if let Some(_idx) = state.viewing_session {
        render_conversation_view(f, state);
    } else {
        render_session_list(f, state);
    }
}

fn render_session_list(f: &mut Frame, state: &SessionViewerState) {
    let area = f.area();

    let items: Vec<ListItem> = state.sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let style = if i == state.selected_session {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let content = format!("{} ({} messages)", session.name, session.message_count);
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Claude Sessions (↑/↓ navigate, Enter view, q quit) ")
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));

    f.render_widget(list, area);
}

fn render_conversation_view(f: &mut Frame, state: &mut SessionViewerState) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let visible_items: Vec<Line> = state.conversation
        .iter()
        .skip(state.conversation_scroll)
        .take(chunks[0].height as usize)
        .flat_map(render_conversation_item)
        .collect();

    let paragraph = Paragraph::new(visible_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Conversation (↑/↓ scroll, / search, e export, q back) "));

    f.render_widget(paragraph, chunks[0]);

    let help_text = if state.search_mode {
        format!("Search: {}", state.search_query)
    } else if !state.search_results.is_empty() {
        format!("Search results: {} (n/N navigate)", state.search_results.len())
    } else {
        "j/k or ↑/↓: scroll | /: search | e: export | q: back".to_string()
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, chunks[1]);
}

fn render_conversation_item(item: &ConversationItem) -> Vec<Line<'_>> {
    let mut lines = Vec::new();

    match item.message_type.as_str() {
        "user" => {
            lines.push(Line::from(vec![
                Span::styled("User: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(item.content.clone()));
            lines.push(Line::from(""));
        }
        "assistant" => {
            lines.push(Line::from(vec![
                Span::styled("Assistant: ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(item.content.clone()));
            lines.push(Line::from(""));
        }
        "tool_use" => {
            if let Some(name) = &item.tool_name {
                let prefix = if item.collapsed { "▶ " } else { "▼ " };
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::Yellow)),
                    Span::styled(format!("Tool: {}", name), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]));

                if !item.collapsed {
                    if let Some(input) = &item.tool_input {
                        lines.push(Line::from("  Input:"));
                        for line in input.lines().take(10) {
                            lines.push(Line::from(format!("    {}", line)));
                        }
                    }
                }
                lines.push(Line::from(""));
            }
        }
        "tool_result" => {
            let prefix = if item.collapsed { "▶ " } else { "▼ " };
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Magenta)),
                Span::styled("Tool Result", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            ]));

            if !item.collapsed {
                if let Some(output) = &item.tool_output {
                    for line in output.lines().take(10) {
                        lines.push(Line::from(format!("  {}", line)));
                    }
                }
            }
            lines.push(Line::from(""));
        }
        _ => {}
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_messages() {
        let temp_dir = std::env::temp_dir().join("hive-test-sessions");
        fs::create_dir_all(&temp_dir).unwrap();

        let session_path = temp_dir.join("test.jsonl");
        let mut file = fs::File::create(&session_path).unwrap();
        use std::io::Write;
        writeln!(file, r#"{{"type":"user","content":"hello"}}"#).unwrap();
        writeln!(file, r#"{{"type":"assistant","content":"hi"}}"#).unwrap();

        let count = count_messages(&session_path).unwrap();
        assert_eq!(count, 2);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_parse_session() {
        let temp_dir = std::env::temp_dir().join("hive-test-parse");
        fs::create_dir_all(&temp_dir).unwrap();

        let session_path = temp_dir.join("test.jsonl");
        let mut file = fs::File::create(&session_path).unwrap();
        use std::io::Write;
        writeln!(file, r#"{{"type":"user","content":"hello"}}"#).unwrap();
        writeln!(file, r#"{{"type":"assistant","content":"hi there"}}"#).unwrap();

        let items = parse_session(&session_path).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].message_type, "user");
        assert_eq!(items[1].message_type, "assistant");

        fs::remove_dir_all(&temp_dir).ok();
    }
}
