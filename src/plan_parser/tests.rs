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
