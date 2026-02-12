use hive_lib::types::{DroneState, DroneStatus, ExecutionMode};
use ratatui::{
    backend::TestBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::collections::HashMap;

// Helper function to render a widget to a string buffer
fn render_to_string<F>(width: u16, height: u16, render_fn: F) -> String
where
    F: FnOnce(&mut ratatui::Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_fn(f);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    buffer_to_string(&buffer, width, height)
}

fn buffer_to_string(buffer: &ratatui::buffer::Buffer, width: u16, height: u16) -> String {
    let mut result = String::new();
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            result.push_str(cell.symbol());
        }
        if y < height - 1 {
            result.push('\n');
        }
    }
    result
}

fn create_mock_drone(
    name: &str,
    status: DroneState,
    completed: Vec<&str>,
    total: usize,
    current_task: Option<&str>,
) -> (String, DroneStatus) {
    (
        name.to_string(),
        DroneStatus {
            drone: name.to_string(),
            prd: "prd-test.json".to_string(),
            branch: format!("hive/{}", name),
            worktree: format!("/tmp/hive/{}", name),
            local_mode: false,
            execution_mode: ExecutionMode::AgentTeam,
            backend: "agent_team".to_string(),
            status,
            current_task: current_task.map(String::from),
            completed: completed.iter().map(|s| s.to_string()).collect(),
            story_times: HashMap::new(),
            total,
            started: "2024-01-01T00:00:00Z".to_string(),
            updated: "2024-01-01T00:00:00Z".to_string(),
            error_count: 0,
            last_error: None,
            lead_model: None,
            active_agents: HashMap::new(),
        },
    )
}

#[test]
fn tui_status_dashboard_single_drone() {
    let drone = create_mock_drone(
        "test-drone",
        DroneState::InProgress,
        vec!["TASK-1", "TASK-2"],
        5,
        Some("TASK-3"),
    );
    let drones = [drone];

    let output = render_to_string(80, 24, |f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("Hive Status Dashboard")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Drone list
        let items: Vec<ListItem> = drones
            .iter()
            .map(|(name, status)| {
                let status_color = match status.status {
                    DroneState::Starting => Color::Yellow,
                    DroneState::Resuming => Color::Yellow,
                    DroneState::InProgress => Color::Green,
                    DroneState::Completed => Color::Green,
                    DroneState::Error => Color::Red,
                    DroneState::Stopped | DroneState::Cleaning | DroneState::Zombie => {
                        Color::DarkGray
                    }
                };

                let progress = if status.total > 0 {
                    format!("{}/{}", status.completed.len(), status.total)
                } else {
                    "0/0".to_string()
                };

                let percentage = if status.total > 0 {
                    (status.completed.len() as f32 / status.total as f32 * 100.0) as u16
                } else {
                    0
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{:<20}", name),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:<15}", status.status.to_string()),
                        Style::default().fg(status_color),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:>6} ({:>3}%)", progress, percentage),
                        Style::default().fg(Color::White),
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Drones"));
        f.render_widget(list, chunks[1]);

        // Footer
        let footer = Paragraph::new("Press 'q' to quit")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    });

    insta::assert_snapshot!("tui_status_dashboard_single_drone", output);
}

#[test]
fn tui_status_dashboard_multiple_drones() {
    let drones = [
        create_mock_drone(
            "frontend",
            DroneState::InProgress,
            vec!["TASK-1"],
            3,
            Some("TASK-2"),
        ),
        create_mock_drone(
            "backend",
            DroneState::Completed,
            vec!["TASK-1", "TASK-2", "TASK-3"],
            3,
            None,
        ),
        create_mock_drone(
            "database",
            DroneState::Error,
            vec!["TASK-1"],
            5,
            Some("TASK-2"),
        ),
    ];

    let output = render_to_string(80, 24, |f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("Hive Status Dashboard")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Drone list
        let items: Vec<ListItem> = drones
            .iter()
            .map(|(name, status)| {
                let status_color = match status.status {
                    DroneState::Starting => Color::Yellow,
                    DroneState::Resuming => Color::Yellow,
                    DroneState::InProgress => Color::Green,
                    DroneState::Completed => Color::Green,
                    DroneState::Error => Color::Red,
                    DroneState::Stopped | DroneState::Cleaning | DroneState::Zombie => {
                        Color::DarkGray
                    }
                };

                let progress = if status.total > 0 {
                    format!("{}/{}", status.completed.len(), status.total)
                } else {
                    "0/0".to_string()
                };

                let percentage = if status.total > 0 {
                    (status.completed.len() as f32 / status.total as f32 * 100.0) as u16
                } else {
                    0
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{:<20}", name),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:<15}", status.status.to_string()),
                        Style::default().fg(status_color),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:>6} ({:>3}%)", progress, percentage),
                        Style::default().fg(Color::White),
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Drones"));
        f.render_widget(list, chunks[1]);

        // Footer
        let footer = Paragraph::new("Press 'q' to quit")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    });

    insta::assert_snapshot!("tui_status_dashboard_multiple_drones", output);
}

#[test]
fn tui_drone_detail_view() {
    let drone = create_mock_drone(
        "test-drone",
        DroneState::InProgress,
        vec!["TASK-1", "TASK-2"],
        5,
        Some("TASK-3"),
    );

    let output = render_to_string(80, 24, |f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new(format!("Drone: {}", drone.0))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Status info
        let status_text = format!(
            "Status: {}\nProgress: {}/{} ({}%)\nBranch: {}\nPRD: {}",
            drone.1.status,
            drone.1.completed.len(),
            drone.1.total,
            if drone.1.total > 0 {
                (drone.1.completed.len() as f32 / drone.1.total as f32 * 100.0) as u16
            } else {
                0
            },
            drone.1.branch,
            drone.1.prd
        );
        let status_widget = Paragraph::new(status_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status_widget, chunks[1]);

        // Current story
        let current_text = if let Some(ref story) = drone.1.current_task {
            format!("Working on: {}", story)
        } else {
            "No task in progress".to_string()
        };
        let current_widget = Paragraph::new(current_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Current"));
        f.render_widget(current_widget, chunks[2]);

        // Completed stories
        let completed_items: Vec<ListItem> = drone
            .1
            .completed
            .iter()
            .map(|story| ListItem::new(format!("✓ {}", story)))
            .collect();
        let completed_list = List::new(completed_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Completed Tasks"),
        );
        f.render_widget(completed_list, chunks[3]);

        // Footer
        let footer = Paragraph::new("Press 'q' to quit, 'j/k' to navigate")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[4]);
    });

    insta::assert_snapshot!("tui_drone_detail_view", output);
}

#[test]
fn tui_progress_bar() {
    let percentages = vec![0, 25, 50, 75, 100];

    for pct in percentages {
        let output = render_to_string(50, 3, |f| {
            let bar_width = 40;
            let filled = (bar_width as f32 * pct as f32 / 100.0) as usize;
            let empty = bar_width - filled;
            let bar_text = format!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), pct);

            let bar = Paragraph::new(bar_text)
                .style(Style::default().fg(Color::Green))
                .block(Block::default().borders(Borders::ALL));

            f.render_widget(bar, f.area());
        });

        insta::assert_snapshot!(format!("tui_progress_bar_{}", pct), output);
    }
}

#[test]
fn tui_session_list() {
    let output = render_to_string(80, 24, |f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("Claude Sessions - test-drone")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Session list
        let sessions = [
            "2024-01-01 10:00:00 - Session 1 (45 messages)",
            "2024-01-01 11:30:00 - Session 2 (32 messages)",
            "2024-01-01 14:15:00 - Session 3 (18 messages)",
        ];

        let items: Vec<ListItem> = sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let style = if i == 1 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(*session, style)))
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Sessions"));
        f.render_widget(list, chunks[1]);

        // Footer
        let footer =
            Paragraph::new("Press 'Enter' to view session, 'q' to quit, 'j/k' to navigate")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    });

    insta::assert_snapshot!("tui_session_list", output);
}

#[test]
fn tui_conversation_view() {
    let output = render_to_string(80, 24, |f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("Session: 2024-01-01 10:00:00")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Conversation
        let messages = [
            "[USER] Can you help me implement a feature?",
            "[ASSISTANT] Of course! I'd be happy to help.",
            "[TOOL: Read] Reading file: src/main.rs",
            "[TOOL RESULT] File content...",
        ];

        let items: Vec<ListItem> = messages
            .iter()
            .map(|msg| {
                let (style, content) = if msg.starts_with("[USER]") {
                    (Style::default().fg(Color::Green), *msg)
                } else if msg.starts_with("[ASSISTANT]") {
                    (Style::default().fg(Color::Cyan), *msg)
                } else if msg.starts_with("[TOOL") {
                    (Style::default().fg(Color::Yellow), *msg)
                } else {
                    (Style::default().fg(Color::White), *msg)
                };
                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let list =
            List::new(items).block(Block::default().borders(Borders::ALL).title("Conversation"));
        f.render_widget(list, chunks[1]);

        // Footer
        let footer = Paragraph::new("Press 'q' to go back, '/' to search, 'j/k' to scroll")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    });

    insta::assert_snapshot!("tui_conversation_view", output);
}

#[test]
fn tui_search_mode() {
    let output = render_to_string(80, 10, |f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(f.area());

        // Content
        let content = Paragraph::new("Content with searchable text...")
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Session"));
        f.render_widget(content, chunks[0]);

        // Search bar
        let search_text = "Search: API";
        let search_bar = Paragraph::new(search_text)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search (2 results)"),
            );
        f.render_widget(search_bar, chunks[1]);
    });

    insta::assert_snapshot!("tui_search_mode", output);
}
