use crate::types::{StructuredTask, TaskType};

/// Parse the `## Tasks` section from a markdown plan into structured tasks.
///
/// Returns an empty vec if no `## Tasks` section or `### N. Title` subsections found.
pub fn parse_tasks(content: &str) -> Vec<StructuredTask> {
    let lines: Vec<&str> = content.lines().collect();

    // Find the `## Tasks` heading
    let Some(tasks_start) = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("## tasks"))
    else {
        return Vec::new();
    };

    // Find the end of the ## Tasks section (next ## heading or EOF)
    let tasks_end = lines
        .iter()
        .enumerate()
        .skip(tasks_start + 1)
        .find(|(_, line)| {
            let trimmed = line.trim();
            trimmed.starts_with("## ") && !trimmed.starts_with("### ")
        })
        .map(|(i, _)| i)
        .unwrap_or(lines.len());

    let task_lines = &lines[tasks_start + 1..tasks_end];

    // Split on ### N. headings
    let mut task_ranges: Vec<(usize, &str)> = Vec::new();

    for (i, line) in task_lines.iter().enumerate() {
        if let Some((number, title)) = parse_task_heading(line) {
            task_ranges.push((i, *line));
            let _ = (number, title); // used below in full parse
        }
    }

    if task_ranges.is_empty() {
        return Vec::new();
    }

    let mut tasks = Vec::new();

    for (idx, &(start_offset, _)) in task_ranges.iter().enumerate() {
        let end_offset = if idx + 1 < task_ranges.len() {
            task_ranges[idx + 1].0
        } else {
            task_lines.len()
        };

        let heading_line = task_lines[start_offset];
        let (number, title) = parse_task_heading(heading_line).unwrap();

        let body_lines = &task_lines[start_offset + 1..end_offset];
        let task = parse_single_task(number, title, body_lines);
        tasks.push(task);
    }

    tasks
}

/// Parse a `### N. Title` heading, returning (number, title) or None.
fn parse_task_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("### ")?;

    // Find the number before the first dot
    let dot_pos = rest.find('.')?;
    let num_str = rest[..dot_pos].trim();
    let number: usize = num_str.parse().ok()?;

    let title = rest[dot_pos + 1..].trim().to_string();

    if title.is_empty() {
        return None;
    }

    Some((number, title))
}

/// Parse a single task's body lines into a StructuredTask.
fn parse_single_task(number: usize, title: String, lines: &[&str]) -> StructuredTask {
    let mut task_type = TaskType::Work;
    let mut model = None;
    let mut parallel = true;
    let mut files = Vec::new();
    let mut depends_on = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_metadata = true;

    for line in lines {
        let trimmed = line.trim();

        // Empty lines end the metadata section
        if trimmed.is_empty() {
            if in_metadata {
                in_metadata = false;
            }
            body_lines.push(*line);
            continue;
        }

        // Metadata bullets: `- key: value`
        if in_metadata {
            if let Some(rest) = trimmed.strip_prefix("- ") {
                if let Some((key, value)) = rest.split_once(':') {
                    let key = key.trim().to_lowercase();
                    let value = value.trim();

                    match key.as_str() {
                        "type" => {
                            task_type = match value.to_lowercase().as_str() {
                                "setup" => TaskType::Setup,
                                "pr" => TaskType::Pr,
                                _ => TaskType::Work,
                            };
                            continue;
                        }
                        "model" => {
                            model = Some(value.to_string());
                            continue;
                        }
                        "parallel" => {
                            parallel = value.to_lowercase() == "true";
                            continue;
                        }
                        "files" => {
                            files = value
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            continue;
                        }
                        "depends_on" => {
                            depends_on = value
                                .split(',')
                                .filter_map(|s| s.trim().parse::<usize>().ok())
                                .collect();
                            continue;
                        }
                        _ => {} // Not a recognized metadata key — treat as body
                    }
                }
            }
            // Non-metadata bullet or text after metadata → switch to body
            in_metadata = false;
        }

        body_lines.push(*line);
    }

    // Trim leading/trailing empty lines from body
    let body = body_lines.to_vec().join("\n");
    let body = body.trim().to_string();

    StructuredTask {
        number,
        title,
        body,
        task_type,
        model,
        parallel,
        files,
        depends_on,
    }
}

#[cfg(test)]
mod tests;
