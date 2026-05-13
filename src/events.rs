//! WebSocket Event Bus — broadcasts platform events to connected clients.
//! Clients connect to `/v1/events/ws` and receive real-time events like
//! "file_created", "model_finished", "research_complete", etc.

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use crate::models;
use crate::AppState;

/// WebSocket upgrade handler for `/v1/events/ws`
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.log_sender.subscribe();

    // Send welcome message
    let welcome = serde_json::json!({
        "type": "system",
        "event": "connected",
        "message": "WebSocket event bus connected"
    });
    if socket.send(axum::extract::ws::Message::Text(welcome.to_string().into())).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            // Forward log events from broadcast channel
            Ok(log_entry) = rx.recv() => {
                let event = serde_json::json!({
                    "type": "log",
                    "timestamp": log_entry.timestamp,
                    "level": log_entry.level,
                    "message": log_entry.message
                });
                if socket.send(axum::extract::ws::Message::Text(event.to_string().into())).await.is_err() {
                    break;
                }
            }
            // Handle incoming client messages (ping/pong, unsubscribe, etc.)
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(axum::extract::ws::Message::Ping(data)) => {
                        if socket.send(axum::extract::ws::Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(axum::extract::ws::Message::Close(_)) => {
                        break;
                    }
                    Ok(axum::extract::ws::Message::Text(text)) => {
                        // Echo client acknowledgments or handle commands
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                            if v.get("action").and_then(|a| a.as_str()) == Some("ping") {
                                let pong = serde_json::json!({ "type": "pong", "ts": chrono::Utc::now().to_rfc3339() });
                                if socket.send(axum::extract::ws::Message::Text(pong.to_string().into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                    _ => {}
                }
            }
        }
    }
}
