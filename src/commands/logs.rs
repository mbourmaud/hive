use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::io::{BufRead, BufReader, Seek};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub fn run(name: String, lines: Option<usize>, story: Option<String>, follow: bool) -> Result<()> {
    let drone_dir = PathBuf::from(".hive/drones").join(&name);

    if !drone_dir.exists() {
        bail!("Drone '{}' not found", name);
    }

    if follow {
        if let Some(story_id) = story {
            follow_story_logs(&drone_dir, &story_id)
        } else {
            follow_activity_log(&drone_dir)
        }
    } else if let Some(story_id) = story {
        show_story_logs(&drone_dir, &story_id, lines)
    } else {
        show_activity_log(&drone_dir, lines)
    }
}

fn show_activity_log(drone_dir: &std::path::Path, lines: Option<usize>) -> Result<()> {
    let log_path = drone_dir.join("activity.log");

    if !log_path.exists() {
        println!("{}", "No activity log found".yellow());
        return Ok(());
    }

    println!("{}", "Activity Log".bright_cyan().bold());
    println!();

    let file = fs::File::open(&log_path).context("Failed to open activity log")?;
    let reader = BufReader::new(file);

    let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let lines_to_show = if let Some(n) = lines {
        if all_lines.len() > n {
            &all_lines[all_lines.len() - n..]
        } else {
            &all_lines
        }
    } else {
        &all_lines
    };

    for line in lines_to_show {
        // Colorize log lines based on content
        if line.contains("✅") {
            println!("{}", line.green());
        } else if line.contains("❌") || line.contains("Error") {
            println!("{}", line.red());
        } else if line.contains("🔨") {
            println!("{}", line.bright_blue());
        } else if line.contains("💾") {
            println!("{}", line.bright_yellow());
        } else {
            println!("{}", line);
        }
    }

    Ok(())
}

fn follow_activity_log(drone_dir: &std::path::Path) -> Result<()> {
    let log_path = drone_dir.join("activity.log");

    if !log_path.exists() {
        println!("{}", "Waiting for activity log...".yellow());
        // Wait for file to be created
        while !log_path.exists() {
            thread::sleep(Duration::from_secs(1));
        }
    }

    println!(
        "{} (Press Ctrl+C to exit)",
        "Following Activity Log".bright_cyan().bold()
    );
    println!();

    let mut file = fs::File::open(&log_path)?;
    let mut reader = BufReader::new(&mut file);

    // Read existing content first
    let mut line = String::new();
    while reader.read_line(&mut line).ok().is_some_and(|n| n > 0) {
        print_colored_line(&line);
        line.clear();
    }

    // Now tail new lines
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // No new data, sleep and try again
                thread::sleep(Duration::from_millis(500));
                // Re-open file to catch new writes
                file = fs::File::open(&log_path)?;
                file.stream_position()?;
                reader = BufReader::new(&mut file);
            }
            Ok(_) => {
                print_colored_line(&line);
            }
            Err(e) => {
                eprintln!("Error reading log: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn follow_story_logs(drone_dir: &std::path::Path, story_id: &str) -> Result<()> {
    let story_dir = drone_dir.join("stories").join(story_id);

    if !story_dir.exists() {
        bail!("No logs found for story '{}'", story_id);
    }

    println!(
        "{} {} (Press Ctrl+C to exit)",
        "Following Story Logs:".bright_cyan().bold(),
        story_id.bright_yellow()
    );
    println!();

    // Find latest attempt log
    let mut attempts: Vec<_> = fs::read_dir(&story_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().ok().is_some_and(|t| t.is_file()))
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("log"))
        .collect();

    attempts.sort_by_key(|entry| entry.path());

    let log_path = if let Some(latest) = attempts.last() {
        latest.path()
    } else {
        bail!("No log files found for story '{}'", story_id);
    };

    let mut file = fs::File::open(&log_path)?;
    let mut reader = BufReader::new(&mut file);

    // Read existing content first
    let mut line = String::new();
    while reader.read_line(&mut line).ok().is_some_and(|n| n > 0) {
        print!("{}", line);
        line.clear();
    }

    // Now tail new lines
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // No new data, sleep and try again
                thread::sleep(Duration::from_millis(500));
                // Re-open file to catch new writes
                file = fs::File::open(&log_path)?;
                file.stream_position()?;
                reader = BufReader::new(&mut file);
            }
            Ok(_) => {
                print!("{}", line);
            }
            Err(e) => {
                eprintln!("Error reading log: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn print_colored_line(line: &str) {
    // Colorize log lines based on content
    if line.contains("✅") {
        print!("{}", line.green());
    } else if line.contains("❌") || line.contains("Error") {
        print!("{}", line.red());
    } else if line.contains("🔨") {
        print!("{}", line.bright_blue());
    } else if line.contains("💾") {
        print!("{}", line.bright_yellow());
    } else {
        print!("{}", line);
    }
}

fn show_story_logs(
    drone_dir: &std::path::Path,
    story_id: &str,
    lines: Option<usize>,
) -> Result<()> {
    let story_dir = drone_dir.join("stories").join(story_id);

    if !story_dir.exists() {
        bail!("No logs found for story '{}'", story_id);
    }

    println!(
        "{} {}",
        "Story Logs:".bright_cyan().bold(),
        story_id.bright_yellow()
    );
    println!();

    // List all attempts
    let mut attempts: Vec<_> = fs::read_dir(&story_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().ok().is_some_and(|t| t.is_file()))
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("log"))
        .collect();

    attempts.sort_by_key(|entry| entry.path());

    if attempts.is_empty() {
        println!("{}", "No log files found".yellow());
        return Ok(());
    }

    // Show metadata for each attempt
    for (idx, entry) in attempts.iter().enumerate() {
        let metadata_path = entry.path().with_extension("json");

        if metadata_path.exists() {
            let metadata_content = fs::read_to_string(&metadata_path)?;
            println!("{} Attempt {}", "→".bright_blue(), idx + 1);

            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_content) {
                if let Some(duration) = metadata.get("duration_seconds") {
                    println!("  Duration: {} seconds", duration);
                }
                if let Some(exit_code) = metadata.get("exit_code") {
                    println!("  Exit code: {}", exit_code);
                }
                if let Some(started) = metadata.get("started") {
                    println!("  Started: {}", started);
                }
            }
            println!();
        }
    }

    // Show log content for latest attempt
    if let Some(latest) = attempts.last() {
        let log_path = latest.path();
        println!("{}", "Latest Log:".bright_cyan().bold());
        println!();

        let file = fs::File::open(&log_path)?;
        let reader = BufReader::new(file);

        let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

        let lines_to_show = if let Some(n) = lines {
            if all_lines.len() > n {
                &all_lines[all_lines.len() - n..]
            } else {
                &all_lines
            }
        } else {
            &all_lines
        };

        for line in lines_to_show {
            println!("{}", line);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_show_activity_log() {
        let temp_dir = std::env::temp_dir().join("hive-test-logs");
        let drone_dir = temp_dir.join(".hive/drones/test-drone");

        fs::create_dir_all(&drone_dir).unwrap();

        let log_path = drone_dir.join("activity.log");
        let mut file = fs::File::create(&log_path).unwrap();
        writeln!(file, "[10:00:00] 🔨 Début TEST-001").unwrap();
        writeln!(file, "[10:00:05] ✅ TEST-001 terminée").unwrap();

        let result = show_activity_log(&drone_dir, None);
        assert!(result.is_ok());

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_show_activity_log_with_lines_limit() {
        let temp_dir = std::env::temp_dir().join("hive-test-logs-limit");
        let drone_dir = temp_dir.join(".hive/drones/test-drone");

        fs::create_dir_all(&drone_dir).unwrap();

        let log_path = drone_dir.join("activity.log");
        let mut file = fs::File::create(&log_path).unwrap();
        for i in 0..100 {
            writeln!(file, "Line {}", i).unwrap();
        }

        let result = show_activity_log(&drone_dir, Some(10));
        assert!(result.is_ok());

        fs::remove_dir_all(&temp_dir).ok();
    }
}
