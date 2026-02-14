pub mod anthropic;
pub mod auth;
pub mod chat;
pub mod error;
pub mod extractors;
pub mod logs;
pub mod mcp_client;
pub mod monitor;
pub mod projects;
pub mod status;
pub mod tools;

use anyhow::Result;
use axum::{response::Html, routing::get, Router};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use monitor::MonitorState;

const EMBEDDED_HTML: &str = include_str!("../../web/dist/index.html");

pub fn run_server(port: u16) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(start_server_async(port))
}

/// Async version of `run_server` for embedding in an existing tokio runtime (e.g. Tauri).
pub async fn start_server_async(port: u16) -> Result<()> {
    let (tx, _rx) = broadcast::channel::<String>(256);
    let chat_sessions: chat::SessionStore = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    let monitor_state = Arc::new(MonitorState {
        snapshot_stores: std::sync::Mutex::new(HashMap::new()),
        tx: tx.clone(),
    });

    // Background poller: every 2s, poll all projects and push SSE
    monitor::spawn_poller(monitor_state.clone());

    let app = Router::new()
        .route("/", get(serve_index))
        .merge(monitor::routes(monitor_state.clone()))
        .merge(logs::routes())
        .merge(auth::routes())
        .merge(chat::routes(chat_sessions.clone()))
        .merge(status::routes(chat_sessions, monitor_state))
        .merge(projects::routes())
        .fallback(get(serve_index))
        .layer(CorsLayer::permissive());

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
}

async fn serve_index() -> Html<&'static str> {
    Html(EMBEDDED_HTML)
}

/// Detect the machine's LAN IP address by opening a UDP socket to a public address.
fn local_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}
