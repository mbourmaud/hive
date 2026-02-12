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
    let mut parallel = false;
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
mod tests {
    use super::*;

    #[test]
    fn test_parse_task_heading_basic() {
        assert_eq!(
            parse_task_heading("### 1. Set up environment"),
            Some((1, "Set up environment".to_string()))
        );
        assert_eq!(
            parse_task_heading("### 12. Write tests"),
            Some((12, "Write tests".to_string()))
        );
    }

    #[test]
    fn test_parse_task_heading_invalid() {
        assert_eq!(parse_task_heading("## Not a task"), None);
        assert_eq!(parse_task_heading("### No number"), None);
        assert_eq!(parse_task_heading("### 1."), None); // empty title
        assert_eq!(parse_task_heading("Regular text"), None);
    }

    #[test]
    fn test_parse_full_structured_plan() {
        let content = r#"# Fix authentication system

## Goal
Refactor the authentication module to support OAuth2.

## Tasks

### 1. Set up environment
- type: setup

### 2. Implement OAuth2 provider
- model: sonnet
- parallel: true
- files: src/auth/oauth.ts, src/auth/provider.ts

Implement the OAuth2 provider class with support for Google and GitHub.

### 3. Update API routes
- model: sonnet
- parallel: true
- files: src/routes/auth.ts
- depends_on: 2

### 4. Write tests
- model: haiku
- depends_on: 2, 3

### 5. Create PR/MR
- type: pr
- depends_on: 2, 3, 4

## Definition of Done
- [ ] OAuth2 works
- [ ] Tests pass
"#;
        let tasks = parse_tasks(content);
        assert_eq!(tasks.len(), 5);

        // Task 1: setup
        assert_eq!(tasks[0].number, 1);
        assert_eq!(tasks[0].title, "Set up environment");
        assert_eq!(tasks[0].task_type, TaskType::Setup);

        // Task 2: work with model + parallel + files
        assert_eq!(tasks[1].number, 2);
        assert_eq!(tasks[1].title, "Implement OAuth2 provider");
        assert_eq!(tasks[1].task_type, TaskType::Work);
        assert_eq!(tasks[1].model, Some("sonnet".to_string()));
        assert!(tasks[1].parallel);
        assert_eq!(
            tasks[1].files,
            vec!["src/auth/oauth.ts", "src/auth/provider.ts"]
        );
        assert!(tasks[1].body.contains("Implement the OAuth2 provider"));

        // Task 3: depends_on
        assert_eq!(tasks[2].depends_on, vec![2]);
        assert!(tasks[2].parallel);

        // Task 4: haiku model, multiple deps
        assert_eq!(tasks[3].model, Some("haiku".to_string()));
        assert_eq!(tasks[3].depends_on, vec![2, 3]);

        // Task 5: PR type
        assert_eq!(tasks[4].task_type, TaskType::Pr);
        assert_eq!(tasks[4].depends_on, vec![2, 3, 4]);
    }

    #[test]
    fn test_parse_bullet_list_tasks_returns_empty() {
        let content = r#"# Simple plan

## Goal
Do something simple.

## Tasks
- Install deps
- Write code
- Test it

## Definition of Done
- [ ] It works
"#;
        assert!(parse_tasks(content).is_empty());
    }

    #[test]
    fn test_parse_no_tasks_section_returns_empty() {
        let content = r#"# Plan without tasks section

## Goal
Do something.

## Steps
1. First step
2. Second step
"#;
        assert!(parse_tasks(content).is_empty());
    }

    #[test]
    fn test_parse_task_with_no_metadata() {
        let content = r#"## Tasks

### 1. Do the thing

Just do it. No metadata needed.

### 2. Do another thing

Also straightforward.
"#;
        let tasks = parse_tasks(content);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].task_type, TaskType::Work);
        assert!(tasks[0].model.is_none());
        assert!(!tasks[0].parallel);
        assert!(tasks[0].depends_on.is_empty());
        assert!(tasks[0].body.contains("Just do it"));
    }

    #[test]
    fn test_parse_task_metadata_case_insensitive() {
        let content = r#"## Tasks

### 1. Setup
- type: SETUP
- model: Sonnet
- parallel: TRUE
"#;
        let tasks = parse_tasks(content);
        assert_eq!(tasks[0].task_type, TaskType::Setup);
        // model preserves original case
        assert_eq!(tasks[0].model, Some("Sonnet".to_string()));
        assert!(tasks[0].parallel);
    }

    #[test]
    fn test_parse_tasks_section_ends_at_next_h2() {
        let content = r#"## Tasks

### 1. Only task
- model: sonnet

Do the work.

## Definition of Done
- [ ] It works
"#;
        let tasks = parse_tasks(content);
        assert_eq!(tasks.len(), 1);
        // Body should not include "Definition of Done"
        assert!(!tasks[0].body.contains("Definition of Done"));
    }

    #[test]
    fn test_parse_tasks_case_insensitive_heading() {
        let content = r#"## tasks

### 1. My task
Simple task.
"#;
        let tasks = parse_tasks(content);
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn test_parse_mixed_metadata_and_body_bullets() {
        let content = r#"## Tasks

### 1. Implement feature
- model: sonnet
- files: src/main.rs

- Install the dependency
- Write the implementation
- Handle edge cases
"#;
        let tasks = parse_tasks(content);
        assert_eq!(tasks[0].model, Some("sonnet".to_string()));
        assert_eq!(tasks[0].files, vec!["src/main.rs"]);
        // Body bullets should be preserved
        assert!(tasks[0].body.contains("Install the dependency"));
        assert!(tasks[0].body.contains("Handle edge cases"));
    }
}
