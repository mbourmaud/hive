use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        Html, IntoResponse,
    },
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

use crate::agent_teams::snapshot::TaskSnapshotStore;
use crate::commands::common::{
    is_process_running, list_drones, list_drones_at, read_drone_pid, read_drone_pid_at,
};
use crate::commands::monitor::cost::{parse_cost_from_log, parse_cost_from_log_at, CostSummary};
use crate::config;

const EMBEDDED_HTML: &str = include_str!("../../web/dist/index.html");

// ===== API Types =====

#[derive(Debug, Clone, Serialize)]
struct ProjectInfo {
    name: String,
    path: String,
    drones: Vec<DroneInfo>,
    total_cost: f64,
    active_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct DroneInfo {
    name: String,
    status: String,
    branch: String,
    worktree: String,
    lead_model: Option<String>,
    started: String,
    updated: String,
    elapsed: String,
    tasks: Vec<TaskInfo>,
    members: Vec<MemberInfo>,
    messages: Vec<MessageInfo>,
    progress: (usize, usize),
    cost: CostInfo,
    liveness: String,
}

#[derive(Debug, Clone, Serialize)]
struct MessageInfo {
    from: String,
    to: String,
    text: String,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
struct TaskInfo {
    id: String,
    subject: String,
    description: String,
    status: String,
    owner: Option<String>,
    active_form: Option<String>,
    is_internal: bool,
    duration: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MemberInfo {
    name: String,
    agent_type: String,
    model: String,
    liveness: String,
}

#[derive(Debug, Clone, Serialize)]
struct CostInfo {
    total_usd: f64,
    input_tokens: u64,
    output_tokens: u64,
}

// ===== Shared State =====

struct AppState {
    /// Per-project snapshot stores, keyed by project path
    snapshot_stores: Mutex<HashMap<String, TaskSnapshotStore>>,
    tx: broadcast::Sender<String>,
}

// ===== Server =====

pub fn run_server(port: u16) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let (tx, _rx) = broadcast::channel::<String>(256);
        let state = Arc::new(AppState {
            snapshot_stores: Mutex::new(HashMap::new()),
            tx: tx.clone(),
        });

        // Background poller: every 2 seconds, poll all projects and push SSE
        let poll_state = state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let projects = poll_all_projects(&poll_state);
                if let Ok(json) = serde_json::to_string(&projects) {
                    let _ = poll_state.tx.send(json);
                }
            }
        });

        let app = Router::new()
            .route("/", get(serve_index))
            .route("/api/projects", get(api_projects))
            .route("/api/drones", get(api_drones))
            .route("/api/drones/{name}", get(api_drone_detail))
            .route("/api/events", get(api_events_sse))
            .route("/api/logs/{name}", get(api_logs_sse))
            .route("/api/logs/{project_path}/{name}", get(api_logs_project_sse))
            .layer(CorsLayer::permissive())
            .with_state(state);

        println!("Hive WebUI running at http://localhost:{}", port);
        if let Some(ip) = local_ip() {
            println!("  Network: http://{}:{}", ip, port);
        }

        let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await {
            Ok(l) => l,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AddrInUse {
                    anyhow::bail!(
                        "Port {} is already in use. Try a different port with --port <PORT>",
                        port
                    );
                }
                return Err(e.into());
            }
        };
        axum::serve(listener, app).await?;

        Ok(())
    })
}

// ===== Handlers =====

async fn serve_index() -> Html<&'static str> {
    Html(EMBEDDED_HTML)
}

async fn api_projects(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let projects = poll_all_projects(&state);
    Json(projects).into_response()
}

async fn api_drones(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Backward compat: flatten all projects' drones
    let projects = poll_all_projects(&state);
    let all_drones: Vec<DroneInfo> = projects.into_iter().flat_map(|p| p.drones).collect();
    Json(all_drones).into_response()
}

async fn api_drone_detail(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let projects = poll_all_projects(&state);
    let all_drones: Vec<DroneInfo> = projects.into_iter().flat_map(|p| p.drones).collect();
    if let Some(drone) = all_drones.into_iter().find(|d| d.name == name) {
        Json(drone).into_response()
    } else {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Drone not found"})),
        )
            .into_response()
    }
}

async fn api_events_sse(
    State(state): State<Arc<AppState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(data) => Some(Ok(Event::default().data(data))),
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Debug, Deserialize)]
struct LogQuery {
    #[serde(default)]
    format: Option<String>,
}

async fn api_logs_sse(
    Path(name): Path<String>,
    Query(query): Query<LogQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let log_path = std::path::PathBuf::from(".hive/drones")
        .join(&name)
        .join("activity.log");
    let raw = query.format.as_deref() == Some("raw");
    stream_log_file(log_path, raw)
}

async fn api_logs_project_sse(
    Path((project_path, name)): Path<(String, String)>,
    Query(query): Query<LogQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let decoded =
        urlencoding::decode(&project_path).unwrap_or_else(|_| project_path.clone().into());
    let log_path = PathBuf::from(decoded.as_ref())
        .join(".hive/drones")
        .join(&name)
        .join("activity.log");
    let raw = query.format.as_deref() == Some("raw");
    stream_log_file(log_path, raw)
}

/// Format a raw NDJSON activity.log line into a human-readable summary.
/// Returns None for lines that should be skipped (tool results, unparseable).
fn format_log_line(raw: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let typ = v.get("type")?.as_str()?;

    match typ {
        "system" => Some("[init] Session started".to_string()),
        "result" => {
            let subtype = v
                .get("subtype")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");
            let result_text = v
                .get("result")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();
            if result_text.is_empty() {
                Some(format!("[done] {subtype}"))
            } else {
                Some(format!("[done] {subtype} — {result_text}"))
            }
        }
        "user" => {
            // Check if this is a tool_result (skip those)
            let content = v.get("message").and_then(|m| m.get("content"));
            if let Some(arr) = content.and_then(|c| c.as_array()) {
                if arr
                    .iter()
                    .any(|item| item.get("type").and_then(|t| t.as_str()) == Some("tool_result"))
                {
                    return None;
                }
            }
            // Regular user message
            let text = content
                .and_then(|c| {
                    if let Some(s) = c.as_str() {
                        Some(s.to_string())
                    } else if let Some(arr) = c.as_array() {
                        arr.iter()
                            .filter_map(|item| {
                                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                    item.get("text").and_then(|t| t.as_str()).map(String::from)
                                } else {
                                    None
                                }
                            })
                            .next()
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let truncated: String = text.chars().take(200).collect();
            if truncated.is_empty() {
                None
            } else {
                Some(format!("[user] {truncated}"))
            }
        }
        "assistant" => {
            let content = v
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())?;

            let mut parts: Vec<String> = Vec::new();
            for item in content {
                let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match item_type {
                    "tool_use" => {
                        let name = item
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        // Try to extract a meaningful arg (file_path for Read/Write/Edit, command for Bash)
                        let input = item.get("input");
                        let detail = input
                            .and_then(|i| {
                                i.get("file_path")
                                    .or_else(|| i.get("command"))
                                    .or_else(|| i.get("pattern"))
                                    .and_then(|v| v.as_str())
                            })
                            .map(|s| {
                                let truncated: String = s.chars().take(80).collect();
                                truncated
                            });
                        if let Some(d) = detail {
                            parts.push(format!("[tool] {name} {d}"));
                        } else {
                            parts.push(format!("[tool] {name}"));
                        }
                    }
                    "text" => {
                        let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        if !text.trim().is_empty() {
                            let truncated: String = text.chars().take(200).collect();
                            parts.push(format!("[assistant] {truncated}"));
                        }
                    }
                    _ => {}
                }
            }

            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
            }
        }
        _ => None,
    }
}

fn stream_log_file(
    log_path: PathBuf,
    raw: bool,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let stream = async_stream::stream! {
        let mut offset: u64 = 0;

        // If the file already exists, start from the end (last 50 lines)
        if let Ok(contents) = tokio::fs::read_to_string(&log_path).await {
            let lines: Vec<&str> = contents.lines().collect();
            let start = if lines.len() > 50 { lines.len() - 50 } else { 0 };
            for line in &lines[start..] {
                if raw {
                    yield Ok(Event::default().data(line));
                } else if let Some(formatted) = format_log_line(line) {
                    for sub in formatted.lines() {
                        yield Ok(Event::default().data(sub));
                    }
                }
            }
            offset = contents.len() as u64;
        }

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Ok(contents) = tokio::fs::read_to_string(&log_path).await {
                let len = contents.len() as u64;
                if len > offset {
                    let new_data = &contents[offset as usize..];
                    for line in new_data.lines() {
                        if !line.is_empty() {
                            if raw {
                                yield Ok(Event::default().data(line));
                            } else if let Some(formatted) = format_log_line(line) {
                                for sub in formatted.lines() {
                                    yield Ok(Event::default().data(sub));
                                }
                            }
                        }
                    }
                    offset = len;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ===== Polling =====

fn poll_all_projects(state: &AppState) -> Vec<ProjectInfo> {
    // Load registered projects
    let mut project_paths: Vec<(String, String)> = config::load_projects_registry()
        .unwrap_or_default()
        .projects
        .into_iter()
        .map(|p| (p.path, p.name))
        .collect();

    // Always include CWD as backward compat (if not already in list)
    if let Ok(cwd) = std::env::current_dir() {
        let cwd_str = cwd.to_string_lossy().to_string();
        if !project_paths.iter().any(|(p, _)| *p == cwd_str) {
            let name = cwd
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("current")
                .to_string();
            project_paths.push((cwd_str, name));
        }
    }

    let mut stores = state.snapshot_stores.lock().unwrap();
    let mut projects = Vec::new();

    for (project_path, project_name) in &project_paths {
        let root = PathBuf::from(project_path);

        // Skip projects whose .hive/drones doesn't exist
        if !root.join(".hive/drones").exists() {
            continue;
        }

        let drones_list = if root == std::env::current_dir().unwrap_or_default() {
            // Use CWD-relative functions for the current project
            list_drones().unwrap_or_default()
        } else {
            list_drones_at(&root).unwrap_or_default()
        };

        // Get or create the snapshot store for this project
        let store = stores
            .entry(project_path.clone())
            .or_insert_with(|| TaskSnapshotStore::with_project_root(root.clone()));

        let mut drone_infos = Vec::new();

        for (name, status) in &drones_list {
            let snapshot = store.update(name);
            let cost = if root == std::env::current_dir().unwrap_or_default() {
                parse_cost_from_log(name)
            } else {
                parse_cost_from_log_at(&root, name)
            };

            let liveness = if root == std::env::current_dir().unwrap_or_default() {
                determine_liveness(name, &status.status)
            } else {
                determine_liveness_at(&root, name, &status.status)
            };

            let tasks: Vec<TaskInfo> = snapshot
                .tasks
                .iter()
                .map(|t| {
                    let duration = compute_task_duration(t.created_at, t.updated_at, &t.status);
                    TaskInfo {
                        id: t.id.clone(),
                        subject: t.subject.clone(),
                        description: t.description.clone(),
                        status: t.status.clone(),
                        owner: t.owner.clone(),
                        active_form: t.active_form.clone(),
                        is_internal: t.is_internal,
                        duration,
                    }
                })
                .collect();

            let members: Vec<MemberInfo> = snapshot
                .members
                .iter()
                .map(|m| {
                    let member_liveness = determine_member_liveness(&m.name, &tasks);
                    MemberInfo {
                        name: m.name.clone(),
                        agent_type: m.agent_type.clone(),
                        model: m.model.clone(),
                        liveness: member_liveness,
                    }
                })
                .collect();

            let messages = collect_messages(name);
            let elapsed = compute_elapsed(&status.started);

            drone_infos.push(DroneInfo {
                name: name.clone(),
                status: format!("{:?}", status.status).to_lowercase(),
                branch: status.branch.clone(),
                worktree: status.worktree.clone(),
                lead_model: status.lead_model.clone(),
                started: status.started.clone(),
                updated: status.updated.clone(),
                elapsed,
                tasks,
                members,
                messages,
                progress: snapshot.progress,
                cost: cost_to_info(&cost),
                liveness,
            });
        }

        let total_cost: f64 = drone_infos.iter().map(|d| d.cost.total_usd).sum();
        let active_count = drone_infos
            .iter()
            .filter(|d| d.liveness == "working")
            .count();

        projects.push(ProjectInfo {
            name: project_name.clone(),
            path: project_path.clone(),
            drones: drone_infos,
            total_cost,
            active_count,
        });
    }

    projects
}

/// Check if activity.log ends with a successful result event.
fn has_success_result(activity_log_path: &std::path::Path) -> bool {
    let contents = match std::fs::read_to_string(activity_log_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    // Find last non-empty line
    let last_line = contents.lines().rev().find(|l| !l.trim().is_empty());
    let last_line = match last_line {
        Some(l) => l,
        None => return false,
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(last_line) {
        let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if typ == "result" {
            let subtype = v.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
            let is_error = v.get("is_error").and_then(|e| e.as_bool()).unwrap_or(true);
            return subtype == "success" || !is_error;
        }
    }
    false
}

fn determine_liveness(drone_name: &str, status: &crate::types::DroneState) -> String {
    use crate::types::DroneState;
    match status {
        DroneState::Completed => "completed".to_string(),
        DroneState::Stopped => "stopped".to_string(),
        DroneState::Zombie => "dead".to_string(),
        DroneState::InProgress | DroneState::Starting | DroneState::Resuming => {
            let pid_alive = read_drone_pid(drone_name)
                .map(is_process_running)
                .unwrap_or(false);
            if pid_alive {
                "working".to_string()
            } else {
                let log_path = PathBuf::from(".hive/drones")
                    .join(drone_name)
                    .join("activity.log");
                if has_success_result(&log_path) {
                    "completed".to_string()
                } else {
                    "dead".to_string()
                }
            }
        }
        _ => "unknown".to_string(),
    }
}

fn determine_liveness_at(
    project_root: &std::path::Path,
    drone_name: &str,
    status: &crate::types::DroneState,
) -> String {
    use crate::types::DroneState;
    match status {
        DroneState::Completed => "completed".to_string(),
        DroneState::Stopped => "stopped".to_string(),
        DroneState::Zombie => "dead".to_string(),
        DroneState::InProgress | DroneState::Starting | DroneState::Resuming => {
            let pid_alive = read_drone_pid_at(project_root, drone_name)
                .map(is_process_running)
                .unwrap_or(false);
            if pid_alive {
                "working".to_string()
            } else {
                let log_path = project_root
                    .join(".hive/drones")
                    .join(drone_name)
                    .join("activity.log");
                if has_success_result(&log_path) {
                    "completed".to_string()
                } else {
                    "dead".to_string()
                }
            }
        }
        _ => "unknown".to_string(),
    }
}

fn determine_member_liveness(member_name: &str, tasks: &[TaskInfo]) -> String {
    let has_active_task = tasks
        .iter()
        .any(|t| t.owner.as_deref() == Some(member_name) && t.status == "in_progress");

    if has_active_task {
        "working".to_string()
    } else {
        "idle".to_string()
    }
}

fn compute_task_duration(
    created_at: Option<u64>,
    updated_at: Option<u64>,
    status: &str,
) -> Option<String> {
    let start = created_at?;
    let end = if status == "completed" {
        updated_at?
    } else if status == "in_progress" {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as u64
    } else {
        return None;
    };

    if end <= start {
        return None;
    }

    let secs = (end - start) / 1000;
    Some(format_duration_secs(secs))
}

fn compute_elapsed(started: &str) -> String {
    use crate::commands::common::elapsed_since;
    elapsed_since(started).unwrap_or_else(|| "?".to_string())
}

fn format_duration_secs(total_secs: u64) -> String {
    crate::commands::common::format_duration(chrono::Duration::seconds(total_secs as i64))
}

fn collect_messages(drone_name: &str) -> Vec<MessageInfo> {
    use crate::agent_teams::task_sync;

    let inboxes = task_sync::read_team_inboxes(drone_name).unwrap_or_default();
    let mut messages: Vec<MessageInfo> = Vec::new();

    for (recipient, inbox) in &inboxes {
        for msg in inbox {
            // Skip empty or system messages
            if msg.text.trim().is_empty() {
                continue;
            }
            // Skip JSON protocol messages (idle, shutdown, etc.)
            if msg.text.trim_start().starts_with('{') {
                continue;
            }
            messages.push(MessageInfo {
                from: msg.from.clone(),
                to: recipient.clone(),
                text: msg.text.clone(),
                timestamp: msg.timestamp.clone(),
            });
        }
    }

    // Sort by timestamp
    messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    messages
}

/// Detect the machine's LAN IP address by opening a UDP socket to a public address.
/// No traffic is actually sent — this just triggers the OS to pick a local interface.
fn local_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

fn cost_to_info(cost: &CostSummary) -> CostInfo {
    CostInfo {
        total_usd: cost.total_cost_usd,
        input_tokens: cost.input_tokens,
        output_tokens: cost.output_tokens,
    }
}
