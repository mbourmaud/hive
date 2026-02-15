use super::super::task_sync::TeamTaskInfo;
use super::super::AgentTeamTask;

/// Load tasks from `todos.json` (written by the TodoWrite hook).
///
/// The hook captures `[{content, status, activeForm}]` — real task names and progress
/// from the team lead's TodoWrite calls.
pub fn load_todos(path: &std::path::Path) -> Vec<TeamTaskInfo> {
    #[derive(serde::Deserialize)]
    struct TodoEntry {
        content: String,
        #[serde(default = "default_pending")]
        status: String,
        #[serde(default, rename = "activeForm")]
        active_form: Option<String>,
    }

    fn default_pending() -> String {
        "pending".to_string()
    }

    let contents = match std::fs::read_to_string(path) {
        Ok(c) if !c.trim().is_empty() => c,
        _ => return Vec::new(),
    };

    let entries: Vec<TodoEntry> = match serde_json::from_str(&contents) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .into_iter()
        .enumerate()
        .map(|(i, e)| TeamTaskInfo {
            id: (i + 1).to_string(),
            subject: e.content,
            description: String::new(),
            status: e.status,
            owner: None,
            active_form: e.active_form,
            model: None,
            is_internal: false,
            created_at: None,
            updated_at: None,
            blocked_by: Vec::new(),
        })
        .collect()
}

/// Map an `AgentTeamTask` (from filesystem JSON) to `TeamTaskInfo` (for TUI display).
pub fn map_task(t: AgentTeamTask) -> TeamTaskInfo {
    let is_internal = t
        .metadata
        .as_ref()
        .and_then(|m| m.get("_internal"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    TeamTaskInfo {
        id: t.id,
        subject: t.subject,
        description: t.description,
        status: t.status,
        owner: t.owner,
        active_form: t.active_form,
        model: None,
        is_internal,
        created_at: t.created_at,
        updated_at: t.updated_at,
        blocked_by: t.blocked_by,
    }
}
