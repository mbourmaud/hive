use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::types::{DroneState, DroneStatus, Prd};

// New monitor command - auto-refresh TUI by default, simple mode for scripts/CI
pub fn run_monitor(name: Option<String>, simple: bool) -> Result<()> {
    if simple {
        run_simple(name, false)
    } else {
        run_tui(name)
    }
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
            drones
                .into_iter()
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
        println!(
            "  {} v{}",
            "👑 hive".yellow().bold(),
            env!("CARGO_PKG_VERSION")
        );
        println!();

        // Sort: active drones first, completed last
        let mut sorted = filtered;
        sorted.sort_by_key(|(_, status)| {
            match status.status {
                DroneState::Completed => 1, // Completed last
                _ => 0,                     // Everything else first
            }
        });

        for (drone_name, status) in &sorted {
            // Use collapsed view for completed drones
            let collapsed = status.status == DroneState::Completed;
            print_drone_status(drone_name, status, collapsed);
            println!();
        }

        // Check for inactive completed drones and suggest cleanup (only once per session)
        if !follow {
            suggest_cleanup_for_inactive(&sorted);
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

// Check if a process is running by PID
fn is_process_running(pid: i32) -> bool {
    #[cfg(unix)]
    {
        // On Unix, send signal 0 to check if process exists
        // This doesn't actually send a signal, just checks permission
        match nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None) {
            Ok(_) => true,
            Err(nix::errno::Errno::ESRCH) => false, // No such process
            Err(_) => true, // Process exists but we don't have permission (still running)
        }
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, check if /proc/<pid> exists
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
}

// Read PID from drone's .pid file
fn read_drone_pid(drone_name: &str) -> Option<i32> {
    let pid_path = PathBuf::from(".hive")
        .join("drones")
        .join(drone_name)
        .join(".pid");

    let pid_str = fs::read_to_string(pid_path).ok()?;
    pid_str.trim().parse().ok()
}

// Suggest cleanup for inactive completed drones
fn suggest_cleanup_for_inactive(drones: &[(String, DroneStatus)]) {
    use std::env;

    // Get threshold from env var or use default (3600 seconds = 1 hour)
    let threshold_seconds = env::var("HIVE_INACTIVE_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(3600);

    let now = Utc::now();

    for (name, status) in drones {
        // Only check completed drones
        if status.status != DroneState::Completed {
            continue;
        }

        // Parse updated timestamp
        let updated = match parse_timestamp(&status.updated) {
            Some(dt) => dt,
            None => continue,
        };

        // Calculate inactive duration
        let inactive_seconds = now.signed_duration_since(updated).num_seconds();

        if inactive_seconds > threshold_seconds {
            // Calculate human-readable duration
            let hours = inactive_seconds / 3600;
            let minutes = (inactive_seconds % 3600) / 60;

            let duration_str = if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };

            println!();
            println!(
                "{} Drone {} completed {} ago. Clean up? {}",
                "💡".bright_yellow(),
                name.bright_cyan(),
                duration_str.bright_black(),
                format!("(hive clean {})", name).bright_black()
            );

            // Only suggest one cleanup per run to avoid spam
            break;
        }
    }
}

fn print_drone_status(name: &str, status: &DroneStatus, collapsed: bool) {
    // Check if process is actually running
    let process_running = read_drone_pid(name)
        .map(is_process_running)
        .unwrap_or(false);

    // Determine status symbol based on actual state and process status
    let status_symbol = match status.status {
        DroneState::Starting => "◐".yellow(),
        DroneState::Resuming => "◐".yellow(),
        DroneState::InProgress => {
            if process_running {
                "●".green() // Actually running
            } else {
                "○".yellow() // Says in_progress but process is dead
            }
        }
        DroneState::Completed => "✓".bright_green().bold(),
        DroneState::Error => "✗".red().bold(),
        DroneState::Blocked => "⚠".red().bold(),
        DroneState::Stopped => "○".bright_black(),
    };

    // Calculate total elapsed time
    let elapsed = elapsed_since(&status.started)
        .map(|e| format!("  {}", e))
        .unwrap_or_default();

    // If collapsed view (completed drones), show single line
    if collapsed {
        let progress = if status.total > 0 {
            format!("{}/{}", status.completed.len(), status.total)
        } else {
            "0/0".to_string()
        };

        println!(
            "  {} {}{}  {}",
            status_symbol,
            format!("🐝 {}", name).bright_black(),
            elapsed.bright_black(),
            progress.bright_black()
        );
        return; // Exit early, don't show full details
    }

    // Full view for active drones
    println!(
        "  {} {}{}  {}",
        status_symbol,
        format!("🐝 {}", name).yellow().bold(),
        elapsed.bright_black(),
        format!("[{}]", status.status).bright_black()
    );

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
    let bar = format!(
        "[{}{}]",
        "━".repeat(filled).green(),
        "─".repeat(empty).bright_black()
    );
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
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
        Terminal,
    };
    use std::collections::HashSet;
    use std::io;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut selected_index: usize = 0;
    let mut message: Option<String> = None;
    let mut message_color = Color::Green;
    let mut expanded_drones: HashSet<String> = HashSet::new();
    let mut scroll_offset: usize = 0;

    loop {
        let mut drones = list_drones()?;

        // Sort: in_progress first, then blocked, then completed
        drones.sort_by_key(|(_, status)| match status.status {
            DroneState::InProgress | DroneState::Starting | DroneState::Resuming => 0,
            DroneState::Blocked | DroneState::Error => 1,
            DroneState::Stopped => 2,
            DroneState::Completed => 3,
        });

        // Clamp selected index
        if !drones.is_empty() && selected_index >= drones.len() {
            selected_index = drones.len() - 1;
        }

        // Load PRDs for story info
        let prd_cache: std::collections::HashMap<String, Prd> = drones
            .iter()
            .filter_map(|(_, status)| {
                let prd_path = PathBuf::from(".hive").join("prds").join(&status.prd);
                load_prd(&prd_path).map(|prd| (status.prd.clone(), prd))
            })
            .collect();

        terminal.draw(|f| {
            let area = f.area();

            // Main layout: header, content, footer
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Header
                    Constraint::Min(0),    // Content
                    Constraint::Length(2), // Footer
                ])
                .split(area);

            // Header - minimal, no borders
            let header = Line::from(vec![
                Span::styled("  👑 ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "hive",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" v{}", env!("CARGO_PKG_VERSION")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{} drones", drones.len()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            f.render_widget(Paragraph::new(header), chunks[0]);

            // Build content lines
            let mut lines: Vec<Line> = Vec::new();
            let mut drone_line_indices: Vec<usize> = Vec::new(); // Track which line each drone header is on

            for (idx, (name, status)) in drones.iter().enumerate() {
                drone_line_indices.push(lines.len());

                let is_selected = idx == selected_index;
                let is_expanded = expanded_drones.contains(name);
                let process_running = read_drone_pid(name)
                    .map(is_process_running)
                    .unwrap_or(false);

                // Status icon and color
                let (icon, status_color) = match status.status {
                    DroneState::Starting | DroneState::Resuming => ("◐", Color::Yellow),
                    DroneState::InProgress => {
                        if process_running {
                            ("●", Color::Green)
                        } else {
                            ("○", Color::Yellow)
                        }
                    }
                    DroneState::Completed => ("✓", Color::Green),
                    DroneState::Error => ("✗", Color::Red),
                    DroneState::Blocked => ("⚠", Color::Rgb(255, 165, 0)), // Orange
                    DroneState::Stopped => ("○", Color::DarkGray),
                };

                let percentage = if status.total > 0 {
                    (status.completed.len() as f32 / status.total as f32 * 100.0) as u16
                } else {
                    0
                };

                // Build progress bar (20 chars wide)
                let bar_width = 20;
                let filled = (bar_width as f32 * percentage as f32 / 100.0) as usize;
                let empty = bar_width - filled;

                let progress_bar = if status.status == DroneState::Completed {
                    // Completed: dim bar
                    format!("{}",  "━".repeat(bar_width))
                } else {
                    format!("{}{}", "━".repeat(filled), "─".repeat(empty))
                };

                let bar_color = match status.status {
                    DroneState::Completed => Color::DarkGray,
                    DroneState::Blocked | DroneState::Error => Color::Rgb(255, 165, 0),
                    _ => Color::Green,
                };

                // Expand/collapse indicator
                let expand_indicator = if status.status != DroneState::Completed {
                    if is_expanded { "▼" } else { "▶" }
                } else {
                    " "
                };

                // Selection indicator
                let select_char = if is_selected { "▸" } else { " " };

                // Elapsed time
                let elapsed = elapsed_since(&status.started).unwrap_or_else(|| "".to_string());

                // Drone header line
                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if status.status == DroneState::Completed {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Yellow)
                };

                let header_line = Line::from(vec![
                    Span::raw(format!(" {} ", select_char)),
                    Span::styled(icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(expand_indicator, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(format!("🐝 {:<16}", name), name_style),
                    Span::styled(progress_bar, Style::default().fg(bar_color)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:>3}/{:<3}", status.completed.len(), status.total),
                        Style::default().fg(if status.status == DroneState::Completed {
                            Color::DarkGray
                        } else {
                            Color::White
                        }),
                    ),
                    Span::styled(
                        format!(" {:>3}%", percentage),
                        Style::default().fg(if status.status == DroneState::Completed {
                            Color::DarkGray
                        } else {
                            Color::White
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled(elapsed, Style::default().fg(Color::DarkGray)),
                ]);
                lines.push(header_line);

                // Show blocked reason inline
                if status.status == DroneState::Blocked {
                    if let Some(ref reason) = status.blocked_reason {
                        let reason_short = if reason.len() > 60 {
                            format!("{}...", &reason[..57])
                        } else {
                            reason.clone()
                        };
                        lines.push(Line::from(vec![
                            Span::raw("         "),
                            Span::styled("⚠ ", Style::default().fg(Color::Rgb(255, 165, 0))),
                            Span::styled(reason_short, Style::default().fg(Color::Rgb(255, 165, 0))),
                        ]));
                    }
                }

                // Expanded: show stories
                if is_expanded && status.status != DroneState::Completed {
                    if let Some(prd) = prd_cache.get(&status.prd) {
                        for story in &prd.stories {
                            let is_completed = status.completed.contains(&story.id);
                            let is_current = status.current_story.as_ref() == Some(&story.id);

                            let (story_icon, story_color) = if is_completed {
                                ("✓", Color::Green)
                            } else if is_current {
                                ("▸", Color::Yellow)
                            } else {
                                ("○", Color::DarkGray)
                            };

                            // Duration
                            let duration_str =
                                if let Some(timing) = status.story_times.get(&story.id) {
                                    if let (Some(started), Some(completed)) =
                                        (&timing.started, &timing.completed)
                                    {
                                        if let Some(dur) = duration_between(started, completed) {
                                            format!(" {}", format_duration(dur))
                                        } else {
                                            String::new()
                                        }
                                    } else if let Some(started) = &timing.started {
                                        if let Some(elapsed) = elapsed_since(started) {
                                            format!(" {}", elapsed)
                                        } else {
                                            String::new()
                                        }
                                    } else {
                                        String::new()
                                    }
                                } else {
                                    String::new()
                                };

                            let title_short = if story.title.len() > 40 {
                                format!("{}...", &story.title[..37])
                            } else {
                                story.title.clone()
                            };

                            lines.push(Line::from(vec![
                                Span::raw("           "),
                                Span::styled(story_icon, Style::default().fg(story_color)),
                                Span::raw(" "),
                                Span::styled(
                                    format!("{:<10}", story.id),
                                    Style::default().fg(story_color),
                                ),
                                Span::styled(title_short, Style::default().fg(story_color)),
                                Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
                            ]));
                        }
                    }
                }

                // Add spacing between drones
                lines.push(Line::raw(""));
            }

            // Calculate visible area and scroll
            let content_height = chunks[1].height as usize;
            let total_lines = lines.len();

            // Ensure selected drone is visible
            if !drone_line_indices.is_empty() && selected_index < drone_line_indices.len() {
                let selected_line = drone_line_indices[selected_index];
                if selected_line < scroll_offset {
                    scroll_offset = selected_line;
                } else if selected_line >= scroll_offset + content_height.saturating_sub(2) {
                    scroll_offset = selected_line.saturating_sub(content_height.saturating_sub(3));
                }
            }

            // Render visible lines
            let visible_lines: Vec<Line> = lines
                .into_iter()
                .skip(scroll_offset)
                .take(content_height)
                .collect();

            let content = Paragraph::new(visible_lines);
            f.render_widget(content, chunks[1]);

            // Scrollbar if needed
            if total_lines > content_height {
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");

                let mut scrollbar_state = ScrollbarState::new(total_lines)
                    .position(scroll_offset)
                    .viewport_content_length(content_height);

                let scrollbar_area = Rect {
                    x: chunks[1].x + chunks[1].width - 1,
                    y: chunks[1].y,
                    width: 1,
                    height: chunks[1].height,
                };
                f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
            }

            // Footer - shortcuts
            let footer_text = if let Some(msg) = &message {
                msg.clone()
            } else {
                " n new  l logs  x stop  c clean  u unblock  ↵ expand  q quit".to_string()
            };

            let footer = Paragraph::new(Line::from(vec![Span::styled(
                footer_text,
                Style::default().fg(if message.is_some() {
                    message_color
                } else {
                    Color::DarkGray
                }),
            )]));
            f.render_widget(footer, chunks[2]);
        })?;

        // Clear message after displaying
        if message.is_some() {
            message = None;
        }

        // Handle input
        if event::poll(std::time::Duration::from_secs(1))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                    KeyCode::Char('j') | KeyCode::Down => {
                        if !drones.is_empty() && selected_index < drones.len() - 1 {
                            selected_index += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        selected_index = selected_index.saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        // Toggle expand/collapse
                        if !drones.is_empty() {
                            let drone_name = &drones[selected_index].0;
                            if expanded_drones.contains(drone_name) {
                                expanded_drones.remove(drone_name);
                            } else {
                                expanded_drones.insert(drone_name.clone());
                            }
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        match handle_new_drone(&mut terminal) {
                            Ok(Some(msg)) => {
                                message = Some(msg);
                                message_color = Color::Green;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                message = Some(format!("Error: {}", e));
                                message_color = Color::Red;
                            }
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => {
                        if !drones.is_empty() {
                            let drone_name = &drones[selected_index].0;
                            message = Some(format!("Use: hive logs {}", drone_name));
                            message_color = Color::Yellow;
                        }
                    }
                    KeyCode::Char('x') | KeyCode::Char('X') => {
                        if !drones.is_empty() {
                            let drone_name = drones[selected_index].0.clone();
                            match handle_stop_drone(&drone_name) {
                                Ok(msg) => {
                                    message = Some(msg);
                                    message_color = Color::Green;
                                }
                                Err(e) => {
                                    message = Some(format!("Error: {}", e));
                                    message_color = Color::Red;
                                }
                            }
                        }
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        if !drones.is_empty() {
                            let drone_name = drones[selected_index].0.clone();
                            match handle_clean_drone(&mut terminal, &drone_name) {
                                Ok(msg) => {
                                    message = Some(msg);
                                    message_color = Color::Green;
                                }
                                Err(e) => {
                                    message = Some(format!("Error: {}", e));
                                    message_color = Color::Red;
                                }
                            }
                        }
                    }
                    KeyCode::Char('u') | KeyCode::Char('U') => {
                        if !drones.is_empty() {
                            let drone_name = &drones[selected_index].0;
                            let status = &drones[selected_index].1;
                            if status.status == DroneState::Blocked {
                                message = Some(format!("Use: hive unblock {}", drone_name));
                                message_color = Color::Yellow;
                            } else {
                                message = Some(format!("Drone {} is not blocked", drone_name));
                                message_color = Color::Yellow;
                            }
                        }
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        if !drones.is_empty() {
                            let drone_name = &drones[selected_index].0;
                            message = Some(format!("Use: hive sessions {}", drone_name));
                            message_color = Color::Yellow;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

// Handler for 'New Drone' action - browse PRDs and launch
fn handle_new_drone<B: ratatui::backend::Backend>(
    _terminal: &mut ratatui::Terminal<B>,
) -> Result<Option<String>> {
    use dialoguer::{theme::ColorfulTheme, Input, Select};
    use std::io;

    // Find all PRD files
    let prds = find_prd_files()?;

    if prds.is_empty() {
        return Ok(Some(
            "No PRD files found in .hive/prds/ or project root".to_string(),
        ));
    }

    // Disable raw mode temporarily for dialoguer
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

    let result = (|| -> Result<Option<String>> {
        // Let user select PRD
        let prd_names: Vec<String> = prds.iter().map(|p| p.display().to_string()).collect();
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select PRD")
            .items(&prd_names)
            .default(0)
            .interact_opt()?;

        let prd_path = match selection {
            Some(idx) => &prds[idx],
            None => return Ok(None), // User cancelled
        };

        // Read PRD to get default name
        let prd_contents = fs::read_to_string(prd_path)?;
        let prd: Prd = serde_json::from_str(&prd_contents)?;
        let default_name = prd.id.clone();

        // Prompt for drone name
        let drone_name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Drone name")
            .default(default_name)
            .interact_text()?;

        // Prompt for model
        let models = vec!["sonnet", "opus", "haiku"];
        let model_idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select model")
            .items(&models)
            .default(0)
            .interact()?;
        let model = models[model_idx].to_string();

        // Launch drone using start command
        crate::commands::start::run(drone_name.clone(), None, false, false, model, false)?;

        Ok(Some(format!("🐝 Launched drone: {}", drone_name)))
    })();

    // Re-enable raw mode
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    result
}

// Find all PRD files in .hive/prds/ and project root
fn find_prd_files() -> Result<Vec<PathBuf>> {
    let mut prds = Vec::new();

    // Search in .hive/prds/
    let hive_prds = PathBuf::from(".hive").join("prds");
    if hive_prds.exists() {
        for entry in fs::read_dir(&hive_prds)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                prds.push(path);
            }
        }
    }

    // Search in project root for prd*.json
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if name.starts_with("prd") && path.extension().and_then(|s| s.to_str()) == Some("json")
            {
                prds.push(path);
            }
        }
    }

    Ok(prds)
}

// Handler for 'Stop' action (uses quiet mode to avoid corrupting TUI)
fn handle_stop_drone(drone_name: &str) -> Result<String> {
    crate::commands::kill_clean::kill_quiet(drone_name.to_string())?;
    Ok(format!("🛑 Stopped drone: {}", drone_name))
}

// Handler for 'Clean' action with confirmation
fn handle_clean_drone<B: ratatui::backend::Backend>(
    _terminal: &mut ratatui::Terminal<B>,
    drone_name: &str,
) -> Result<String> {
    use dialoguer::{theme::ColorfulTheme, Confirm};
    use std::io;

    // Disable raw mode temporarily for dialoguer
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

    let result = (|| -> Result<String> {
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Clean drone '{}' (remove worktree and branch)?",
                drone_name
            ))
            .default(false)
            .interact()?;

        if confirmed {
            crate::commands::kill_clean::clean(drone_name.to_string(), true)?;
            Ok(format!("🧹 Cleaned drone: {}", drone_name))
        } else {
            Ok("Cancelled".to_string())
        }
    })();

    // Re-enable raw mode
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    result
}
