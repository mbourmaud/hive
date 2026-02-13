use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

// ===== Types =====

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Idle,
    Busy,
    Completed,
    #[allow(dead_code)]
    Error(String),
}

pub struct ChatSession {
    pub id: String,
    pub process: Child,
    pub stdin: tokio::process::ChildStdin,
    pub cwd: PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: SessionStatus,
    pub tx: broadcast::Sender<String>,
    pub title: Option<String>,
}

pub type SessionStore = Arc<Mutex<HashMap<String, ChatSession>>>;

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub cwd: String,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub status: SessionStatus,
    pub cwd: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListItem {
    pub id: String,
    pub status: String,
    pub cwd: String,
    pub created_at: String,
    pub updated_at: String,
    pub title: String,
}

/// Metadata persisted to .hive/sessions/{id}/meta.json
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMeta {
    id: String,
    cwd: String,
    created_at: String,
    updated_at: String,
    status: String,
    title: String,
}

// ===== Persistence helpers =====

fn sessions_dir() -> PathBuf {
    PathBuf::from(".hive/sessions")
}

fn session_dir(id: &str) -> PathBuf {
    sessions_dir().join(id)
}

fn ensure_session_dir(id: &str) -> std::io::Result<PathBuf> {
    let dir = session_dir(id);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn append_event(id: &str, line: &str) {
    if let Ok(dir) = ensure_session_dir(id) {
        let path = dir.join("events.ndjson");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            use std::io::Write;
            let _ = writeln!(f, "{}", line);
        }
    }
}

fn write_meta(meta: &SessionMeta) {
    if let Ok(dir) = ensure_session_dir(&meta.id) {
        let path = dir.join("meta.json");
        if let Ok(json) = serde_json::to_string_pretty(meta) {
            let _ = std::fs::write(path, json);
        }
    }
}

fn read_meta(id: &str) -> Option<SessionMeta> {
    let path = session_dir(id).join("meta.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn extract_title(text: &str) -> String {
    let cleaned: String = text
        .lines()
        .next()
        .unwrap_or(text)
        .trim()
        .chars()
        .take(60)
        .collect();
    if cleaned.is_empty() {
        "Untitled".to_string()
    } else {
        cleaned
    }
}

fn update_meta_status(id: &str, status: &str) {
    if let Some(mut meta) = read_meta(id) {
        meta.status = status.to_string();
        meta.updated_at = chrono::Utc::now().to_rfc3339();
        write_meta(&meta);
    }
}

// ===== Handlers =====

/// POST /api/chat/sessions — spawn a new Claude CLI session
pub async fn create_session(
    State(store): State<SessionStore>,
    Json(body): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let cwd = PathBuf::from(&body.cwd);
    if !cwd.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "cwd is not a valid directory"})),
        )
            .into_response();
    }

    let id = uuid::Uuid::new_v4().to_string();

    let mut child = match Command::new("claude")
        .arg("-p")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .current_dir(&cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to spawn claude: {e}")})),
            )
                .into_response();
        }
    };

    let stdin = child.stdin.take().expect("stdin was piped");
    let stdout = child.stdout.take().expect("stdout was piped");

    let (tx, _rx) = broadcast::channel::<String>(512);
    let now = chrono::Utc::now();

    // Persist initial metadata
    let meta = SessionMeta {
        id: id.clone(),
        cwd: cwd.to_string_lossy().to_string(),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        status: "idle".to_string(),
        title: "New session".to_string(),
    };
    write_meta(&meta);

    let session = ChatSession {
        id: id.clone(),
        process: child,
        stdin,
        cwd: cwd.clone(),
        created_at: now,
        status: SessionStatus::Idle,
        tx: tx.clone(),
        title: None,
    };

    store.lock().await.insert(id.clone(), session);

    // Spawn background task to read stdout lines and broadcast them
    let store_bg = store.clone();
    let session_id = id.clone();
    tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            // Persist event to disk
            append_event(&session_id, &line);
            // Broadcast the raw NDJSON line to all SSE subscribers
            let _ = tx.send(line);
        }

        // Process exited — send a completion event and update status
        let completion = serde_json::json!({"type": "session.completed"}).to_string();
        append_event(&session_id, &completion);
        let _ = tx.send(completion);

        // Update persisted metadata
        update_meta_status(&session_id, "completed");

        let mut sessions = store_bg.lock().await;
        if let Some(s) = sessions.get_mut(&session_id) {
            if s.status == SessionStatus::Busy || s.status == SessionStatus::Idle {
                s.status = SessionStatus::Completed;
            }
        }
    });

    let resp = SessionResponse {
        id,
        status: SessionStatus::Idle,
        cwd: cwd.to_string_lossy().to_string(),
        created_at: now.to_rfc3339(),
    };

    (StatusCode::CREATED, Json(serde_json::json!(resp))).into_response()
}

/// GET /api/chat/sessions/{id}/stream — SSE stream of Claude output
pub async fn stream_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let sessions = store.lock().await;
    let session = match sessions.get(&id) {
        Some(s) => s,
        None => {
            // Return an empty SSE stream with an error event for 404
            let stream = async_stream::stream! {
                yield Ok::<_, std::convert::Infallible>(
                    Event::default().event("error").data("session not found")
                );
            };
            return Sse::new(stream)
                .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(30)))
                .into_response();
        }
    };

    let rx = session.tx.subscribe();
    drop(sessions);

    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(data) => Some(Ok::<_, std::convert::Infallible>(
            Event::default().data(data),
        )),
        Err(_) => None,
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(30)))
        .into_response()
}

/// POST /api/chat/sessions/{id}/message — send a message to Claude's stdin
pub async fn send_message(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let mut sessions = store.lock().await;
    let session = match sessions.get_mut(&id) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "session not found"})),
            )
                .into_response();
        }
    };

    if session.status == SessionStatus::Completed {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "session has completed"})),
        )
            .into_response();
    }

    // Set title from first user message
    if session.title.is_none() {
        let title = extract_title(&body.text);
        session.title = Some(title.clone());
        if let Some(mut meta) = read_meta(&id) {
            meta.title = title;
            meta.updated_at = chrono::Utc::now().to_rfc3339();
            meta.status = "busy".to_string();
            write_meta(&meta);
        }
    } else {
        update_meta_status(&id, "busy");
    }

    let message = format!("{}\n", body.text);
    if let Err(e) = session.stdin.write_all(message.as_bytes()).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to write to stdin: {e}")})),
        )
            .into_response();
    }

    session.status = SessionStatus::Busy;

    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

/// POST /api/chat/sessions/{id}/abort — send SIGINT to Claude process
pub async fn abort_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let sessions = store.lock().await;
    let session = match sessions.get(&id) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "session not found"})),
            )
                .into_response();
        }
    };

    if let Some(pid) = session.process.id() {
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        let _ = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGINT);
    }

    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

/// DELETE /api/chat/sessions/{id} — kill process and remove session
pub async fn delete_session(
    State(store): State<SessionStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut sessions = store.lock().await;
    let mut session = match sessions.remove(&id) {
        Some(s) => s,
        None => {
            // Session not in memory — try to delete from disk
            let dir = session_dir(&id);
            if dir.exists() {
                let _ = std::fs::remove_dir_all(&dir);
                return (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response();
            }
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "session not found"})),
            )
                .into_response();
        }
    };

    // Kill the process
    let _ = session.process.kill().await;

    // Remove persisted session data
    let dir = session_dir(&id);
    if dir.exists() {
        let _ = std::fs::remove_dir_all(&dir);
    }

    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

/// GET /api/chat/sessions — list all sessions (in-memory + persisted on disk)
pub async fn list_sessions(State(store): State<SessionStore>) -> impl IntoResponse {
    let sessions = store.lock().await;
    let mut items: Vec<SessionListItem> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // In-memory sessions first (most current)
    for s in sessions.values() {
        let title = s.title.clone().unwrap_or_else(|| "New session".to_string());
        let status = match &s.status {
            SessionStatus::Idle => "idle",
            SessionStatus::Busy => "busy",
            SessionStatus::Completed => "completed",
            SessionStatus::Error(_) => "error",
        };
        items.push(SessionListItem {
            id: s.id.clone(),
            status: status.to_string(),
            cwd: s.cwd.to_string_lossy().to_string(),
            created_at: s.created_at.to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            title,
        });
        seen_ids.insert(s.id.clone());
    }

    // Persisted sessions from disk
    let dir = sessions_dir();
    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let id = match path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };
                if seen_ids.contains(&id) {
                    continue;
                }
                if let Some(meta) = read_meta(&id) {
                    items.push(SessionListItem {
                        id: meta.id,
                        status: meta.status,
                        cwd: meta.cwd,
                        created_at: meta.created_at,
                        updated_at: meta.updated_at,
                        title: meta.title,
                    });
                }
            }
        }
    }

    // Sort newest first
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Json(items).into_response()
}

/// GET /api/chat/sessions/{id}/history — replay persisted events
pub async fn session_history(Path(id): Path<String>) -> impl IntoResponse {
    let events_path = session_dir(&id).join("events.ndjson");
    match std::fs::read_to_string(&events_path) {
        Ok(contents) => {
            let events: Vec<serde_json::Value> = contents
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            Json(serde_json::json!({ "events": events })).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "no history found for this session"})),
        )
            .into_response(),
    }
}
