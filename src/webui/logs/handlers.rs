use std::path::PathBuf;

use axum::{
    extract::{Path, Query},
    response::sse::{Event, Sse},
};
use serde::Deserialize;

use super::formatter::stream_log_file;

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    #[serde(default)]
    format: Option<String>,
}

pub async fn api_logs_sse(
    Path(name): Path<String>,
    Query(query): Query<LogQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let log_path = PathBuf::from(".hive/drones")
        .join(&name)
        .join("activity.log");
    let raw = query.format.as_deref() == Some("raw");
    stream_log_file(log_path, raw)
}

pub async fn api_logs_project_sse(
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
