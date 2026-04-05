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

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct ImageGenRequest {
    pub prompt: String,
    pub n: Option<u32>,
    pub size: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SdApiTxt2ImgResponse {
    images: Vec<String>, // Base64 pngs
}

#[allow(clippy::collapsible_if)]
pub async fn generate_image_handler(
    State(_state): State<Arc<crate::AppState>>,
    Json(payload): Json<ImageGenRequest>,
) -> impl IntoResponse {
    let client = reqwest::Client::new();
    
    tracing::info!("📸 [Sovereign Vision Engine] Recebida a incubação Visual para Base64: {}", payload.prompt);
    
    // Stable Diffusion WebUI / SD.cpp Compatible local endpoint
    let sd_url = "http://127.0.0.1:7860/sdapi/v1/txt2img";
    
    // We synthesize the request payload
    let sd_payload = json!({
        "prompt": payload.prompt,
        "negative_prompt": "blurry, low quality, deformed, mutated, ugly, bad anatomy",
        "steps": 4,
        "cfg_scale": 1.5,
        "width": 1024,
        "height": 1024,
        "sampler_name": "Euler a"
    });

    match client.post(sd_url).json(&sd_payload).send().await {
        Ok(res) => {
            if !res.status().is_success() {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("SD API Error: {}", res.status())})),
                );
            }
            if let Ok(sd_resp) = res.json::<SdApiTxt2ImgResponse>().await {
                if let Some(b64_img) = sd_resp.images.first() {
                    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/home/jefersonlopes".to_string());
                    let images_dir = std::path::PathBuf::from(home_dir).join("Vault").join("Images");
                    
                    fs::create_dir_all(&images_dir).await.ok();
                    
                    let filename = format!("art_{}.png", chrono::Local::now().format("%Y%m%d%H%M%S"));
                    let file_path = images_dir.join(&filename);
                    
                    use base64::{Engine as _, engine::general_purpose};
                    match general_purpose::STANDARD.decode(b64_img) {
                        Ok(image_bytes) => {
                            if let Err(e) = fs::write(&file_path, image_bytes).await {
                                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to save image locally: {}", e)})));
                            }
                            
                            // Map the absolute path to a URL reachable by Svelte UI / LLM
                            // Assuming /v1/vault/media/... can serve this, or we just pass the URL back via the Office Parser vault trick
                            let img_url = format!("http://localhost:38001/v1/vault/media?path={}", urlencoding::encode(&file_path.to_string_lossy()));
                            
                            return (StatusCode::OK, Json(json!({
                                "created": chrono::Utc::now().timestamp(),
                                "data": [
                                    { "url": img_url }
                                ]
                            })));
                        },
                        Err(e) => {
                            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to decode Base64: {}", e)})));
                        }
                    }
                }
            }
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "No image returned from SD API"})))
        },
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to connect to local Diffusion Daemon (Is it running on port 7860?): {}", e)})))
        }
    }
}
