use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub drones: Vec<DroneInfo>,
    pub total_cost: f64,
    pub active_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DroneInfo {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub branch: String,
    pub worktree: String,
    pub lead_model: Option<String>,
    pub started: String,
    pub updated: String,
    pub elapsed: String,
    pub tasks: Vec<TaskInfo>,
    pub members: Vec<MemberInfo>,
    pub messages: Vec<MessageInfo>,
    pub progress: (usize, usize),
    pub cost: CostInfo,
    pub liveness: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageInfo {
    pub from: String,
    pub to: String,
    pub text: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskInfo {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub status: String,
    pub owner: Option<String>,
    pub active_form: Option<String>,
    pub is_internal: bool,
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemberInfo {
    pub name: String,
    pub agent_type: String,
    pub model: String,
    pub liveness: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostInfo {
    pub total_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}
