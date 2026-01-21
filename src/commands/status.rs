use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::types::{DroneState, DroneStatus, Prd};

// New monitor command - always runs TUI with auto-refresh
pub fn run_monitor(name: Option<String>) -> Result<()> {
    run_tui(name)
}

// Legacy run function for backward compatibility (can be removed later)
pub fn run(name: Option<String>, interactive: bool, follow: bool) -> Result<()> {
    if interactive {
        run_tui(name)
    } else {
        run_simple(name, follow)
    }
}

fn run_simple(name: Option<String>, follow: bool) -> Result<()> {
    loop {
        // Clear screen in follow mode
        if follow {
            print!("\x1B[2J\x1B[1;1H");
        }

        let drones = list_drones()?;

        if drones.is_empty() {
            println!("{}", "No drones found".yellow());
            println!("\nRun 'hive-rust init' to initialize Hive");
            return Ok(());
        }

        // Filter by name if provided
        let filtered: Vec<_> = if let Some(ref n) = name {
            drones.into_iter()
                .filter(|(drone_name, _)| drone_name == n)
                .collect()
        } else {
            drones
        };

        if filtered.is_empty() {
            eprintln!("Drone '{}' not found", name.unwrap());
            return Ok(());
        }

        // Use yellow/gold for the header with crown emoji
        println!("  {} v{}", "👑 hive".yellow().bold(), env!("CARGO_PKG_VERSION"));
        println!();

        for (drone_name, status) in filtered {
            print_drone_status(&drone_name, &status);
            println!();
        }

        if !follow {
            break;
        }

        // Sleep 30 seconds before refresh
        std::thread::sleep(std::time::Duration::from_secs(30));
    }

    Ok(())
}

// Helper function to parse ISO8601 timestamp
fn parse_timestamp(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

// Helper function to calculate duration between two timestamps
fn duration_between(start: &str, end: &str) -> Option<chrono::Duration> {
    let start_dt = parse_timestamp(start)?;
    let end_dt = parse_timestamp(end)?;
    Some(end_dt.signed_duration_since(start_dt))
}

// Helper function to format duration as "Xh Ym" or "Xm Ys"
fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

// Calculate elapsed time since a timestamp
fn elapsed_since(start: &str) -> Option<String> {
    let start_dt = parse_timestamp(start)?;
    let now = Utc::now();
    let duration = now.signed_duration_since(start_dt);
    Some(format_duration(duration))
}

// Load PRD from path
fn load_prd(path: &PathBuf) -> Option<Prd> {
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn print_drone_status(name: &str, status: &DroneStatus) {
    // Print drone name with status - honey theme with bee emoji
    let status_symbol = match status.status {
        DroneState::Starting => "◐".yellow(),
        DroneState::Resuming => "◐".yellow(),
        DroneState::InProgress => "●".green(),
        DroneState::Completed => "✓".bright_green().bold(),
        DroneState::Error => "✗".red().bold(),
        DroneState::Blocked => "⊗".red().bold(),
        DroneState::Stopped => "○".bright_black(),
    };

    // Calculate total elapsed time
    let elapsed = elapsed_since(&status.started)
        .map(|e| format!("  {}", e))
        .unwrap_or_default();

    println!("  {} {}{}  {}",
             status_symbol,
             format!("🐝 {}", name).yellow().bold(),
             elapsed.bright_black(),
             format!("[{}]", status.status).bright_black());

    // Print progress
    let progress = if status.total > 0 {
        format!("{}/{}", status.completed.len(), status.total)
    } else {
        "0/0".to_string()
    };

    let percentage = if status.total > 0 {
        (status.completed.len() as f32 / status.total as f32 * 100.0) as u32
    } else {
        0
    };

    println!("  Progress: {} ({}%)", progress.bright_white(), percentage);

    // Print progress bar with honey theme (━ filled, ─ empty)
    let bar_width = 40;
    let filled = (bar_width as f32 * percentage as f32 / 100.0) as usize;
    let empty = bar_width - filled;
    let bar = format!("[{}{}]",
                      "━".repeat(filled).green(),
                      "─".repeat(empty).bright_black());
    println!("  {}", bar);

    // Load PRD to get story titles
    let prd_path = PathBuf::from(".hive").join("prds").join(&status.prd);
    if let Some(prd) = load_prd(&prd_path) {
        println!("\n  Stories:");
        for story in &prd.stories {
            let is_completed = status.completed.contains(&story.id);
            let is_current = status.current_story.as_ref() == Some(&story.id);

            let (icon, color_fn): (_, fn(String) -> colored::ColoredString) = if is_completed {
                ("✓", |s| s.green())
            } else if is_current {
                ("▸", |s| s.yellow())
            } else {
                ("○", |s| s.bright_black())
            };

            // Calculate duration
            let duration_str = if let Some(timing) = status.story_times.get(&story.id) {
                if let (Some(started), Some(completed)) = (&timing.started, &timing.completed) {
                    // Completed story - show duration
                    if let Some(dur) = duration_between(started, completed) {
                        format!(" ({})", format_duration(dur))
                    } else {
                        String::new()
                    }
                } else if let Some(started) = &timing.started {
                    // In-progress story - show elapsed time
                    if let Some(elapsed) = elapsed_since(started) {
                        format!(" ({})", elapsed)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let story_line = format!("    {} {} {}{}", icon, story.id, story.title, duration_str);
            println!("{}", color_fn(story_line));
        }
    } else {
        // Fallback: just show current story if PRD not loaded
        if let Some(ref story) = status.current_story {
            println!("  Current: {}", story.bright_yellow());
        }
    }

    // Print blocked reason
    if status.status == DroneState::Blocked {
        if let Some(ref reason) = status.blocked_reason {
            println!("  {} {}", "Blocked:".red().bold(), reason.red());
        }
    }

    // Print error info
    if status.status == DroneState::Error {
        println!("  {} {} errors", "Errors:".red().bold(), status.error_count);
        if let Some(ref last_error_story) = status.last_error_story {
            println!("  Last error in: {}", last_error_story.red());
        }
    }

    // Print metadata
    println!("  Branch: {}", status.branch.bright_black());
    println!("  PRD: {}", status.prd.bright_black());
}

fn list_drones() -> Result<Vec<(String, DroneStatus)>> {
    let hive_dir = PathBuf::from(".hive");
    let drones_dir = hive_dir.join("drones");

    if !drones_dir.exists() {
        return Ok(Vec::new());
    }

    let mut drones = Vec::new();

    for entry in fs::read_dir(&drones_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let drone_name = entry.file_name().to_string_lossy().to_string();
        let status_path = entry.path().join("status.json");

        if status_path.exists() {
            let contents = fs::read_to_string(&status_path)
                .context(format!("Failed to read status for drone '{}'", drone_name))?;
            let status: DroneStatus = serde_json::from_str(&contents)
                .context(format!("Failed to parse status for drone '{}'", drone_name))?;
            drones.push((drone_name, status));
        }
    }

    // Sort by updated timestamp (most recent first)
    drones.sort_by(|a, b| b.1.updated.cmp(&a.1.updated));

    Ok(drones)
}

fn run_tui(_name: Option<String>) -> Result<()> {
    // TUI implementation with ratatui
    use crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, List, ListItem, Paragraph},
        Terminal,
    };
    use std::io;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        let drones = list_drones()?;

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // Title with honey theme
            let title = Paragraph::new(format!("👑 hive v{}", env!("CARGO_PKG_VERSION")))
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // Drone list
            let items: Vec<ListItem> = drones.iter().map(|(name, status)| {
                let status_color = match status.status {
                    DroneState::Starting => Color::Yellow,
                    DroneState::Resuming => Color::Yellow,
                    DroneState::InProgress => Color::Green,
                    DroneState::Completed => Color::Green,
                    DroneState::Error => Color::Red,
                    DroneState::Blocked => Color::Red,
                    DroneState::Stopped => Color::DarkGray,
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

                // Calculate elapsed time
                let elapsed = elapsed_since(&status.started)
                    .unwrap_or_else(|| "?".to_string());

                let line = Line::from(vec![
                    Span::styled(format!("🐝 {:<18}", name), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(format!("{:<10}", elapsed), Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(format!("{:<15}", status.status.to_string()), Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(format!("{:>6} ({:>3}%)", progress, percentage), Style::default().fg(Color::White)),
                ]);

                ListItem::new(line)
            }).collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Drones"));
            f.render_widget(list, chunks[1]);

            // Footer
            let footer = Paragraph::new("Press 'q' to quit")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        // Handle input with 1s refresh interval
        if crossterm::event::poll(std::time::Duration::from_secs(1))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if key.code == crossterm::event::KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
