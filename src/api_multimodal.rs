use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
    Json,
};
use reqwest::StatusCode;
use serde_json::json;
use std::sync::Arc;
use tokio::fs;

pub async fn audio_transcribe_handler(
    State(_state): State<Arc<crate::AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut audio_temp_path = String::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "audio" {
            let file_name = field.file_name().unwrap_or("upload.audio").to_string();
            let data = match field.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"success": false, "error": format!("Falha ao ler multipart: {}", e)})),
                    );
                }
            };

            let temp_dir = std::env::temp_dir().join("sovereign_audio");
            fs::create_dir_all(&temp_dir).await.ok();
            
            let id = uuid::Uuid::new_v4().to_string();
            let safe_name = format!("{}_{}", id, file_name);
            let target_path = temp_dir.join(safe_name);
            
            if let Err(e) = fs::write(&target_path, data).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"success": false, "error": format!("Falha ao salvar audio temp: {}", e)})),
                );
            }
            
            audio_temp_path = target_path.to_string_lossy().to_string();
            break;
        }
    }

    if audio_temp_path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "error": "Campo 'audio' não encontrado no multipart form"})),
        );
    }

    // Processar o aúdio invocado via Python whisper CLI (Zero-Overhead Node)
    match crate::multimodal::extract_text_from_audio(&audio_temp_path).await {
        Ok(res) => {
            // Delete temp file after execution
            let _ = fs::remove_file(&audio_temp_path).await;
            
            (StatusCode::OK, Json(json!(res)))
        },
        Err(e) => {
            let _ = fs::remove_file(&audio_temp_path).await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "error": e})),
            )
        }
    }
}
