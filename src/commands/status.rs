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
            println!("\nRun 'hive init' to initialize Hive");
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
    // ◐ = half-full (in progress), ● = full (completed), ○ = empty (pending)
    let status_symbol = match status.status {
        DroneState::Starting => "◐".yellow(),
        DroneState::Resuming => "◐".yellow(),
        DroneState::InProgress => {
            if process_running {
                "◐".green() // Half-full green = in progress
            } else {
                "○".yellow() // Empty yellow = stalled
            }
        }
        DroneState::Completed => "●".bright_green().bold(), // Full green = completed
        DroneState::Error => "◐".red().bold(),              // Half-full red = error
        DroneState::Blocked => "◐".red().bold(),            // Half-full red = blocked
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

            // ◐ = half-full (in progress), ● = full (completed), ○ = empty (pending)
            let (icon, color_fn): (_, fn(String) -> colored::ColoredString) = if is_completed {
                ("●", |s| s.green()) // Full green = completed
            } else if is_current {
                ("◐", |s| s.yellow()) // Half-full yellow = in progress
            } else {
                ("○", |s| s.bright_black()) // Empty = pending
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
    let mut selected_story_index: Option<usize> = None; // None = drone selected, Some(i) = story i selected
    let mut message: Option<String> = None;
    let mut message_color = Color::Green;
    let mut scroll_offset: usize = 0;
    let mut blocked_view: Option<String> = None; // Some(drone_name) = showing blocked detail view

    // Pre-expand in_progress and blocked drones by default
    let initial_drones = list_drones()?;
    let mut expanded_drones: HashSet<String> = initial_drones
        .iter()
        .filter(|(_, status)| {
            matches!(
                status.status,
                DroneState::InProgress
                    | DroneState::Starting
                    | DroneState::Resuming
                    | DroneState::Blocked
                    | DroneState::Error
            )
        })
        .map(|(name, _)| name.clone())
        .collect();

    // Track drones that have been auto-resumed to avoid duplicate resumes
    let mut auto_resumed_drones: HashSet<String> = HashSet::new();

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

        // Auto-resume drones with new stories (only once per drone)
        for (name, status) in &drones {
            if auto_resumed_drones.contains(name) {
                continue;
            }

            // Check if drone has new stories
            let prd_story_count = prd_cache
                .get(&status.prd)
                .map(|p| p.stories.len())
                .unwrap_or(status.total);

            if prd_story_count > status.total {
                // Check if drone is not running
                let process_running = read_drone_pid(name)
                    .map(is_process_running)
                    .unwrap_or(false);

                if !process_running
                    && matches!(
                        status.status,
                        DroneState::Completed | DroneState::Stopped | DroneState::InProgress
                    )
                {
                    let new_count = prd_story_count - status.total;
                    message = Some(format!(
                        "🔄 Auto-resuming '{}' ({} new stor{})",
                        name,
                        new_count,
                        if new_count == 1 { "y" } else { "ies" }
                    ));
                    message_color = Color::Cyan;
                    auto_resumed_drones.insert(name.clone());

                    // Resume the drone
                    if let Err(e) = handle_resume_drone(name) {
                        message = Some(format!("❌ Failed to resume: {}", e));
                        message_color = Color::Red;
                    }
                }
            }
        }

        terminal.draw(|f| {
            let area = f.area();

            // Check if we're showing the blocked detail view
            if let Some(ref blocked_drone_name) = blocked_view {
                // Find the blocked drone
                if let Some((_, status)) =
                    drones.iter().find(|(name, _)| name == blocked_drone_name)
                {
                    render_blocked_detail_view(f, area, blocked_drone_name, status, &prd_cache);
                    return;
                }
            }

            // Main layout: header, content, footer
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4), // Header with ASCII art
                    Constraint::Min(0),    // Content
                    Constraint::Length(1), // Footer
                ])
                .split(area);

            // Header with ASCII art
            let header_lines = vec![
                Line::from(vec![
                    Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "  Orchestrate Claude Code",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("  v{}", env!("CARGO_PKG_VERSION")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  ╩ ╩╩ ╚╝ ╚═╝", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!(
                            "  {} drone{}",
                            drones.len(),
                            if drones.len() != 1 { "s" } else { "" }
                        ),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
            ];
            f.render_widget(Paragraph::new(header_lines), chunks[0]);

            // Build content lines
            let mut lines: Vec<Line> = Vec::new();
            let mut drone_line_indices: Vec<usize> = Vec::new(); // Track which line each drone header is on

            // Show placeholder when no drones
            if drones.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![Span::styled(
                    "  No drones running",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Get started:", Style::default().fg(Color::Yellow)),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled("1. ", Style::default().fg(Color::Cyan)),
                    Span::styled("Create a PRD with ", Style::default().fg(Color::White)),
                    Span::styled(
                        "/hive:prd",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" in Claude Code", Style::default().fg(Color::White)),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled("2. ", Style::default().fg(Color::Cyan)),
                    Span::styled("Launch a drone with ", Style::default().fg(Color::White)),
                    Span::styled(
                        "hive start <name>",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled("3. ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        "Monitor progress here with ",
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        "hive monitor",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Press 'n' to create a new drone",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }

            for (idx, (name, status)) in drones.iter().enumerate() {
                drone_line_indices.push(lines.len());

                let is_selected = idx == selected_index;
                let is_expanded = expanded_drones.contains(name);
                let process_running = read_drone_pid(name)
                    .map(is_process_running)
                    .unwrap_or(false);

                // Status icon and color
                // ◐ = half-full (in progress), ● = full (completed), ○ = empty (pending)
                // Drone is "active" if process is running OR if there's a current story being worked on
                let is_active = process_running || status.current_story.is_some();
                let (icon, status_color) = match status.status {
                    DroneState::Starting | DroneState::Resuming => ("◐", Color::Yellow),
                    DroneState::InProgress => {
                        if is_active {
                            ("◐", Color::Green) // Half-full green = in progress
                        } else {
                            ("○", Color::Yellow) // Empty yellow = stalled
                        }
                    }
                    DroneState::Completed => ("●", Color::Green), // Full green = completed
                    DroneState::Error => ("◐", Color::Red),       // Half-full red = error
                    DroneState::Blocked => ("◐", Color::Red),     // Half-full red = blocked
                    DroneState::Stopped => ("○", Color::DarkGray),
                };

                // Use PRD story count as source of truth (may differ from status.total if PRD was updated)
                let prd_story_count = prd_cache
                    .get(&status.prd)
                    .map(|p| p.stories.len())
                    .unwrap_or(status.total);
                let has_new_stories = prd_story_count > status.total;

                let percentage = if prd_story_count > 0 {
                    (status.completed.len() as f32 / prd_story_count as f32 * 100.0) as u16
                } else {
                    0
                };

                // Build progress bar (20 chars wide)
                let bar_width = 20;
                let filled = (bar_width as f32 * percentage as f32 / 100.0) as usize;
                let empty = bar_width - filled;

                let (filled_bar, empty_bar) =
                    if status.status == DroneState::Completed && !has_new_stories {
                        // Completed: full green bar
                        ("━".repeat(bar_width), String::new())
                    } else {
                        ("━".repeat(filled), "─".repeat(empty))
                    };

                let filled_color = match status.status {
                    DroneState::Completed => Color::Green, // Full green when completed
                    DroneState::Blocked | DroneState::Error => Color::Rgb(255, 165, 0),
                    _ => Color::Green,
                };

                // Expand/collapse indicator (all drones can be expanded)
                let expand_indicator = if is_expanded { "▼" } else { "▶" };

                // Selection indicator
                let select_char = if is_selected { "▸" } else { " " };

                // Elapsed time - stop timer if completed
                let elapsed = if status.status == DroneState::Completed {
                    // Find the last completed story time
                    let last_completed = status
                        .story_times
                        .values()
                        .filter_map(|t| t.completed.as_ref())
                        .max();
                    if let (Some(last), Some(start)) =
                        (last_completed, parse_timestamp(&status.started))
                    {
                        if let Some(end) = parse_timestamp(last) {
                            format_duration(end.signed_duration_since(start))
                        } else {
                            elapsed_since(&status.started).unwrap_or_default()
                        }
                    } else {
                        elapsed_since(&status.started).unwrap_or_default()
                    }
                } else if status.status == DroneState::Stopped {
                    // Stopped - show time at stop (use updated timestamp)
                    if let Some(duration) = duration_between(&status.started, &status.updated) {
                        format_duration(duration)
                    } else {
                        elapsed_since(&status.started).unwrap_or_default()
                    }
                } else {
                    // In progress - show live elapsed time
                    elapsed_since(&status.started).unwrap_or_default()
                };

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
                    Span::styled(format!("🐝 {:<30} ", name), name_style),
                    Span::styled(filled_bar, Style::default().fg(filled_color)),
                    Span::styled(empty_bar, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:>3}/{:<3}", status.completed.len(), prd_story_count),
                        Style::default().fg(
                            if status.status == DroneState::Completed && !has_new_stories {
                                Color::DarkGray
                            } else if has_new_stories {
                                Color::Cyan // Highlight when new stories available
                            } else {
                                Color::White
                            },
                        ),
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

                // Expanded: show stories (works for all drones including completed)
                if is_expanded {
                    if let Some(prd) = prd_cache.get(&status.prd) {
                        for (story_idx, story) in prd.stories.iter().enumerate() {
                            let is_completed = status.completed.contains(&story.id);
                            let is_current = status.current_story.as_ref() == Some(&story.id);
                            let is_story_selected =
                                is_selected && selected_story_index == Some(story_idx);

                            // ◐ = half-full (in progress), ● = full (completed), ○ = empty (pending)
                            let (story_icon, story_color) = if is_story_selected {
                                ("▸", Color::Cyan)
                            } else if is_completed {
                                ("●", Color::Green) // Full green = completed
                            } else if is_current {
                                ("◐", Color::Yellow) // Half-full yellow = in progress
                            } else {
                                ("○", Color::DarkGray) // Empty = pending
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

                            let line_style = if is_story_selected {
                                Style::default().add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            };

                            lines.push(Line::from(vec![
                                Span::styled("      ", line_style),
                                Span::styled(story_icon, line_style.fg(story_color)),
                                Span::raw(" "),
                                Span::styled(
                                    format!("{:<16} ", story.id),
                                    line_style.fg(if is_story_selected {
                                        Color::Cyan
                                    } else {
                                        story_color
                                    }),
                                ),
                                Span::styled(
                                    title_short,
                                    line_style.fg(if is_story_selected {
                                        Color::Cyan
                                    } else {
                                        story_color
                                    }),
                                ),
                                Span::styled(duration_str, line_style.fg(Color::DarkGray)),
                            ]));
                        }
                    }

                    // Show blocked indicator (press 'b' for details)
                    if status.status == DroneState::Blocked {
                        let orange = Color::Rgb(255, 165, 0);
                        lines.push(Line::from(vec![
                            Span::raw("      "),
                            Span::styled(
                                "⚠ BLOCKED",
                                Style::default().fg(orange).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                " - press 'b' for details",
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    }

                    // Show new stories indicator (auto-resume pending)
                    if has_new_stories {
                        let new_count = prd_story_count - status.total;
                        lines.push(Line::from(vec![
                            Span::raw("      "),
                            Span::styled(
                                format!(
                                    "✨ {} new stor{}",
                                    new_count,
                                    if new_count == 1 { "y" } else { "ies" }
                                ),
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                " - auto-resuming...",
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    }
                }

                // Add separator between drones
                lines.push(Line::styled(
                    "  ─────────────────────────────────────────────────────",
                    Style::default().fg(Color::DarkGray),
                ));
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

            // Footer - shortcuts (context-dependent)
            let footer_text = if let Some(msg) = &message {
                msg.clone()
            } else if selected_story_index.is_some() {
                " i info  l logs  ↑↓ navigate  ← back  q back".to_string()
            } else {
                " ↵ expand  l logs  b blocked  x stop  c clean  q quit".to_string()
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

        // Handle input (including resize events)
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Resize(_, _) => {
                    // Terminal resized, just continue to redraw
                    continue;
                }
                Event::Key(key) => {
                    // Get story count for current drone if expanded
                    let current_story_count = if !drones.is_empty() {
                        let drone_name = &drones[selected_index].0;
                        let status = &drones[selected_index].1;
                        if expanded_drones.contains(drone_name) {
                            prd_cache
                                .get(&status.prd)
                                .map(|p| p.stories.len())
                                .unwrap_or(0)
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            // If in blocked view, go back to main view
                            if blocked_view.is_some() {
                                blocked_view = None;
                            } else if selected_story_index.is_some() {
                                // If story selected, go back to drone
                                selected_story_index = None;
                            } else {
                                // Quit
                                break;
                            }
                        }
                        KeyCode::Char('b') | KeyCode::Char('B') => {
                            // Open blocked detail view for current drone if it's blocked
                            if !drones.is_empty() {
                                let drone_name = &drones[selected_index].0;
                                let status = &drones[selected_index].1;
                                if status.status == DroneState::Blocked {
                                    blocked_view = Some(drone_name.clone());
                                } else {
                                    message = Some("Drone is not blocked".to_string());
                                    message_color = Color::Yellow;
                                }
                            }
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            if !drones.is_empty() {
                                if let Some(story_idx) = selected_story_index {
                                    // Navigate within stories
                                    if story_idx < current_story_count.saturating_sub(1) {
                                        selected_story_index = Some(story_idx + 1);
                                    }
                                } else if selected_index < drones.len() - 1 {
                                    // Navigate between drones
                                    selected_index += 1;
                                    selected_story_index = None;
                                }
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if let Some(story_idx) = selected_story_index {
                                // Navigate within stories
                                if story_idx > 0 {
                                    selected_story_index = Some(story_idx - 1);
                                } else {
                                    // Go back to drone header
                                    selected_story_index = None;
                                }
                            } else {
                                // Navigate between drones
                                selected_index = selected_index.saturating_sub(1);
                            }
                        }
                        KeyCode::Enter => {
                            if !drones.is_empty() {
                                let drone_name = &drones[selected_index].0;
                                if selected_story_index.is_some() {
                                    // Enter on story = show story details
                                    if let Some(story_idx) = selected_story_index {
                                        let status = &drones[selected_index].1;
                                        if let Some(prd) = prd_cache.get(&status.prd) {
                                            if let Some(story) = prd.stories.get(story_idx) {
                                                message =
                                                    Some(format!("{}: {}", story.id, story.title));
                                                message_color = Color::Cyan;
                                            }
                                        }
                                    }
                                } else if expanded_drones.contains(drone_name) {
                                    // Collapse or enter story navigation
                                    if current_story_count > 0 {
                                        // Enter story navigation mode
                                        selected_story_index = Some(0);
                                    } else {
                                        // Collapse
                                        expanded_drones.remove(drone_name);
                                    }
                                } else {
                                    // Expand
                                    expanded_drones.insert(drone_name.clone());
                                }
                            }
                        }
                        KeyCode::Left => {
                            // Collapse current drone
                            if !drones.is_empty() {
                                let drone_name = &drones[selected_index].0;
                                expanded_drones.remove(drone_name);
                                selected_story_index = None;
                            }
                        }
                        KeyCode::Right => {
                            // Expand current drone or enter stories
                            if !drones.is_empty() {
                                let drone_name = &drones[selected_index].0;
                                if !expanded_drones.contains(drone_name) {
                                    expanded_drones.insert(drone_name.clone());
                                } else if current_story_count > 0 && selected_story_index.is_none()
                                {
                                    selected_story_index = Some(0);
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
                            // Open logs viewer
                            if let Some(ref drone_name) = blocked_view {
                                // In blocked view - open logs for this drone
                                match show_logs_viewer(&mut terminal, drone_name) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        message = Some(format!("Error: {}", e));
                                        message_color = Color::Red;
                                    }
                                }
                            } else if !drones.is_empty() {
                                let drone_name = &drones[selected_index].0;
                                let status = &drones[selected_index].1;
                                if let Some(story_idx) = selected_story_index {
                                    // Show logs for specific story
                                    if let Some(prd) = prd_cache.get(&status.prd) {
                                        if let Some(story) = prd.stories.get(story_idx) {
                                            message = Some(format!(
                                                "Use: hive logs {} --story {}",
                                                drone_name, story.id
                                            ));
                                            message_color = Color::Yellow;
                                        }
                                    }
                                } else {
                                    // Open logs viewer for selected drone
                                    match show_logs_viewer(&mut terminal, drone_name) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            message = Some(format!("Error: {}", e));
                                            message_color = Color::Red;
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('i') | KeyCode::Char('I') => {
                            // Show story details
                            if !drones.is_empty() {
                                if let Some(story_idx) = selected_story_index {
                                    let status = &drones[selected_index].1;
                                    if let Some(prd) = prd_cache.get(&status.prd) {
                                        if let Some(story) = prd.stories.get(story_idx) {
                                            // Show full story info
                                            let desc = if story.description.is_empty() {
                                                "No description".to_string()
                                            } else if story.description.len() > 80 {
                                                format!("{}...", &story.description[..77])
                                            } else {
                                                story.description.clone()
                                            };
                                            message = Some(format!("[{}] {}", story.id, desc));
                                            message_color = Color::Cyan;
                                        }
                                    }
                                } else {
                                    message = Some(
                                        "Select a story first (↵ to enter stories)".to_string(),
                                    );
                                    message_color = Color::Yellow;
                                }
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
                                match handle_clean_drone(&drone_name) {
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
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            // Resume drone (especially useful when new stories added to PRD)
                            if !drones.is_empty() {
                                let drone_name = drones[selected_index].0.clone();
                                let status = &drones[selected_index].1;
                                let prd_story_count = prd_cache
                                    .get(&status.prd)
                                    .map(|p| p.stories.len())
                                    .unwrap_or(status.total);
                                let has_new_stories = prd_story_count > status.total;

                                if has_new_stories
                                    || status.status == DroneState::Completed
                                    || status.status == DroneState::Stopped
                                {
                                    match handle_resume_drone(&drone_name) {
                                        Ok(msg) => {
                                            message = Some(msg);
                                            message_color = Color::Green;
                                        }
                                        Err(e) => {
                                            message = Some(format!("Error: {}", e));
                                            message_color = Color::Red;
                                        }
                                    }
                                } else {
                                    message =
                                        Some(format!("Drone {} is already running", drone_name));
                                    message_color = Color::Yellow;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
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

// Handler for 'Clean' action - cleans in background, disappears from list immediately
fn handle_clean_drone(drone_name: &str) -> Result<String> {
    crate::commands::kill_clean::clean_background(drone_name.to_string());
    Ok(format!("🧹 Cleaning drone: {}", drone_name))
}

// Handler for 'Resume' action - resumes a drone with new stories or stopped drone
fn handle_resume_drone(drone_name: &str) -> Result<String> {
    // Update status.json to reflect new PRD story count
    let status_path = PathBuf::from(".hive")
        .join("drones")
        .join(drone_name)
        .join("status.json");
    let prd_path_dir = PathBuf::from(".hive").join("prds");

    if let Ok(status_content) = fs::read_to_string(&status_path) {
        if let Ok(mut status) = serde_json::from_str::<DroneStatus>(&status_content) {
            // Find and load PRD to get new story count
            let prd_path = prd_path_dir.join(&status.prd);
            if let Some(prd) = load_prd(&prd_path) {
                // Update total to match PRD
                status.total = prd.stories.len();
                // Reset status to in_progress
                status.status = DroneState::InProgress;
                status.updated = chrono::Utc::now().to_rfc3339();

                // Write updated status
                if let Ok(updated_json) = serde_json::to_string_pretty(&status) {
                    let _ = fs::write(&status_path, updated_json);
                }
            }
        }
    }

    // Launch drone with resume flag
    crate::commands::start::run(
        drone_name.to_string(),
        None,
        true,
        false,
        "sonnet".to_string(),
        false,
    )?;
    Ok(format!("🔄 Resumed drone: {}", drone_name))
}

// Render the blocked detail view
fn render_blocked_detail_view(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    drone_name: &str,
    status: &DroneStatus,
    prd_cache: &std::collections::HashMap<String, Prd>,
) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::Paragraph,
    };

    let orange = Color::Rgb(255, 165, 0);

    // Layout: header (4) + subheader (2) + content + footer (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header with ASCII art
            Constraint::Length(2), // Subheader with drone info
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Header with ASCII art (same as main view)
    let header_lines = vec![
        Line::from(vec![
            Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
            Span::styled(
                "  Orchestrate Claude Code",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("  v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ╩ ╩╩ ╚╝ ╚═╝", Style::default().fg(Color::Yellow)),
            Span::styled(
                "  BLOCKED DRONE",
                Style::default().fg(orange).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(header_lines), chunks[0]);

    // Subheader: drone name + blocked story
    let blocked_story = status.current_story.as_deref().unwrap_or("Unknown");
    let story_title = prd_cache
        .get(&status.prd)
        .and_then(|prd| prd.stories.iter().find(|s| s.id == blocked_story))
        .map(|s| s.title.as_str())
        .unwrap_or("");

    let subheader_lines = vec![
        Line::from(vec![
            Span::styled("  ⚠ ", Style::default().fg(orange)),
            Span::styled(
                format!("🐝 {}", drone_name),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                blocked_story,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", story_title),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::styled(
            "  ─────────────────────────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        ),
    ];
    f.render_widget(Paragraph::new(subheader_lines), chunks[1]);

    // Content: blocked reason + questions
    let mut content_lines: Vec<Line> = Vec::new();
    content_lines.push(Line::raw(""));

    // Blocked reason
    if let Some(ref reason) = status.blocked_reason {
        content_lines.push(Line::from(vec![Span::styled(
            "  REASON",
            Style::default().fg(orange).add_modifier(Modifier::BOLD),
        )]));
        content_lines.push(Line::raw(""));

        // Word-wrap the reason text
        let max_width = (area.width as usize).saturating_sub(6).min(80);
        let wrapped = wrap_text(reason, max_width);
        for line in wrapped {
            content_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(line, Style::default().fg(Color::White)),
            ]));
        }
    }

    // Questions
    if !status.blocked_questions.is_empty() {
        content_lines.push(Line::raw(""));
        content_lines.push(Line::from(vec![Span::styled(
            "  QUESTIONS",
            Style::default().fg(orange).add_modifier(Modifier::BOLD),
        )]));
        content_lines.push(Line::raw(""));

        let max_width = (area.width as usize).saturating_sub(8).min(78);
        for (i, question) in status.blocked_questions.iter().enumerate() {
            content_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::Yellow)),
            ]));

            let wrapped = wrap_text(question, max_width);
            for (j, line) in wrapped.iter().enumerate() {
                if j == 0 {
                    // First line: just append to the number
                    if let Some(last_line) = content_lines.last_mut() {
                        last_line.spans.push(Span::styled(
                            line.clone(),
                            Style::default().fg(Color::White),
                        ));
                    }
                } else {
                    // Continuation lines: indent
                    content_lines.push(Line::from(vec![
                        Span::raw("     "),
                        Span::styled(line.clone(), Style::default().fg(Color::White)),
                    ]));
                }
            }
            content_lines.push(Line::raw(""));
        }
    }

    // Show last errors from log if any
    if status.error_count > 0 {
        content_lines.push(Line::raw(""));
        content_lines.push(Line::from(vec![Span::styled(
            format!("  ERRORS ({})", status.error_count),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]));
        content_lines.push(Line::raw(""));
        content_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Press 'l' to view full logs",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(content_lines), chunks[2]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " l logs  q back",
        Style::default().fg(Color::DarkGray),
    )]));
    f.render_widget(footer, chunks[3]);
}

// Word wrap helper function
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// Show logs viewer in TUI with line selection and JSON pretty-print
fn show_logs_viewer<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    drone_name: &str,
) -> Result<()> {
    use crossterm::event::{self, Event, KeyCode};
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    };

    // Read log file
    let log_path = PathBuf::from(".hive")
        .join("drones")
        .join(drone_name)
        .join("activity.log");

    let log_content = fs::read_to_string(&log_path).unwrap_or_else(|_| "No logs found".to_string());

    let log_lines: Vec<String> = log_content.lines().map(|s| s.to_string()).collect();
    let total_lines = log_lines.len();
    let mut selected_line: usize = total_lines.saturating_sub(1); // Start at last line
    let mut scroll_offset: usize = total_lines.saturating_sub(20);
    let mut detail_view: Option<String> = None; // Pretty-printed JSON for detail view
    let mut detail_scroll: usize = 0;

    loop {
        // Reload log file to get updates
        let log_content =
            fs::read_to_string(&log_path).unwrap_or_else(|_| "No logs found".to_string());
        let log_lines: Vec<String> = log_content.lines().map(|s| s.to_string()).collect();
        let total_lines = log_lines.len();

        // Clamp selected line
        if total_lines > 0 && selected_line >= total_lines {
            selected_line = total_lines - 1;
        }

        terminal.draw(|f| {
            let area = f.area();

            // If showing detail view (pretty-printed JSON)
            if let Some(ref detail) = detail_view {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(4), // Header
                        Constraint::Min(0),    // Content
                        Constraint::Length(1), // Footer
                    ])
                    .split(area);

                // Header with HIVE ASCII art
                let header_lines = vec![
                    Line::from(vec![
                        Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
                        Span::styled("  Log Detail", Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(vec![
                        Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("  🐝 {}", drone_name),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  ╩ ╩╩ ╚╝ ╚═╝", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("  Line {}/{}", selected_line + 1, total_lines),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                ];
                f.render_widget(Paragraph::new(header_lines), chunks[0]);

                // Detail content with word wrap
                let content_width = chunks[1].width.saturating_sub(4) as usize;

                // Wrap lines to fit screen width
                let wrapped_lines: Vec<(String, Style)> = detail
                    .lines()
                    .flat_map(|line| {
                        let style = if line.contains("\"error\"") || line.contains("ERROR") {
                            Style::default().fg(Color::Red)
                        } else if line.contains("\"type\"") {
                            Style::default().fg(Color::Cyan)
                        } else if line.contains("\"text\"") || line.contains("\"name\"") {
                            Style::default().fg(Color::Green)
                        } else if line.trim().starts_with("\"") {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::White)
                        };

                        // Wrap long lines
                        if line.len() > content_width {
                            let mut wrapped = Vec::new();
                            let mut remaining = line;
                            while !remaining.is_empty() {
                                let (chunk, rest) = if remaining.len() > content_width {
                                    remaining.split_at(content_width)
                                } else {
                                    (remaining, "")
                                };
                                wrapped.push((chunk.to_string(), style));
                                remaining = rest;
                            }
                            wrapped
                        } else {
                            vec![(line.to_string(), style)]
                        }
                    })
                    .collect();

                let detail_total = wrapped_lines.len();
                let content_height = chunks[1].height as usize;

                let visible_detail: Vec<Line> = wrapped_lines
                    .iter()
                    .skip(detail_scroll)
                    .take(content_height)
                    .map(|(line, style)| Line::from(Span::styled(format!("  {}", line), *style)))
                    .collect();

                f.render_widget(Paragraph::new(visible_detail), chunks[1]);

                // Scrollbar for detail view
                if detail_total > content_height {
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(None)
                        .end_symbol(None)
                        .track_symbol(Some("│"))
                        .thumb_symbol("█");

                    let mut scrollbar_state = ScrollbarState::new(detail_total)
                        .position(detail_scroll)
                        .viewport_content_length(content_height);

                    let scrollbar_area = ratatui::layout::Rect {
                        x: chunks[1].x + chunks[1].width - 1,
                        y: chunks[1].y,
                        width: 1,
                        height: chunks[1].height,
                    };
                    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
                }

                // Footer
                let footer = Paragraph::new(Line::from(vec![Span::styled(
                    " ↑↓ scroll  q/Esc back to list",
                    Style::default().fg(Color::DarkGray),
                )]));
                f.render_widget(footer, chunks[2]);

                return;
            }

            // Main log list view
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4), // Header with ASCII art
                    Constraint::Min(0),    // Content
                    Constraint::Length(1), // Footer
                ])
                .split(area);

            // Header with HIVE ASCII art
            let header_lines = vec![
                Line::from(vec![
                    Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
                    Span::styled("  Activity Logs", Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("  🐝 {}", drone_name),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  ╩ ╩╩ ╚╝ ╚═╝", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("  {} entries", total_lines),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ];
            f.render_widget(Paragraph::new(header_lines), chunks[0]);

            // Content - log lines with selection
            let content_height = chunks[1].height as usize;
            let content_width = chunks[1].width.saturating_sub(4) as usize;

            // Ensure selected line is visible
            if selected_line < scroll_offset {
                scroll_offset = selected_line;
            } else if selected_line >= scroll_offset + content_height {
                scroll_offset = selected_line.saturating_sub(content_height - 1);
            }

            let visible_lines: Vec<Line> = log_lines
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .take(content_height)
                .map(|(idx, line)| {
                    let is_selected = idx == selected_line;

                    // Parse JSON to get summary
                    let summary = parse_log_summary(line, content_width);

                    let base_style = if line.contains("\"error\"") || line.contains("ERROR") {
                        Style::default().fg(Color::Red)
                    } else if line.contains("tool_use") || line.contains("\"name\"") {
                        Style::default().fg(Color::Cyan)
                    } else if line.contains("\"text\"") {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let style = if is_selected {
                        base_style.bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else {
                        base_style
                    };

                    let prefix = if is_selected { "▸ " } else { "  " };
                    Line::from(Span::styled(format!("{}{}", prefix, summary), style))
                })
                .collect();

            f.render_widget(Paragraph::new(visible_lines), chunks[1]);

            // Scrollbar
            if total_lines > content_height {
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");

                let mut scrollbar_state = ScrollbarState::new(total_lines)
                    .position(scroll_offset)
                    .viewport_content_length(content_height);

                let scrollbar_area = ratatui::layout::Rect {
                    x: chunks[1].x + chunks[1].width - 1,
                    y: chunks[1].y,
                    width: 1,
                    height: chunks[1].height,
                };
                f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
            }

            // Footer
            let footer = Paragraph::new(Line::from(vec![Span::styled(
                " ↑↓/jk navigate  ↵ expand  G end  g start  q back",
                Style::default().fg(Color::DarkGray),
            )]));
            f.render_widget(footer, chunks[2]);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                let content_height = terminal.size()?.height.saturating_sub(5) as usize;

                if detail_view.is_some() {
                    // Detail view controls
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            detail_view = None;
                            detail_scroll = 0;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            detail_scroll += 1;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            detail_scroll = detail_scroll.saturating_sub(1);
                        }
                        KeyCode::PageDown => {
                            detail_scroll += content_height;
                        }
                        KeyCode::PageUp => {
                            detail_scroll = detail_scroll.saturating_sub(content_height);
                        }
                        KeyCode::Char('g') => {
                            detail_scroll = 0;
                        }
                        KeyCode::Char('G') => {
                            // Go to end - will be clamped in render
                            detail_scroll = usize::MAX / 2;
                        }
                        _ => {}
                    }
                } else {
                    // List view controls
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                        KeyCode::Char('j') | KeyCode::Down => {
                            if total_lines > 0 && selected_line < total_lines - 1 {
                                selected_line += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            selected_line = selected_line.saturating_sub(1);
                        }
                        KeyCode::Char('g') => {
                            selected_line = 0;
                            scroll_offset = 0;
                        }
                        KeyCode::Char('G') => {
                            if total_lines > 0 {
                                selected_line = total_lines - 1;
                                scroll_offset = total_lines.saturating_sub(content_height);
                            }
                        }
                        KeyCode::PageDown => {
                            selected_line =
                                (selected_line + content_height).min(total_lines.saturating_sub(1));
                        }
                        KeyCode::PageUp => {
                            selected_line = selected_line.saturating_sub(content_height);
                        }
                        KeyCode::Enter => {
                            // Pretty-print selected line
                            if total_lines > 0 && selected_line < log_lines.len() {
                                let line = &log_lines[selected_line];
                                detail_view = Some(pretty_print_json(line));
                                detail_scroll = 0;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

// Parse log line and return a summary for display
fn parse_log_summary(line: &str, max_width: usize) -> String {
    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        // Extract useful info from stream-json format
        let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("?");

        let summary = match msg_type {
            "assistant" => {
                if let Some(content) = json
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_array())
                {
                    if let Some(first) = content.first() {
                        if let Some(text) = first.get("text").and_then(|t| t.as_str()) {
                            let short = text.chars().take(80).collect::<String>();
                            format!("💬 {}", short.replace('\n', " "))
                        } else if let Some(name) = first.get("name").and_then(|n| n.as_str()) {
                            // Get tool input for more context
                            let context = if let Some(input) = first.get("input") {
                                if let Some(file) = input.get("file_path").and_then(|f| f.as_str())
                                {
                                    // Extract just filename
                                    file.rsplit('/').next().unwrap_or(file).to_string()
                                } else if let Some(cmd) =
                                    input.get("command").and_then(|c| c.as_str())
                                {
                                    cmd.chars().take(40).collect::<String>()
                                } else if let Some(pattern) =
                                    input.get("pattern").and_then(|p| p.as_str())
                                {
                                    format!("/{}/", pattern.chars().take(30).collect::<String>())
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            if context.is_empty() {
                                format!("🔧 {}", name)
                            } else {
                                format!("🔧 {} → {}", name, context)
                            }
                        } else {
                            "💬 assistant".to_string()
                        }
                    } else {
                        "💬 assistant".to_string()
                    }
                } else {
                    "💬 assistant".to_string()
                }
            }
            "user" => {
                // User messages are typically tool results
                if let Some(content) = json
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_array())
                {
                    if let Some(first) = content.first() {
                        let tool_type = first.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if tool_type == "tool_result" {
                            // Check tool_use_result for details
                            if let Some(result) = json.get("tool_use_result") {
                                // Edit result
                                if let Some(file) = result.get("filePath").and_then(|f| f.as_str())
                                {
                                    let filename = file.rsplit('/').next().unwrap_or(file);
                                    return truncate_summary(
                                        &format!("✓ Edit → {}", filename),
                                        max_width,
                                    );
                                }
                                // Bash result
                                if let Some(stdout) = result.get("stdout").and_then(|s| s.as_str())
                                {
                                    let short = stdout
                                        .lines()
                                        .next()
                                        .unwrap_or("")
                                        .chars()
                                        .take(50)
                                        .collect::<String>();
                                    return truncate_summary(
                                        &format!("✓ Bash → {}", short),
                                        max_width,
                                    );
                                }
                                // Read result
                                if result.get("content").is_some() {
                                    if let Some(file) =
                                        result.get("filePath").and_then(|f| f.as_str())
                                    {
                                        let filename = file.rsplit('/').next().unwrap_or(file);
                                        return truncate_summary(
                                            &format!("✓ Read → {}", filename),
                                            max_width,
                                        );
                                    }
                                }
                                // Glob/Grep result
                                if let Some(files) = result.get("files").and_then(|f| f.as_array())
                                {
                                    return truncate_summary(
                                        &format!("✓ Found {} files", files.len()),
                                        max_width,
                                    );
                                }
                            }
                            // Fallback: get content text
                            if let Some(content_text) =
                                first.get("content").and_then(|c| c.as_str())
                            {
                                let short = content_text.chars().take(50).collect::<String>();
                                return truncate_summary(
                                    &format!("✓ {}", short.replace('\n', " ")),
                                    max_width,
                                );
                            }
                        }
                    }
                }
                "👤 user".to_string()
            }
            "result" => {
                if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                    let short = result.chars().take(60).collect::<String>();
                    format!("✓ {}", short.replace('\n', " "))
                } else {
                    "✓ result".to_string()
                }
            }
            "system" => {
                let subtype = json.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
                match subtype {
                    "init" => "⚙ Session started".to_string(),
                    _ => format!("⚙ {}", subtype),
                }
            }
            "error" => {
                if let Some(err) = json.get("error").and_then(|e| e.as_str()) {
                    format!("❌ {}", err)
                } else {
                    "❌ error".to_string()
                }
            }
            _ => format!("[{}]", msg_type),
        };

        truncate_summary(&summary, max_width)
    } else {
        // Not JSON, show raw line truncated
        truncate_summary(line, max_width)
    }
}

fn truncate_summary(s: &str, max_width: usize) -> String {
    if s.len() > max_width {
        format!("{}...", &s[..max_width.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

// Pretty-print JSON with indentation and word wrap
fn pretty_print_json(line: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        // Use custom formatting for better readability
        format_json_value(&json, 0)
    } else {
        line.to_string()
    }
}

// Recursively format JSON with proper indentation and no truncation
fn format_json_value(value: &serde_json::Value, indent: usize) -> String {
    let indent_str = "  ".repeat(indent);
    let next_indent = "  ".repeat(indent + 1);

    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            // For long strings, wrap them
            if s.len() > 80 {
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n");
                format!("\"{}\"", escaped)
            } else {
                format!("{:?}", s)
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| format!("{}{}", next_indent, format_json_value(v, indent + 1)))
                    .collect();
                format!("[\n{}\n{}]", items.join(",\n"), indent_str)
            }
        }
        serde_json::Value::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        let formatted_value = format_json_value(v, indent + 1);
                        format!("{}\"{}\": {}", next_indent, k, formatted_value)
                    })
                    .collect();
                format!("{{\n{}\n{}}}", items.join(",\n"), indent_str)
            }
        }
    }
}
