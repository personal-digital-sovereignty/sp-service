// =============================================================================
// Sovereign Pair — Coding Module API
// =============================================================================
// File System operations + Terminal WebSocket + LLM Completions
// for the sp-ui-coding micro-frontend.
// =============================================================================

use axum::{
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
};
#[cfg(unix)]
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::AppState;

// ============================================================================
// Workspace resolution: uses same vault path as sovereign-pair or configurable
// ============================================================================

fn resolve_coding_workspace() -> PathBuf {
    std::env::var("CODING_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_default()
                .join("sovereign-workspace")
        })
}

fn ensure_workspace() -> std::io::Result<PathBuf> {
    let ws = resolve_coding_workspace();
    std::fs::create_dir_all(&ws)?;
    Ok(ws)
}

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub kind: String, // "file" or "directory"
    pub size: Option<u64>,
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Deserialize)]
pub struct ReadFileQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct WriteFilePayload {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFileQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameFilePayload {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug, Deserialize)]
pub struct CompletionPayload {
    pub prefix: String,
    pub suffix: Option<String>,
    pub language: String,
    pub max_tokens: Option<usize>,
}

// ============================================================================
// File Tree (recursive, depth-limited)
// ============================================================================

fn build_tree(dir: &Path, depth: u8, max_depth: u8) -> std::io::Result<Vec<FileNode>> {
    if depth > max_depth {
        return Ok(vec![]);
    }

    let mut nodes = Vec::new();
    let entries = std::fs::read_dir(dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden dirs and common ignore patterns
        if name.starts_with('.') || name == "node_modules" || name == "target" || name == ".git" {
            continue;
        }

        let rel = path.strip_prefix(resolve_coding_workspace()).unwrap_or(&path);
        let rel_str = rel.to_string_lossy().to_string();

        if path.is_dir() {
            let children = build_tree(&path, depth + 1, max_depth).ok();
            nodes.push(FileNode {
                name,
                path: rel_str,
                kind: "directory".to_string(),
                size: None,
                children,
            });
        } else {
            let size = entry.metadata().ok().map(|m| m.len());
            nodes.push(FileNode {
                name,
                path: rel_str,
                kind: "file".to_string(),
                size,
                children: None,
            });
        }
    }

    // Sort: directories first, then alphabetically
    nodes.sort_by(|a, b| {
        a.kind.cmp(&b.kind).reverse().then(a.name.cmp(&b.name))
    });

    Ok(nodes)
}

// ============================================================================
// Handlers
// ============================================================================

pub async fn coding_tree_handler() -> Result<Json<Vec<FileNode>>, StatusCode> {
    let ws = ensure_workspace().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tree = build_tree(&ws, 0, 5).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(tree))
}

pub async fn coding_read_handler(
    Query(query): Query<ReadFileQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let ws = ensure_workspace().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let full_path = ws.join(&query.path);

    // Security: prevent path traversal
    if !full_path.starts_with(&ws) {
        return Err(StatusCode::FORBIDDEN);
    }

    let content = tokio::fs::read_to_string(&full_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "path": query.path,
        "content": content,
        "size": content.len()
    })))
}

pub async fn coding_write_handler(
    Json(payload): Json<WriteFilePayload>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let ws = ensure_workspace().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let full_path = ws.join(&payload.path);

    if !full_path.starts_with(&ws) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    tokio::fs::write(&full_path, &payload.content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "path": payload.path,
        "message": "File written successfully"
    })))
}

pub async fn coding_delete_handler(
    Query(query): Query<DeleteFileQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let ws = ensure_workspace().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let full_path = ws.join(&query.path);

    if !full_path.starts_with(&ws) {
        return Err(StatusCode::FORBIDDEN);
    }

    if full_path.is_dir() {
        tokio::fs::remove_dir_all(&full_path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        tokio::fs::remove_file(&full_path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(serde_json::json!({
        "path": query.path,
        "message": "Deleted successfully"
    })))
}

pub async fn coding_rename_handler(
    Json(payload): Json<RenameFilePayload>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let ws = ensure_workspace().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let old_full = ws.join(&payload.old_path);
    let new_full = ws.join(&payload.new_path);

    if !old_full.starts_with(&ws) || !new_full.starts_with(&ws) {
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(parent) = new_full.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    tokio::fs::rename(&old_full, &new_full)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "old_path": payload.old_path,
        "new_path": payload.new_path,
        "message": "Renamed successfully"
    })))
}

// ============================================================================
// Terminal WebSocket (PTY via tokio)
// ============================================================================

#[cfg(unix)]
pub async fn coding_terminal_ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| async move {
        tracing::info!("🖥️  [Coding Terminal] WebSocket connected");

        let workspace = resolve_coding_workspace();
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

        use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

        let pty_system = NativePtySystem::default();
        let pty_pair = match pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to open PTY: {}", e);
                return;
            }
        };

        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(&workspace);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let _child = match pty_pair.slave.spawn_command(cmd) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to spawn shell: {}", e);
                return;
            }
        };
        // Important: close the slave handle in the parent process
        drop(pty_pair.slave);

        let mut master = pty_pair.master; // to handle resize
        let mut reader = master.try_clone_reader().unwrap();
        let mut writer = master.take_writer().unwrap();

        let (mut ws_tx, mut ws_rx) = socket.split();

        // PTY -> WebSocket (using blocking thread with tokio channel)
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let write_task = tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                let text = String::from_utf8_lossy(&bytes).to_string();
                if ws_tx.send(axum::extract::ws::Message::Text(text)).await.is_err() {
                    break;
                }
            }
        });

        // WebSocket -> PTY
        let read_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_rx.next().await {
                if let axum::extract::ws::Message::Text(input) = msg {
                    // Try to parse JSON
                    #[derive(serde::Deserialize)]
                    struct TermMsg {
                        #[serde(rename = "type")]
                        msg_type: String,
                        data: Option<String>,
                        cols: Option<u16>,
                        rows: Option<u16>,
                    }
                    
                    if let Ok(m) = serde_json::from_str::<TermMsg>(&input) {
                        if m.msg_type == "input" {
                            if let Some(data) = m.data {
                                use std::io::Write;
                                let _ = writer.write_all(data.as_bytes());
                                let _ = writer.flush();
                            }
                        } else if m.msg_type == "resize" {
                            if let (Some(cols), Some(rows)) = (m.cols, m.rows) {
                                let _ = master.resize(PtySize {
                                    rows,
                                    cols,
                                    pixel_width: 0,
                                    pixel_height: 0,
                                });
                            }
                        }
                    } else {
                        // Fallback to raw text
                        use std::io::Write;
                        let _ = writer.write_all(input.as_bytes());
                        let _ = writer.flush();
                    }
                }
            }
        });

        let _ = tokio::join!(write_task, read_task);
        tracing::info!("🖥️  [Coding Terminal] WebSocket disconnected");
    })
}

#[cfg(not(unix))]
pub async fn coding_terminal_ws_handler(
    _ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
) -> axum::response::Response {
    // Terminal PTY is only supported on Unix (Linux/macOS)
    StatusCode::SERVICE_UNAVAILABLE.into_response()
}

// ============================================================================
// Completions (LLM proxy via existing chat endpoint)
// ============================================================================

pub async fn coding_completions_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CompletionPayload>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let max_tokens = payload.max_tokens.unwrap_or(64);

    // Build completion prompt
    let prompt = if let Some(suffix) = &payload.suffix {
        format!(
            "Complete this {} code:\n\n{}\n<|cursor|>\n{}",
            payload.language, payload.prefix, suffix
        )
    } else {
        format!(
            "Complete this {} code:\n\n{}\n<|cursor|>",
            payload.language, payload.prefix
        )
    };

    // Forward to existing Ollama endpoint
    let ollama_url = std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());

    let req_body = serde_json::json!({
        "model": "qwen2.5-coder:7b",
        "prompt": prompt,
        "raw": true,
        "stream": false,
        "options": {
            "temperature": 0.2,
            "num_predict": max_tokens,
            "top_p": 0.9,
        }
    });

    let resp = state
        .http_client
        .post(format!("{}/api/generate", ollama_url))
        .json(&req_body)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            match r.json::<serde_json::Value>().await {
                Ok(json) => Ok(Json(serde_json::json!({
                    "completions": [{
                        "text": json.get("response").and_then(|v| v.as_str()).unwrap_or("")
                    }]
                }))),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        _ => Err(StatusCode::BAD_GATEWAY),
    }
}
