use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use super::common::{
    duration_between, elapsed_since, format_duration, is_process_running, list_drones, load_prd,
    parse_timestamp, read_drone_pid, reconcile_progress, reconcile_progress_with_prd,
    truncate_with_ellipsis, wrap_text, DEFAULT_INACTIVE_THRESHOLD_SECS, FULL_PROGRESS_BAR_WIDTH,
    MAX_DRONE_NAME_LEN, MAX_STORY_TITLE_LEN,
};
use crate::communication::MessageBus;
use crate::types::{DroneState, DroneStatus, ExecutionMode, Prd};

/// Refresh interval for follow mode in seconds
const FOLLOW_REFRESH_SECS: u64 = 30;

/// Event poll timeout in milliseconds for TUI
const TUI_POLL_TIMEOUT_MS: u64 = 100;

/// Log viewer poll timeout in milliseconds
const LOG_POLL_TIMEOUT_MS: u64 = 500;

/// ANSI escape sequence to clear screen and move cursor to top-left
const CLEAR_SCREEN: &str = "\x1B[2J\x1B[1;1H";

/// Number of sparkline data points (8 chars wide)
const SPARKLINE_WIDTH: usize = 8;

/// Duration of each activity bucket in seconds (1 minute)
const ACTIVITY_BUCKET_SECS: u64 = 60;

/// Cost summary parsed from activity logs
#[derive(Debug, Clone, Default)]
struct CostSummary {
    total_cost_usd: f64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_creation_tokens: u64,
}

/// Cache entry for cost data to avoid re-parsing unchanged logs
#[derive(Debug, Clone)]
struct CostCacheEntry {
    summary: CostSummary,
    log_mtime: SystemTime,
}

// Global cache for cost data per drone (using thread_local for simplicity)
thread_local! {
    static COST_CACHE: std::cell::RefCell<HashMap<String, CostCacheEntry>> =
        std::cell::RefCell::new(HashMap::new());
}

/// View mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Dashboard,
    Timeline,
}

/// Run the monitor command with auto-refresh TUI by default, simple mode for scripts/CI.
pub fn run_monitor(name: Option<String>, simple: bool) -> Result<()> {
    if simple {
        run_simple(name, false)
    } else {
        run_tui(name)
    }
}

/// Legacy run function for backward compatibility (can be removed later).
pub fn run(name: Option<String>, interactive: bool, follow: bool) -> Result<()> {
    if interactive {
        run_tui(name)
    } else {
        run_simple(name, follow)
    }
}

fn run_simple(name: Option<String>, follow: bool) -> Result<()> {
    loop {
        if follow {
            print!("{}", CLEAR_SCREEN);
        }

        let drones = list_drones()?;

        if drones.is_empty() {
            println!("{}", "No drones found".yellow());
            println!("\nRun 'hive init' to initialize Hive");
            return Ok(());
        }

        let filtered: Vec<_> = match name {
            Some(ref n) => drones
                .into_iter()
                .filter(|(drone_name, _)| drone_name == n)
                .collect(),
            None => drones,
        };

        if filtered.is_empty() {
            eprintln!("Drone '{}' not found", name.unwrap());
            return Ok(());
        }

        println!(
            "  {} v{}",
            "👑 hive".yellow().bold(),
            env!("CARGO_PKG_VERSION")
        );
        println!();

        let mut sorted = filtered;
        sorted.sort_by_key(|(_, status)| match status.status {
            DroneState::Completed => 1,
            _ => 0,
        });

        for (drone_name, status) in &sorted {
            let collapsed = status.status == DroneState::Completed;
            print_drone_status(drone_name, status, collapsed);
            println!();
        }

        if !follow {
            suggest_cleanup_for_inactive(&sorted);
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(FOLLOW_REFRESH_SECS));
    }

    Ok(())
}

fn suggest_cleanup_for_inactive(drones: &[(String, DroneStatus)]) {
    let threshold_seconds = std::env::var("HIVE_INACTIVE_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(DEFAULT_INACTIVE_THRESHOLD_SECS);

    let now = Utc::now();

    for (name, status) in drones {
        if status.status != DroneState::Completed {
            continue;
        }

        let Some(updated) = parse_timestamp(&status.updated) else {
            continue;
        };

        let inactive_seconds = now.signed_duration_since(updated).num_seconds();

        if inactive_seconds > threshold_seconds {
            let duration = chrono::Duration::seconds(inactive_seconds);
            let duration_str = format_duration(duration);

            println!();
            println!(
                "{} Drone {} completed {} ago. Clean up? {}",
                "💡".bright_yellow(),
                name.bright_cyan(),
                duration_str.bright_black(),
                format!("(hive clean {})", name).bright_black()
            );

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

    // Determine emoji based on execution mode
    let mode_emoji = match status.execution_mode {
        ExecutionMode::Subagent => "🤖",
        ExecutionMode::Worktree => "🐝",
        ExecutionMode::Swarm => "🐝",
    };

    // Reconcile progress with actual PRD (filters out old completed stories)
    let (valid_completed, total_stories) = reconcile_progress(status);

    // If collapsed view (completed drones), show single line
    if collapsed {
        let progress = if total_stories > 0 {
            format!("{}/{}", valid_completed, total_stories)
        } else {
            "0/0".to_string()
        };

        println!(
            "  {} {}{}  {}",
            status_symbol,
            format!("{} {}", mode_emoji, name).bright_black(),
            elapsed.bright_black(),
            progress.bright_black()
        );
        return; // Exit early, don't show full details
    }

    // Full view for active drones
    println!(
        "  {} {}{}  {}",
        status_symbol,
        format!("{} {}", mode_emoji, name).yellow().bold(),
        elapsed.bright_black(),
        format!("[{}]", status.status).bright_black()
    );

    // Print progress using reconciled values
    let progress = if total_stories > 0 {
        format!("{}/{}", valid_completed, total_stories)
    } else {
        "0/0".to_string()
    };

    let percentage = if total_stories > 0 {
        (valid_completed as f32 / total_stories as f32 * 100.0) as u32
    } else {
        0
    };

    println!("  Progress: {} ({}%)", progress.bright_white(), percentage);

    let filled = (FULL_PROGRESS_BAR_WIDTH as f32 * percentage as f32 / 100.0) as usize;
    let empty = FULL_PROGRESS_BAR_WIDTH - filled;
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

            let title_display = truncate_with_ellipsis(&story.title, MAX_STORY_TITLE_LEN);
            let story_line = format!(
                "    {} {} {}{}",
                icon, story.id, title_display, duration_str
            );
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

/// Format token count with k suffix (e.g., 12345 -> 12.3k)
fn format_token_count(tokens: u64) -> String {
    if tokens >= 1000 {
        format!("{:.1}k", tokens as f64 / 1000.0)
    } else {
        tokens.to_string()
    }
}

/// Parse cost information from a drone's activity log
/// Returns CostSummary with accumulated token usage and costs
/// Uses caching to avoid re-parsing unchanged logs
fn parse_cost_from_log(drone_name: &str) -> Option<CostSummary> {
    let log_path = PathBuf::from(".hive")
        .join("drones")
        .join(drone_name)
        .join("activity.log");

    // Check if log file exists
    if !log_path.exists() {
        return None;
    }

    // Get log file modification time
    let log_mtime = fs::metadata(&log_path).ok()?.modified().ok()?;

    // Check cache
    let cached = COST_CACHE.with(|cache| {
        let cache_ref = cache.borrow();
        cache_ref.get(drone_name).and_then(|entry| {
            if entry.log_mtime == log_mtime {
                Some(entry.summary.clone())
            } else {
                None
            }
        })
    });

    if let Some(summary) = cached {
        return Some(summary);
    }

    // Parse the log file
    let log_content = fs::read_to_string(&log_path).ok()?;
    let mut summary = CostSummary::default();

    for line in log_content.lines() {
        // Parse each line as JSON
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            // Look for result type with cost information
            if value.get("type")?.as_str() == Some("result") {
                if let Some(total_cost) = value.get("total_cost_usd") {
                    summary.total_cost_usd += total_cost.as_f64().unwrap_or(0.0);
                }

                if let Some(model_usage) = value.get("modelUsage") {
                    // Sum up costs from all models
                    if let Some(obj) = model_usage.as_object() {
                        for (_model, usage) in obj {
                            if let Some(cost) = usage.get("costUSD") {
                                summary.total_cost_usd += cost.as_f64().unwrap_or(0.0);
                            }
                        }
                    }
                }
            }

            // Look for assistant messages with usage info
            if value.get("type")?.as_str() == Some("assistant") {
                if let Some(message) = value.get("message") {
                    if let Some(usage) = message.get("usage") {
                        summary.input_tokens += usage
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        summary.output_tokens += usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        summary.cache_read_tokens += usage
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        summary.cache_creation_tokens += usage
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    }
                }
            }
        }
    }

    // Update cache
    COST_CACHE.with(|cache| {
        let mut cache_mut = cache.borrow_mut();
        cache_mut.insert(
            drone_name.to_string(),
            CostCacheEntry {
                summary: summary.clone(),
                log_mtime,
            },
        );
    });

    Some(summary)
}

fn run_tui(_name: Option<String>) -> Result<()> {
    /// Render activity data as a sparkline string using Unicode block characters.
    /// Takes a slice of activity values and returns an 8-character string.
    fn render_sparkline(data: &[u64]) -> String {
        // Unicode block characters for sparkline (from empty to full)
        const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        if data.is_empty() {
            return " ".repeat(SPARKLINE_WIDTH);
        }

        // Find max value for normalization
        let max_value = *data.iter().max().unwrap_or(&1);

        if max_value == 0 {
            return " ".repeat(SPARKLINE_WIDTH);
        }

        // Normalize and convert to block characters
        data.iter()
            .map(|&value| {
                let normalized = (value * 8) / max_value.max(1);
                let block_idx = normalized.min(8) as usize;
                BLOCKS[block_idx]
            })
            .collect()
    }

    /// Parse activity buckets for sparkline rendering.
    /// Returns a Vec<u64> where each element is the count of log events in that time bucket.
    /// Uses file size deltas to approximate activity without parsing each log line.
    fn parse_activity_buckets(
        log_path: &std::path::Path,
        activity_history: &mut Vec<(std::time::Instant, u64)>,
        bucket_count: usize,
        bucket_duration_secs: u64,
    ) -> Vec<u64> {
        use std::time::{Duration, Instant};

        // Get current file size
        let current_size = fs::metadata(log_path).map(|m| m.len()).unwrap_or(0);

        let now = Instant::now();

        // Add current reading to history
        activity_history.push((now, current_size));

        // Clean up old readings (keep last bucket_count * bucket_duration_secs worth of data)
        let max_age = Duration::from_secs(bucket_count as u64 * bucket_duration_secs);
        activity_history.retain(|(instant, _)| now.duration_since(*instant) <= max_age);

        // Create buckets
        let mut buckets = vec![0u64; bucket_count];
        let bucket_duration = Duration::from_secs(bucket_duration_secs);

        // For each bucket, calculate activity by summing file size deltas
        for i in 0..bucket_count {
            let bucket_start = now - bucket_duration * (bucket_count - i) as u32;
            let bucket_end = bucket_start + bucket_duration;

            // Find all readings in this bucket and sum the deltas
            let mut bucket_activity = 0u64;
            for j in 1..activity_history.len() {
                let (_prev_instant, prev_size) = activity_history[j - 1];
                let (curr_instant, curr_size) = activity_history[j];

                // Check if this delta falls within the bucket
                if curr_instant >= bucket_start && curr_instant < bucket_end {
                    bucket_activity += curr_size.saturating_sub(prev_size);
                }
            }

            // Normalize to a reasonable range (0-100) for sparkline rendering
            // Each character of log roughly = 1 byte, so divide by ~100 to get event count
            buckets[i] = bucket_activity / 100;
        }

        buckets
    }
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
    use std::collections::{HashMap, HashSet};
    use std::io;
    use std::time::Instant;

    // Install panic hook to restore terminal before printing panic info
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

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

    // Track activity history for sparkline rendering
    // Map: drone_name -> Vec<(check_time, file_size)>
    let mut activity_history: HashMap<String, Vec<(Instant, u64)>> = HashMap::new();

    // Feature 3: Split-pane log viewer state
    let mut log_pane: Option<String> = None; // Some(drone_name) when split view is active
    let mut log_pane_scroll: usize = 0;
    let mut log_pane_auto_scroll: bool = true;
    let mut log_pane_focus: bool = false; // false = dashboard focused, true = log pane focused

    // Feature 4: Timeline/Gantt view state
    let mut view_mode = ViewMode::Dashboard;
    let mut timeline_scroll: (usize, usize) = (0, 0); // (vertical, horizontal)

    loop {
        let mut drones = list_drones()?;

        // Sort: in_progress first, then blocked, then completed
        drones.sort_by_key(|(_, status)| match status.status {
            DroneState::InProgress | DroneState::Starting | DroneState::Resuming => 0,
            DroneState::Blocked | DroneState::Error => 1,
            DroneState::Stopped => 2,
            DroneState::Completed => 3,
        });

        // Load PRDs for story info (needed for archive calculation)
        let prd_cache: std::collections::HashMap<String, Prd> = drones
            .iter()
            .filter_map(|(_, status)| {
                let prd_path = PathBuf::from(".hive").join("prds").join(&status.prd);
                load_prd(&prd_path).map(|prd| (status.prd.clone(), prd))
            })
            .collect();

        // Build display order: active drones first, then archived
        // A drone is archived if: completed + all stories done + inactive > 1h
        let now = Utc::now();
        let mut display_order: Vec<usize> = Vec::new();
        let mut archived_order: Vec<usize> = Vec::new();

        for (idx, (_, status)) in drones.iter().enumerate() {
            if status.status == DroneState::Completed {
                let (valid_completed, prd_story_count) = prd_cache
                    .get(&status.prd)
                    .map(|prd| reconcile_progress_with_prd(status, prd))
                    .unwrap_or((status.completed.len(), status.total));

                if valid_completed >= prd_story_count {
                    let inactive_secs = parse_timestamp(&status.updated)
                        .map(|updated| now.signed_duration_since(updated).num_seconds())
                        .unwrap_or(0);

                    if inactive_secs >= DEFAULT_INACTIVE_THRESHOLD_SECS {
                        archived_order.push(idx);
                        continue;
                    }
                }
            }
            display_order.push(idx);
        }
        display_order.extend(archived_order);

        // Clamp selected index to display order
        if !display_order.is_empty() && selected_index >= display_order.len() {
            selected_index = display_order.len() - 1;
        }

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

            // Check if we're showing the timeline view
            if view_mode == ViewMode::Timeline {
                render_timeline_view(f, area, &drones, &prd_cache, timeline_scroll);
                return;
            }

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

            // Main layout: header, content (with optional split pane), footer
            let chunks = if log_pane.is_some() {
                // Split-pane layout: header, dashboard, divider, log pane, footer
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(4),      // Header (reduced by 1 for compactness)
                        Constraint::Percentage(50), // Dashboard
                        Constraint::Length(1),      // Divider
                        Constraint::Percentage(50), // Log pane
                        Constraint::Length(1),      // Footer
                    ])
                    .split(area)
            } else {
                // Normal layout: header, content, footer
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(5), // Header with ASCII art + padding
                        Constraint::Min(0),    // Content
                        Constraint::Length(1), // Footer
                    ])
                    .split(area)
            };

            // Header with ASCII art (with top padding)
            let header_lines = vec![
                Line::raw(""), // Top padding
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

            // Use pre-computed display_order for rendering
            // display_order contains: [active indices..., archived indices...]
            let active_count = display_order
                .iter()
                .take_while(|&&idx| {
                    let status = &drones[idx].1;
                    if status.status != DroneState::Completed {
                        return true;
                    }
                    let (valid_completed, prd_story_count) = prd_cache
                        .get(&status.prd)
                        .map(|prd| reconcile_progress_with_prd(status, prd))
                        .unwrap_or((status.completed.len(), status.total));
                    if valid_completed < prd_story_count {
                        return true;
                    }
                    let inactive_secs = parse_timestamp(&status.updated)
                        .map(|updated| now.signed_duration_since(updated).num_seconds())
                        .unwrap_or(0);
                    inactive_secs < DEFAULT_INACTIVE_THRESHOLD_SECS
                })
                .count();

            // Render ACTIVE section
            if active_count > 0 {
                lines.push(Line::from(vec![
                    Span::styled(
                        "  🍯 ACTIVE",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({})", active_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                lines.push(Line::raw(""));
            }

            for (display_idx, &drone_idx) in display_order.iter().enumerate() {
                // Add ARCHIVED header before first archived drone
                if display_idx == active_count && active_count < display_order.len() {
                    lines.push(Line::styled(
                        "  ─────────────────────────────────────────────────────",
                        Style::default().fg(Color::DarkGray),
                    ));
                    lines.push(Line::raw(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            "  🐻 ARCHIVED",
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" ({})", display_order.len() - active_count),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                    lines.push(Line::raw(""));
                }

                let _is_archived = display_idx >= active_count;
                let (name, status) = &drones[drone_idx];
                drone_line_indices.push(lines.len());

                let is_selected = display_idx == selected_index;
                let is_expanded = expanded_drones.contains(name);
                let process_running = read_drone_pid(name)
                    .map(is_process_running)
                    .unwrap_or(false);

                // Status icon and color
                let is_active_process = process_running || status.current_story.is_some();
                let (icon, status_color) = match status.status {
                    DroneState::Starting | DroneState::Resuming => ("◐", Color::Yellow),
                    DroneState::InProgress => {
                        if is_active_process {
                            ("◐", Color::Green)
                        } else {
                            ("○", Color::Yellow)
                        }
                    }
                    DroneState::Completed => ("●", Color::Green),
                    DroneState::Error => ("◐", Color::Red),
                    DroneState::Blocked => ("◐", Color::Red),
                    DroneState::Stopped => ("○", Color::DarkGray),
                };

                // Use reconciled progress to filter out old completed stories
                let (valid_completed, prd_story_count) = prd_cache
                    .get(&status.prd)
                    .map(|prd| reconcile_progress_with_prd(status, prd))
                    .unwrap_or((status.completed.len(), status.total));
                let has_new_stories = prd_story_count > status.total;

                let percentage = if prd_story_count > 0 {
                    (valid_completed as f32 / prd_story_count as f32 * 100.0) as u16
                } else {
                    0
                };

                // Build progress bar (10 chars wide - compact)
                let bar_width = 10;
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

                // Use different emoji based on execution mode
                let mode_emoji = match status.execution_mode {
                    ExecutionMode::Subagent => "🤖",
                    ExecutionMode::Worktree => "🐝",
                    ExecutionMode::Swarm => "🐝",
                };

                let name_display = truncate_with_ellipsis(name, MAX_DRONE_NAME_LEN);

                // Backend tag: [worktree|native], [subagent|swarm], etc.
                let mode_tag = format!("[{}|{}]", status.execution_mode, status.backend);

                // Check inbox for pending messages
                let inbox_dir = PathBuf::from(".hive/drones").join(name).join("inbox");
                let inbox_count = if inbox_dir.exists() {
                    fs::read_dir(&inbox_dir)
                        .map(|entries| {
                            entries
                                .filter_map(|e| e.ok())
                                .filter(|e| {
                                    e.path().extension().and_then(|s| s.to_str()) == Some("json")
                                })
                                .count()
                        })
                        .unwrap_or(0)
                } else {
                    0
                };
                let inbox_indicator = if inbox_count > 0 {
                    format!(" ✉{}", inbox_count)
                } else {
                    String::new()
                };

                // Parse cost from activity log
                let cost_info = parse_cost_from_log(name);
                let (cost_str, cost_color) = if let Some(ref cost) = cost_info {
                    let cost_usd = cost.total_cost_usd;
                    let color = if cost_usd < 1.0 {
                        Color::Green
                    } else if cost_usd < 5.0 {
                        Color::Yellow
                    } else {
                        Color::Red
                    };
                    (format!(" ${:.2}", cost_usd), color)
                } else {
                    (String::new(), Color::DarkGray)
                };

                // Calculate activity sparkline
                let log_path = PathBuf::from(".hive")
                    .join("drones")
                    .join(name)
                    .join("activity.log");

                let drone_history = activity_history.entry(name.clone()).or_default();
                let activity_data = parse_activity_buckets(
                    &log_path,
                    drone_history,
                    SPARKLINE_WIDTH,
                    ACTIVITY_BUCKET_SECS,
                );

                // Check if sparkline is all zeros (stalled drone)
                let is_stalled = activity_data.iter().all(|&v| v == 0);
                let sparkline_chars = render_sparkline(&activity_data);
                let sparkline_style = if is_stalled {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Cyan)
                };

                let header_line = Line::from(vec![
                    Span::raw(format!(" {} ", select_char)),
                    Span::styled(icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(expand_indicator, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(format!("{} {} ", mode_emoji, name_display), name_style),
                    Span::styled(filled_bar, Style::default().fg(filled_color)),
                    Span::styled(empty_bar, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{}/{}", valid_completed, prd_story_count),
                        Style::default().fg(
                            if status.status == DroneState::Completed && !has_new_stories {
                                Color::DarkGray
                            } else if has_new_stories {
                                Color::Cyan
                            } else {
                                Color::White
                            },
                        ),
                    ),
                    Span::raw("  "),
                    Span::styled(mode_tag.clone(), Style::default().fg(Color::DarkGray)),
                    Span::styled(inbox_indicator.clone(), Style::default().fg(Color::Cyan)),
                    Span::styled(cost_str, Style::default().fg(cost_color)),
                    Span::raw("  "),
                    Span::styled(elapsed, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(sparkline_chars, sparkline_style),
                ]);
                lines.push(header_line);

                // Expanded: show cost breakdown and stories
                if is_expanded {
                    // Add cost breakdown line if cost data is available
                    if let Some(cost) = cost_info {
                        let cost_breakdown = Line::from(vec![
                            Span::raw("    "),
                            Span::styled(
                                format!("Cost: ${:.2}", cost.total_cost_usd),
                                Style::default().fg(cost_color),
                            ),
                            Span::raw(" | "),
                            Span::styled(
                                format!("In: {}", format_token_count(cost.input_tokens)),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::raw(" | "),
                            Span::styled(
                                format!("Out: {}", format_token_count(cost.output_tokens)),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::raw(" | "),
                            Span::styled(
                                format!(
                                    "Cache: {} read, {} created",
                                    format_token_count(cost.cache_read_tokens),
                                    format_token_count(cost.cache_creation_tokens)
                                ),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]);
                        lines.push(cost_breakdown);
                    }

                    if let Some(prd) = prd_cache.get(&status.prd) {
                        for (story_idx, story) in prd.stories.iter().enumerate() {
                            let is_completed = status.completed.contains(&story.id);
                            let is_current = status.current_story.as_ref() == Some(&story.id);
                            let is_story_selected =
                                is_selected && selected_story_index == Some(story_idx);

                            // Check if story has unsatisfied dependencies
                            let has_blocked_deps = if !story.depends_on.is_empty() && !is_completed
                            {
                                story
                                    .depends_on
                                    .iter()
                                    .any(|dep_id| !status.completed.contains(dep_id))
                            } else {
                                false
                            };

                            // ◐ = half-full (in progress), ● = full (completed), ○ = empty (pending), ⏳ = blocked by deps
                            let (story_icon, story_color) = if is_story_selected {
                                ("▸", Color::Cyan)
                            } else if is_completed {
                                ("●", Color::Green) // Full green = completed
                            } else if has_blocked_deps {
                                ("⏳", Color::Yellow) // Waiting for dependency
                            } else if is_current {
                                ("◐", Color::Yellow) // Half-full yellow = in progress
                            } else {
                                ("○", Color::DarkGray) // Empty = pending
                            };

                            // Dependency info suffix
                            let dep_info = if has_blocked_deps {
                                let missing: Vec<&str> = story
                                    .depends_on
                                    .iter()
                                    .filter(|dep_id| !status.completed.contains(dep_id))
                                    .map(|s| s.as_str())
                                    .collect();
                                format!(" ⏳ waiting: {}", missing.join(", "))
                            } else {
                                String::new()
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

                            let line_style = if is_story_selected {
                                Style::default().add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            };

                            let title_color = if is_story_selected {
                                Color::Cyan
                            } else {
                                story_color
                            };

                            // Calculate max title width (terminal width - prefix - duration)
                            // Prefix: "      ● STORY-ID         " = ~26 chars
                            let prefix_len = 26;
                            let duration_len = duration_str.len();
                            let available_width = area.width as usize;
                            let max_title_width =
                                available_width.saturating_sub(prefix_len + duration_len + 2);

                            if story.title.len() <= max_title_width || max_title_width < 20 {
                                // Title fits on one line or terminal too narrow
                                let mut spans = vec![
                                    Span::styled("      ", line_style),
                                    Span::styled(story_icon, line_style.fg(story_color)),
                                    Span::raw(" "),
                                    Span::styled(
                                        format!("{:<16} ", story.id),
                                        line_style.fg(title_color),
                                    ),
                                    Span::styled(story.title.clone(), line_style.fg(title_color)),
                                    Span::styled(
                                        duration_str.clone(),
                                        line_style.fg(Color::DarkGray),
                                    ),
                                ];
                                if !dep_info.is_empty() {
                                    spans.push(Span::styled(
                                        dep_info.clone(),
                                        Style::default().fg(Color::Yellow),
                                    ));
                                }
                                lines.push(Line::from(spans));
                            } else {
                                // Title needs to wrap - split into lines
                                let title_indent = "                         "; // 25 spaces to align with title start
                                let mut remaining = story.title.as_str();
                                let mut first_line = true;

                                while !remaining.is_empty() {
                                    let char_count = remaining.chars().count();
                                    let (chunk, rest) = if char_count <= max_title_width {
                                        (remaining, "")
                                    } else {
                                        // Find byte index for max_title_width characters
                                        let byte_limit: usize = remaining
                                            .char_indices()
                                            .nth(max_title_width)
                                            .map(|(i, _)| i)
                                            .unwrap_or(remaining.len());
                                        // Try to break at word boundary within the safe range
                                        let break_at = remaining[..byte_limit]
                                            .rfind(' ')
                                            .unwrap_or(byte_limit);
                                        (&remaining[..break_at], remaining[break_at..].trim_start())
                                    };

                                    if first_line {
                                        lines.push(Line::from(vec![
                                            Span::styled("      ", line_style),
                                            Span::styled(story_icon, line_style.fg(story_color)),
                                            Span::raw(" "),
                                            Span::styled(
                                                format!("{:<16} ", story.id),
                                                line_style.fg(title_color),
                                            ),
                                            Span::styled(
                                                chunk.to_string(),
                                                line_style.fg(title_color),
                                            ),
                                            if rest.is_empty() {
                                                Span::styled(
                                                    duration_str.clone(),
                                                    line_style.fg(Color::DarkGray),
                                                )
                                            } else {
                                                Span::raw("")
                                            },
                                        ]));
                                        first_line = false;
                                    } else {
                                        lines.push(Line::from(vec![
                                            Span::styled(title_indent, line_style),
                                            Span::styled(
                                                chunk.to_string(),
                                                line_style.fg(title_color),
                                            ),
                                            if rest.is_empty() {
                                                Span::styled(
                                                    duration_str.clone(),
                                                    line_style.fg(Color::DarkGray),
                                                )
                                            } else {
                                                Span::raw("")
                                            },
                                        ]));
                                    }
                                    remaining = rest;
                                }
                            }
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

                // Add separator between drones with spacing
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

            // Render divider and log pane if split view is active
            if let Some(ref log_drone_name) = log_pane {
                // Divider line
                let divider_line = Line::styled(
                    "─".repeat(area.width as usize),
                    Style::default().fg(Color::DarkGray),
                );
                f.render_widget(Paragraph::new(divider_line), chunks[2]);

                // Get execution mode for the log pane drone
                let log_exec_mode = drones
                    .iter()
                    .find(|(name, _)| name == log_drone_name)
                    .map(|(_, status)| &status.execution_mode)
                    .unwrap_or(&ExecutionMode::Worktree);

                // Render log pane
                render_log_pane(
                    f,
                    chunks[3],
                    log_drone_name,
                    log_pane_scroll,
                    log_pane_auto_scroll,
                    log_pane_focus,
                    log_exec_mode,
                );
            }

            // Footer - shortcuts (context-dependent)
            let footer_chunk_idx = if log_pane.is_some() { 4 } else { 2 };
            let footer_text = if let Some(msg) = &message {
                msg.clone()
            } else if log_pane.is_some() && log_pane_focus {
                " ↑↓jk scroll  g/G top/bottom  Tab switch  q close".to_string()
            } else if log_pane.is_some() {
                " ↑↓jk navigate  Tab switch to logs  q close logs  t timeline".to_string()
            } else if selected_story_index.is_some() {
                " i info  l logs  L logs-split  ↑↓ navigate  ← back  q back".to_string()
            } else {
                " ↵ expand  l logs  L split  b blocked  m msgs  x stop  c clean  t timeline  q quit"
                    .to_string()
            };

            let footer = Paragraph::new(Line::from(vec![Span::styled(
                footer_text,
                Style::default().fg(if message.is_some() {
                    message_color
                } else {
                    Color::DarkGray
                }),
            )]));
            f.render_widget(footer, chunks[footer_chunk_idx]);
        })?;

        // Clear message after displaying
        if message.is_some() {
            message = None;
        }

        // Handle input (including resize events)
        if event::poll(std::time::Duration::from_millis(TUI_POLL_TIMEOUT_MS))? {
            match event::read()? {
                Event::Resize(_, _) => {
                    // Terminal resized, just continue to redraw
                    continue;
                }
                Event::Key(key) => {
                    // Convert display index to actual drone index
                    // selected_index is position in display_order, we need actual index in drones
                    let current_drone_idx =
                        if !display_order.is_empty() && selected_index < display_order.len() {
                            display_order[selected_index]
                        } else {
                            0
                        };

                    // Get story count for current drone if expanded
                    let current_story_count =
                        if !drones.is_empty() && current_drone_idx < drones.len() {
                            let drone_name = &drones[current_drone_idx].0;
                            let status = &drones[current_drone_idx].1;
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
                            // Close log pane if open
                            if log_pane.is_some() {
                                log_pane = None;
                                log_pane_focus = false;
                                log_pane_scroll = 0;
                                log_pane_auto_scroll = true;
                            } else if blocked_view.is_some() {
                                // If in blocked view, go back to main view
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
                                let drone_name = &drones[current_drone_idx].0;
                                let status = &drones[current_drone_idx].1;
                                if status.status == DroneState::Blocked {
                                    blocked_view = Some(drone_name.clone());
                                } else {
                                    message = Some("Drone is not blocked".to_string());
                                    message_color = Color::Yellow;
                                }
                            }
                        }
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            // Show messages for current drone
                            if !drones.is_empty() {
                                let drone_name = &drones[current_drone_idx].0;
                                let bus = crate::communication::file_bus::FileBus::new();
                                let inbox = bus.peek(drone_name).unwrap_or_default();
                                let outbox = bus.list_outbox(drone_name).unwrap_or_default();
                                if inbox.is_empty() && outbox.is_empty() {
                                    message = Some(format!("No messages for '{}'", drone_name));
                                    message_color = Color::DarkGray;
                                } else {
                                    let mut msg_parts = Vec::new();
                                    if !inbox.is_empty() {
                                        msg_parts.push(format!("📥 {} inbox", inbox.len()));
                                    }
                                    if !outbox.is_empty() {
                                        msg_parts.push(format!("📤 {} outbox", outbox.len()));
                                    }
                                    message = Some(format!(
                                        "✉ '{}': {}",
                                        drone_name,
                                        msg_parts.join(", ")
                                    ));
                                    message_color = Color::Cyan;
                                }
                            }
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            if view_mode == ViewMode::Timeline {
                                // Scroll timeline down
                                timeline_scroll.0 = timeline_scroll.0.saturating_add(1);
                            } else if log_pane_focus {
                                // Scroll log pane down
                                log_pane_scroll = log_pane_scroll.saturating_add(1);
                                log_pane_auto_scroll = false;
                            } else if !drones.is_empty() {
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
                            if view_mode == ViewMode::Timeline {
                                // Scroll timeline up
                                timeline_scroll.0 = timeline_scroll.0.saturating_sub(1);
                            } else if log_pane_focus {
                                // Scroll log pane up
                                log_pane_scroll = log_pane_scroll.saturating_sub(1);
                                log_pane_auto_scroll = false;
                            } else if let Some(story_idx) = selected_story_index {
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
                        KeyCode::Char('g') => {
                            if log_pane_focus {
                                // Jump to top of log pane
                                log_pane_scroll = 0;
                                log_pane_auto_scroll = false;
                            }
                        }
                        KeyCode::Char('G') => {
                            if log_pane_focus {
                                // Jump to bottom of log pane
                                log_pane_scroll = usize::MAX;
                                log_pane_auto_scroll = true;
                            }
                        }
                        KeyCode::Tab => {
                            if log_pane.is_some() {
                                // Toggle focus between dashboard and log pane
                                log_pane_focus = !log_pane_focus;
                            }
                        }
                        KeyCode::Enter => {
                            if !drones.is_empty() {
                                let drone_name = &drones[current_drone_idx].0;
                                if selected_story_index.is_some() {
                                    // Enter on story = show story details
                                    if let Some(story_idx) = selected_story_index {
                                        let status = &drones[current_drone_idx].1;
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
                        KeyCode::Left if view_mode == ViewMode::Dashboard => {
                            // Collapse current drone (dashboard mode only)
                            if !drones.is_empty() {
                                let drone_name = &drones[current_drone_idx].0;
                                expanded_drones.remove(drone_name);
                                selected_story_index = None;
                            }
                        }
                        KeyCode::Right if view_mode == ViewMode::Dashboard => {
                            // Expand current drone or enter stories (dashboard mode only)
                            if !drones.is_empty() {
                                let drone_name = &drones[current_drone_idx].0;
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
                        KeyCode::Char('l') if view_mode == ViewMode::Dashboard => {
                            // Toggle split-pane log viewer (lowercase 'l') - only in dashboard mode
                            if !drones.is_empty() {
                                let drone_name = &drones[current_drone_idx].0;
                                if log_pane.as_ref() == Some(drone_name) {
                                    // Close log pane if already open for this drone
                                    log_pane = None;
                                    log_pane_focus = false;
                                    log_pane_scroll = 0;
                                    log_pane_auto_scroll = true;
                                } else {
                                    // Open log pane for selected drone
                                    log_pane = Some(drone_name.clone());
                                    log_pane_focus = false;
                                    log_pane_scroll = 0;
                                    log_pane_auto_scroll = true;
                                }
                            }
                        }
                        KeyCode::Char('L') => {
                            // Open full-screen logs viewer (uppercase 'L')
                            if let Some(ref drone_name) = blocked_view {
                                // In blocked view - open logs for this drone
                                // Find the execution mode for this drone
                                let exec_mode = drones
                                    .iter()
                                    .find(|(name, _)| name == drone_name)
                                    .map(|(_, s)| &s.execution_mode)
                                    .unwrap_or(&ExecutionMode::Worktree);
                                match show_logs_viewer(&mut terminal, drone_name, exec_mode) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        message = Some(format!("Error: {}", e));
                                        message_color = Color::Red;
                                    }
                                }
                            } else if !drones.is_empty() {
                                let drone_name = &drones[current_drone_idx].0;
                                let status = &drones[current_drone_idx].1;
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
                                    match show_logs_viewer(
                                        &mut terminal,
                                        drone_name,
                                        &status.execution_mode,
                                    ) {
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
                                    let status = &drones[current_drone_idx].1;
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
                                let drone_name = drones[current_drone_idx].0.clone();
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
                                let drone_name = drones[current_drone_idx].0.clone();
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
                                let drone_name = &drones[current_drone_idx].0;
                                let status = &drones[current_drone_idx].1;
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
                                let drone_name = &drones[current_drone_idx].0;
                                message = Some(format!("Use: hive sessions {}", drone_name));
                                message_color = Color::Yellow;
                            }
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            // Resume drone (especially useful when new stories added to PRD)
                            if !drones.is_empty() {
                                let drone_name = drones[current_drone_idx].0.clone();
                                let status = &drones[current_drone_idx].1;
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
                        KeyCode::Char('t') | KeyCode::Char('T') => {
                            // Toggle timeline view
                            view_mode = match view_mode {
                                ViewMode::Dashboard => ViewMode::Timeline,
                                ViewMode::Timeline => ViewMode::Dashboard,
                            };
                            // Reset timeline scroll when entering
                            if view_mode == ViewMode::Timeline {
                                timeline_scroll = (0, 0);
                            }
                        }
                        KeyCode::Char('h') | KeyCode::Left if view_mode == ViewMode::Timeline => {
                            // Scroll timeline left
                            timeline_scroll.1 = timeline_scroll.1.saturating_sub(5);
                        }
                        KeyCode::Char('l') | KeyCode::Right if view_mode == ViewMode::Timeline => {
                            // Scroll timeline right
                            timeline_scroll.1 = timeline_scroll.1.saturating_add(5);
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

        // Launch drone using start command (default to worktree mode, not subagent)
        crate::commands::start::run(
            drone_name.clone(),
            None,
            false,
            false,
            model,
            false,
            false,
            false,
        )?;

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

    // Launch drone with resume flag (preserve original mode, default to worktree)
    crate::commands::start::run(
        drone_name.to_string(),
        None,
        true,
        false,
        "sonnet".to_string(),
        false,
        false, // subagent mode - TODO: could read from status.json to preserve mode
        false, // wait
    )?;
    Ok(format!("🔄 Resumed drone: {}", drone_name))
}

/// Render the timeline/Gantt chart view
fn render_timeline_view(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    drones: &[(String, DroneStatus)],
    prd_cache: &HashMap<String, Prd>,
    scroll: (usize, usize),
) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    };

    // Layout: header, timeline content, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header
            Constraint::Min(0),    // Timeline content
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Header
    let header_lines = vec![
        Line::from(vec![
            Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
            Span::styled(
                "  Timeline View",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!(
                    "  {} drone{}",
                    drones.len(),
                    if drones.len() != 1 { "s" } else { "" }
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![Span::styled(
            "  ╩ ╩╩ ╚╝ ╚═╝",
            Style::default().fg(Color::Yellow),
        )]),
    ];
    f.render_widget(Paragraph::new(header_lines), chunks[0]);

    // Collect all story timings across all drones
    let mut timeline_entries: Vec<(
        String,
        String,
        String,
        Option<chrono::DateTime<Utc>>,
        Option<chrono::DateTime<Utc>>,
        DroneState,
    )> = Vec::new();

    for (drone_name, status) in drones {
        if let Some(prd) = prd_cache.get(&status.prd) {
            for story in &prd.stories {
                if let Some(story_time) = status.story_times.get(&story.id) {
                    let start_time = story_time.started.as_ref().and_then(|s| parse_timestamp(s));
                    let end_time = story_time
                        .completed
                        .as_ref()
                        .and_then(|s| parse_timestamp(s));

                    // Determine story state
                    let story_state = if status.completed.contains(&story.id) {
                        DroneState::Completed
                    } else if status.current_story.as_deref() == Some(&story.id) {
                        DroneState::InProgress
                    } else {
                        DroneState::Stopped
                    };

                    timeline_entries.push((
                        drone_name.clone(),
                        story.id.clone(),
                        story.title.clone(),
                        start_time,
                        end_time,
                        story_state,
                    ));
                }
            }
        }
    }

    // If no timeline data, show placeholder
    if timeline_entries.is_empty() {
        let placeholder_lines = vec![
            Line::raw(""),
            Line::raw(""),
            Line::from(vec![Span::styled(
                "  No timeline data available",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "Timeline shows story progress across drones",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(placeholder_lines), chunks[1]);

        let footer = Paragraph::new(Line::from(vec![Span::styled(
            " t/q: back to dashboard  hjkl/arrows: scroll",
            Style::default().fg(Color::DarkGray),
        )]));
        f.render_widget(footer, chunks[2]);
        return;
    }

    // Calculate time range
    let now = Utc::now();
    let min_time = timeline_entries
        .iter()
        .filter_map(|(_, _, _, start, _, _)| *start)
        .min()
        .unwrap_or(now);
    let max_time = timeline_entries
        .iter()
        .filter_map(|(_, _, _, _, end, _)| *end)
        .max()
        .unwrap_or(now)
        .max(now);

    let total_duration = max_time
        .signed_duration_since(min_time)
        .num_seconds()
        .max(1);

    // Available width for timeline bars (accounting for labels)
    let label_width = 35; // "drone-name | story-id"
    let duration_width = 10; // " 1h 23m"
    let available_width = (area.width as usize).saturating_sub(label_width + duration_width + 4);

    // Build timeline lines
    let mut lines: Vec<Line> = Vec::new();

    // Time axis header
    let time_axis = {
        let mut axis_line = String::new();
        axis_line.push_str(&format!("{:label_width$}  ", "DRONE | STORY"));

        // Show start and end times
        let start_str = min_time.format("%H:%M").to_string();
        let end_str = max_time.format("%H:%M").to_string();

        if available_width > 20 {
            let padding = available_width.saturating_sub(start_str.len() + end_str.len());
            axis_line.push_str(&start_str);
            axis_line.push_str(&"─".repeat(padding));
            axis_line.push_str(&end_str);
        } else {
            axis_line.push_str(&"─".repeat(available_width));
        }

        Line::from(vec![Span::styled(
            axis_line,
            Style::default().fg(Color::DarkGray),
        )])
    };
    lines.push(time_axis);
    lines.push(Line::raw(""));

    // Render each story as a timeline bar
    for (drone_name, story_id, story_title, start_time, end_time, story_state) in &timeline_entries
    {
        let label = format!(
            "{} | {}",
            truncate_with_ellipsis(drone_name, 15),
            truncate_with_ellipsis(story_id, 12)
        );

        // Calculate bar position and width
        let (bar_start, bar_width) = if let Some(start) = start_time {
            let start_offset = start.signed_duration_since(min_time).num_seconds();
            let end_offset = if let Some(end) = end_time {
                end.signed_duration_since(min_time).num_seconds()
            } else {
                now.signed_duration_since(min_time).num_seconds()
            };

            let start_pos =
                (start_offset as f64 / total_duration as f64 * available_width as f64) as usize;
            let end_pos =
                (end_offset as f64 / total_duration as f64 * available_width as f64) as usize;
            let width = end_pos.saturating_sub(start_pos).max(1);

            (start_pos, width)
        } else {
            (0, 1)
        };

        // Choose bar character and color based on state
        let (bar_char, bar_color) = match story_state {
            DroneState::Completed => ('█', Color::Green),
            DroneState::InProgress => ('▓', Color::Yellow),
            _ => ('░', Color::DarkGray),
        };

        // Build the timeline bar
        let mut bar_line = format!("{:label_width$}  ", label);
        bar_line.push_str(&" ".repeat(bar_start));
        bar_line.push_str(&bar_char.to_string().repeat(bar_width));

        // Add duration
        let duration_str = if let (Some(start), Some(end)) = (start_time, end_time) {
            let dur = end.signed_duration_since(*start);
            let hours = dur.num_hours();
            let mins = dur.num_minutes() % 60;
            if hours > 0 {
                format!(" {}h {}m", hours, mins)
            } else {
                format!(" {}m", mins)
            }
        } else if let Some(start) = start_time {
            let dur = now.signed_duration_since(*start);
            let hours = dur.num_hours();
            let mins = dur.num_minutes() % 60;
            if hours > 0 {
                format!(" {}h {}m", hours, mins)
            } else {
                format!(" {}m", mins)
            }
        } else {
            String::from(" -")
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("{:label_width$}  ", label),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" ".repeat(bar_start), Style::default()),
            Span::styled(
                bar_char.to_string().repeat(bar_width),
                Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
        ]));

        // Add story title on next line (indented)
        let title_line = format!("    {}", truncate_with_ellipsis(story_title, 70));
        lines.push(Line::from(vec![Span::styled(
            title_line,
            Style::default().fg(Color::DarkGray),
        )]));
        lines.push(Line::raw(""));
    }

    // Handle scrolling
    let (v_scroll, _h_scroll) = scroll;
    let content_height = chunks[1].height as usize;
    let total_lines = lines.len();
    let actual_v_scroll = v_scroll.min(total_lines.saturating_sub(content_height).max(0));

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(actual_v_scroll)
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
            .position(actual_v_scroll)
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
        " t/q/Esc: back to dashboard  hjkl/arrows: scroll",
        Style::default().fg(Color::DarkGray),
    )]));
    f.render_widget(footer, chunks[2]);
}

/// Render the split-pane log viewer at the bottom of the screen
fn render_log_pane(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    drone_name: &str,
    scroll_offset: usize,
    auto_scroll: bool,
    is_focused: bool,
    execution_mode: &ExecutionMode,
) {
    use ratatui::{
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    };

    // Build the log path based on execution mode
    let log_path = match execution_mode {
        ExecutionMode::Subagent => {
            // Subagent logs are in ~/.hive/drones/{name}/subagent.log
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join(".hive")
                .join("drones")
                .join(drone_name)
                .join("subagent.log")
        }
        _ => {
            // Worktree logs are in .hive/drones/{name}/activity.log
            PathBuf::from(".hive")
                .join("drones")
                .join(drone_name)
                .join("activity.log")
        }
    };

    // Read log file
    let log_lines: Vec<String> = if log_path.exists() {
        match fs::read_to_string(&log_path) {
            Ok(content) => content.lines().map(|s| s.to_string()).collect(),
            Err(_) => vec!["Error reading log file".to_string()],
        }
    } else {
        vec!["Log file not found".to_string()]
    };

    // Calculate scroll position
    let total_lines = log_lines.len();
    let content_height = area.height.saturating_sub(2) as usize; // Subtract border
    let actual_scroll = if auto_scroll && total_lines > content_height {
        total_lines.saturating_sub(content_height)
    } else {
        scroll_offset.min(total_lines.saturating_sub(content_height).max(0))
    };

    // Get visible lines
    let visible_lines: Vec<Line> = log_lines
        .iter()
        .skip(actual_scroll)
        .take(content_height)
        .map(|line| {
            // Parse JSON log lines for better formatting
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                let timestamp = value
                    .get("timestamp")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let level = value
                    .get("level")
                    .and_then(|l| l.as_str())
                    .unwrap_or("INFO")
                    .to_string();
                let message = value
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or(line)
                    .to_string();

                let level_color = match level.as_str() {
                    "ERROR" => Color::Red,
                    "WARN" => Color::Yellow,
                    "INFO" => Color::Cyan,
                    "DEBUG" => Color::DarkGray,
                    _ => Color::White,
                };

                let time_str = if timestamp.len() >= 19 {
                    timestamp[11..19].to_string()
                } else {
                    timestamp.clone()
                };

                Line::from(vec![
                    Span::styled(
                        format!("{:<8} ", level),
                        Style::default()
                            .fg(level_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{} ", time_str),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(message),
                ])
            } else {
                Line::raw(line.clone())
            }
        })
        .collect();

    // Build border with title
    let border_style = if is_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(
        " Logs: {} {} (Tab: switch focus | q: close) ",
        drone_name,
        if auto_scroll { "[auto-scroll]" } else { "" }
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(title, border_style));

    let paragraph = Paragraph::new(visible_lines).block(block);
    f.render_widget(paragraph, area);

    // Render scrollbar if needed
    if total_lines > content_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(actual_scroll)
            .viewport_content_length(content_height);

        let scrollbar_area = ratatui::layout::Rect {
            x: area.x + area.width - 1,
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
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

    // Use different emoji based on execution mode
    let mode_emoji = match status.execution_mode {
        ExecutionMode::Subagent => "🤖",
        ExecutionMode::Worktree => "🐝",
        ExecutionMode::Swarm => "🐝",
    };

    let subheader_lines = vec![
        Line::from(vec![
            Span::styled("  ⚠ ", Style::default().fg(orange)),
            Span::styled(
                format!("{} {}", mode_emoji, drone_name),
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

// Show logs viewer in TUI with line selection and JSON pretty-print
fn show_logs_viewer<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    drone_name: &str,
    execution_mode: &ExecutionMode,
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

                // Use different emoji based on execution mode
                let mode_emoji = match execution_mode {
                    ExecutionMode::Subagent => "🤖",
                    ExecutionMode::Worktree => "🐝",
                    ExecutionMode::Swarm => "🐝",
                };

                // Header with HIVE ASCII art
                let header_lines = vec![
                    Line::from(vec![
                        Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
                        Span::styled("  Log Detail", Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(vec![
                        Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("  {} {}", mode_emoji, drone_name),
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

            // Use different emoji based on execution mode
            let mode_emoji = match execution_mode {
                ExecutionMode::Subagent => "🤖",
                ExecutionMode::Worktree => "🐝",
                ExecutionMode::Swarm => "🐝",
            };

            // Header with HIVE ASCII art
            let header_lines = vec![
                Line::from(vec![
                    Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
                    Span::styled("  Activity Logs", Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("  {} {}", mode_emoji, drone_name),
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
        if event::poll(std::time::Duration::from_millis(LOG_POLL_TIMEOUT_MS))? {
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
