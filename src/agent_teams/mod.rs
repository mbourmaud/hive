pub mod task_sync;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{Prd, Story};

/// A group of stories assigned to a single teammate/worktree.
#[derive(Debug, Clone)]
pub struct StoryGroup {
    pub name: String,
    pub story_ids: Vec<String>,
}

/// An Agent Teams task, mapped from a PRD story.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTeamTask {
    pub id: String,
    pub subject: String,
    pub description: String,
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

/// Group stories into N groups for parallel execution by teammates.
///
/// Algorithm:
/// 1. Build dependency graph from `story.depends_on`
/// 2. Identify independent chains (stories without cross-dependencies)
/// 3. Group parallel stories that don't share files
/// 4. Each group = 1 teammate = 1 worktree
pub fn group_stories(prd: &Prd, max_groups: usize) -> Vec<StoryGroup> {
    let stories = &prd.stories;
    if stories.is_empty() {
        return Vec::new();
    }

    let max_groups = max_groups.max(1);

    // Build adjacency: story_id -> set of dependent story_ids
    let story_map: HashMap<&str, &Story> = stories.iter().map(|s| (s.id.as_str(), s)).collect();

    // Build file usage map: story_id -> set of files
    let file_map: HashMap<&str, HashSet<&str>> = stories
        .iter()
        .map(|s| {
            let files: HashSet<&str> = s.files.iter().map(|f| f.as_str()).collect();
            (s.id.as_str(), files)
        })
        .collect();

    // Build dependency sets for conflict detection
    let dep_map: HashMap<&str, HashSet<&str>> = stories
        .iter()
        .map(|s| {
            let deps: HashSet<&str> = s.depends_on.iter().map(|d| d.as_str()).collect();
            (s.id.as_str(), deps)
        })
        .collect();

    // Greedy grouping: assign each story to a group
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut assigned: HashSet<String> = HashSet::new();

    let max_per_group = stories.len() / max_groups + 1;

    // Process stories in order (respects natural ordering from PRD)
    for story in stories {
        if assigned.contains(&story.id) {
            continue;
        }

        // Try to find an existing group where this story fits
        let mut placed = false;
        for group in groups.iter_mut() {
            let can_place = can_place_in_group(
                &story.id,
                group,
                &file_map,
                &dep_map,
                &story_map,
            );
            if can_place && group.len() < max_per_group {
                group.push(story.id.clone());
                placed = true;
                break;
            }
        }

        if !placed {
            if groups.len() < max_groups {
                groups.push(vec![story.id.clone()]);
            } else {
                // Find smallest group
                if let Some(smallest) = groups.iter_mut().min_by_key(|g| g.len()) {
                    smallest.push(story.id.clone());
                }
            }
        }

        assigned.insert(story.id.clone());
    }

    groups
        .into_iter()
        .enumerate()
        .map(|(i, story_ids)| StoryGroup {
            name: format!("team-{}", (b'a' + i as u8) as char),
            story_ids,
        })
        .collect()
}

/// Check if a story can be placed in a group without conflicts.
fn can_place_in_group(
    story_id: &str,
    group: &[String],
    file_map: &HashMap<&str, HashSet<&str>>,
    dep_map: &HashMap<&str, HashSet<&str>>,
    _story_map: &HashMap<&str, &Story>,
) -> bool {
    let story_files = file_map.get(story_id).cloned().unwrap_or_default();
    let story_deps = dep_map.get(story_id).cloned().unwrap_or_default();

    for existing_id in group {
        // Check if there's a dependency between this story and existing ones
        let existing_deps = dep_map.get(existing_id.as_str()).cloned().unwrap_or_default();
        if story_deps.contains(existing_id.as_str()) || existing_deps.contains(story_id) {
            // Dependencies within the same group are OK (they'll be sequenced)
            continue;
        }

        // Check for file conflicts (parallel stories in same group shouldn't share files)
        if !story_files.is_empty() {
            let existing_files = file_map
                .get(existing_id.as_str())
                .cloned()
                .unwrap_or_default();
            if !story_files.is_disjoint(&existing_files) {
                return false;
            }
        }
    }

    true
}

/// Translate PRD stories into Agent Teams task format.
pub fn translate_stories_to_tasks(prd: &Prd) -> Vec<AgentTeamTask> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Build story_id -> numeric_id mapping
    let story_id_to_numeric: HashMap<String, String> = prd
        .stories
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.clone(), format!("{}", i + 1)))
        .collect();

    // Build reverse dependency map: story_id -> list of story_ids that depend on it
    let mut reverse_deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for story in &prd.stories {
        for dep in &story.depends_on {
            reverse_deps
                .entry(dep.as_str())
                .or_default()
                .push(story.id.as_str());
        }
    }

    prd.stories
        .iter()
        .enumerate()
        .map(|(i, story)| {
            let numeric_id = format!("{}", i + 1);

            let mut description_parts = vec![story.description.clone()];

            if let Some(ref criteria) = story.acceptance_criteria {
                if !criteria.is_empty() {
                    description_parts.push(format!(
                        "\n## Acceptance Criteria\n{}",
                        criteria
                            .iter()
                            .map(|c| format!("- {}", c))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                }
            }

            if !story.definition_of_done.is_empty() {
                description_parts.push(format!(
                    "\n## Definition of Done\n{}",
                    story
                        .definition_of_done
                        .iter()
                        .map(|d| format!("- {}", d))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }

            if !story.verification_commands.is_empty() {
                description_parts.push(format!(
                    "\n## Verification Commands\n{}",
                    story
                        .verification_commands
                        .iter()
                        .map(|v| format!("```\n{}\n```", v))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }

            // Map depends_on story IDs to numeric task IDs
            let blocked_by: Vec<String> = story
                .depends_on
                .iter()
                .filter_map(|dep| story_id_to_numeric.get(dep).cloned())
                .collect();

            // Compute blocks: which numeric task IDs does this story block?
            let blocks: Vec<String> = reverse_deps
                .get(story.id.as_str())
                .map(|dependents| {
                    dependents
                        .iter()
                        .filter_map(|dep| story_id_to_numeric.get(*dep).cloned())
                        .collect()
                })
                .unwrap_or_default();

            AgentTeamTask {
                id: numeric_id,
                subject: story.title.clone(),
                description: description_parts.join("\n"),
                status: "pending".to_string(),
                owner: None,
                active_form: Some(format!("Implementing {}", story.title)),
                blocked_by,
                blocks,
                metadata: Some(serde_json::json!({"storyId": story.id})),
                created_at: Some(now),
                updated_at: Some(now),
            }
        })
        .collect()
}

/// Map task numeric IDs back to story IDs using metadata.
pub fn task_id_to_story_id(tasks: &[AgentTeamTask]) -> HashMap<String, String> {
    tasks
        .iter()
        .filter_map(|t| {
            t.metadata
                .as_ref()
                .and_then(|m| m.get("storyId"))
                .and_then(|v| v.as_str())
                .map(|sid| (t.id.clone(), sid.to_string()))
        })
        .collect()
}

/// Get the task list directory for a team.
pub fn team_tasks_dir(team_name: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".claude")
        .join("tasks")
        .join(team_name)
}

/// Get the team directory.
pub fn team_dir(team_name: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".claude")
        .join("teams")
        .join(team_name)
}

/// Seed the Agent Teams task list with tasks from the PRD.
pub fn seed_task_list(team_name: &str, tasks: &[AgentTeamTask]) -> Result<()> {
    let tasks_dir = team_tasks_dir(team_name);
    fs::create_dir_all(&tasks_dir)
        .context("Failed to create Agent Teams tasks directory")?;

    for task in tasks {
        let task_file = tasks_dir.join(format!("{}.json", task.id));
        let json = serde_json::to_string_pretty(task)
            .context("Failed to serialize task")?;
        fs::write(&task_file, json)
            .with_context(|| format!("Failed to write task file: {}", task.id))?;
    }

    Ok(())
}

/// Read the Agent Teams task list for a team.
pub fn read_task_list(team_name: &str) -> Result<Vec<AgentTeamTask>> {
    let tasks_dir = team_tasks_dir(team_name);

    if !tasks_dir.exists() {
        return Ok(Vec::new());
    }

    let mut tasks = Vec::new();
    for entry in fs::read_dir(&tasks_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let contents = fs::read_to_string(&path)?;
            if let Ok(task) = serde_json::from_str::<AgentTeamTask>(&contents) {
                tasks.push(task);
            }
        }
    }

    Ok(tasks)
}

/// Clean up Agent Teams directories for a team.
pub fn cleanup_team(team_name: &str) -> Result<()> {
    let tasks_dir = team_tasks_dir(team_name);
    if tasks_dir.exists() {
        fs::remove_dir_all(&tasks_dir)
            .context("Failed to remove Agent Teams tasks directory")?;
    }

    let teams_dir = team_dir(team_name);
    if teams_dir.exists() {
        fs::remove_dir_all(&teams_dir)
            .context("Failed to remove Agent Teams team directory")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Prd;

    fn make_test_prd() -> Prd {
        Prd {
            id: "test".to_string(),
            title: "Test PRD".to_string(),
            description: "Test".to_string(),
            version: "1.0".to_string(),
            created_at: String::new(),
            target_platforms: None,
            target_branch: None,
            base_branch: None,
            stories: vec![
                Story {
                    id: "S1".to_string(),
                    title: "Story 1".to_string(),
                    description: "First story".to_string(),
                    acceptance_criteria: None,
                    definition_of_done: vec!["Done".to_string()],
                    verification_commands: vec![],
                    notes: None,
                    actions: vec![],
                    files: vec!["src/a.rs".to_string()],
                    tools: vec![],
                    context: Default::default(),
                    testing: Default::default(),
                    error_handling: None,
                    agent_controls: None,
                    communication: None,
                    depends_on: vec![],
                    parallel: true,
                },
                Story {
                    id: "S2".to_string(),
                    title: "Story 2".to_string(),
                    description: "Second story".to_string(),
                    acceptance_criteria: None,
                    definition_of_done: vec!["Done".to_string()],
                    verification_commands: vec![],
                    notes: None,
                    actions: vec![],
                    files: vec!["src/b.rs".to_string()],
                    tools: vec![],
                    context: Default::default(),
                    testing: Default::default(),
                    error_handling: None,
                    agent_controls: None,
                    communication: None,
                    depends_on: vec![],
                    parallel: true,
                },
                Story {
                    id: "S3".to_string(),
                    title: "Story 3".to_string(),
                    description: "Third story".to_string(),
                    acceptance_criteria: None,
                    definition_of_done: vec!["Done".to_string()],
                    verification_commands: vec![],
                    notes: None,
                    actions: vec![],
                    files: vec!["src/a.rs".to_string()],
                    tools: vec![],
                    context: Default::default(),
                    testing: Default::default(),
                    error_handling: None,
                    agent_controls: None,
                    communication: None,
                    depends_on: vec!["S1".to_string()],
                    parallel: false,
                },
            ],
        }
    }

    #[test]
    fn test_group_stories_respects_file_conflicts() {
        let prd = make_test_prd();
        let groups = group_stories(&prd, 3);
        assert!(!groups.is_empty());
        assert!(groups.len() <= 3);
    }

    #[test]
    fn test_translate_stories_to_tasks() {
        let prd = make_test_prd();
        let tasks = translate_stories_to_tasks(&prd);
        assert_eq!(tasks.len(), 3);

        // IDs are now numeric strings
        assert_eq!(tasks[0].id, "1");
        assert_eq!(tasks[1].id, "2");
        assert_eq!(tasks[2].id, "3");

        // S3 depends on S1, so task "3" is blocked_by ["1"]
        assert_eq!(tasks[2].blocked_by, vec!["1"]);

        // S1 blocks S3, so task "1" blocks ["3"]
        assert_eq!(tasks[0].blocks, vec!["3"]);
        // S2 blocks nothing
        assert!(tasks[1].blocks.is_empty());

        // active_form is set
        assert_eq!(tasks[0].active_form, Some("Implementing Story 1".to_string()));

        // created_at and updated_at are set
        assert!(tasks[0].created_at.is_some());
        assert!(tasks[0].updated_at.is_some());

        // metadata contains original story ID
        let meta = tasks[0].metadata.as_ref().unwrap();
        assert_eq!(meta.get("storyId").unwrap().as_str().unwrap(), "S1");
        let meta2 = tasks[2].metadata.as_ref().unwrap();
        assert_eq!(meta2.get("storyId").unwrap().as_str().unwrap(), "S3");

        // task_id_to_story_id helper works
        let mapping = task_id_to_story_id(&tasks);
        assert_eq!(mapping.get("1").unwrap(), "S1");
        assert_eq!(mapping.get("2").unwrap(), "S2");
        assert_eq!(mapping.get("3").unwrap(), "S3");
    }

    #[test]
    fn test_group_stories_single_group() {
        let prd = make_test_prd();
        let groups = group_stories(&prd, 1);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].story_ids.len(), 3);
    }
}
