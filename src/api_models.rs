
// ============================================================================
// Dynamic Model Discovery — /v1/models
// Aggregates models from Ollama (local) + OpenRouter (cloud) into OpenAI format
// ============================================================================

use axum::{extract::State, response::Json};
use serde::Serialize;
use sqlx::Row;
use std::sync::Arc;

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct ModelEntry {
    pub id: String,
    pub object: String,
    pub owned_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ModelList {
    pub object: String,
    pub data: Vec<ModelEntry>,
}

pub async fn get_models_handler(State(state): State<Arc<AppState>>) -> Json<ModelList> {
    let mut models: Vec<ModelEntry> = Vec::new();

    // 1. Discover Ollama models (local)
    let ollama_url = resolve_ollama_url(&state).await;
    match state.http_client.get(format!("{}/api/tags", ollama_url)).send().await {
        Ok(res) if res.status().is_success() => {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                if let Some(model_arr) = json.get("models").and_then(|v| v.as_array()) {
                    for m in model_arr {
                        if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                            let details = m.get("details").and_then(|v| v.as_object());
                            let context_length = details
                                .and_then(|d| d.get("context_length"))
                                .and_then(|v| v.as_u64())
                                .map(|v| v as usize);

                            let id = if name.contains(':') {
                                name.to_string()
                            } else {
                                format!("{}:latest", name)
                            };

                            models.push(ModelEntry {
                                id,
                                object: "model".to_string(),
                                owned_by: "ollama".to_string(),
                                context_length,
                            });
                        }
                    }
                }
            }
        }
        _ => {
            tracing::debug!("[Models] Ollama not available at {}", ollama_url);
        }
    }

    // 2. Discover OpenRouter models (cloud) — only if API key is configured
    if let Ok(Some(row)) = sqlx::query(
        "SELECT value_json FROM global_settings WHERE id = 'openrouter'"
    )
    .fetch_optional(&state.db)
    .await
    {
        let val: String = row.get("value_json");
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
            if let Some(api_key) = parsed.get("api_key").and_then(|v| v.as_str()) {
                if !api_key.is_empty() {
                    // Decrypt if needed (KMS encrypted)
                    let key = if api_key.len() > 20 {
                        crate::kms::decrypt_vault_secret(api_key)
                            .unwrap_or_else(|| api_key.to_string())
                    } else {
                        api_key.to_string()
                    };

                    match state
                        .http_client
                        .get("https://openrouter.ai/api/v1/models")
                        .header("Authorization", format!("Bearer {}", key))
                        .send()
                        .await
                    {
                        Ok(res) if res.status().is_success() => {
                            if let Ok(json) = res.json::<serde_json::Value>().await {
                                if let Some(model_arr) = json.get("data").and_then(|v| v.as_array())
                                {
                                    for m in model_arr {
                                        if let Some(id) = m.get("id").and_then(|v| v.as_str()) {
                                            models.push(ModelEntry {
                                                id: id.to_string(),
                                                object: "model".to_string(),
                                                owned_by: "openrouter".to_string(),
                                                context_length: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            tracing::debug!("[Models] OpenRouter model list unavailable");
                        }
                    }
                }
            }
        }
    }

    // Sort: ollama local models first, then openrouter cloud models
    models.sort_by(|a, b| a.owned_by.cmp(&b.owned_by).then(a.id.cmp(&b.id)));

    // Deduplicate by id
    models.dedup_by(|a, b| a.id == b.id);

    Json(ModelList {
        object: "list".to_string(),
        data: models,
    })
}

async fn resolve_ollama_url(state: &AppState) -> String {
    // Check ollama_clusters config first
    if let Ok(Some(row)) = sqlx::query(
        "SELECT value_json FROM global_settings WHERE id = 'ollama_clusters'",
    )
    .fetch_optional(&state.db)
    .await
    {
        let val: String = row.get("value_json");
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
            let active_id = parsed
                .get("active_cluster_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(clusters) = parsed.get("clusters").and_then(|v| v.as_array()) {
                for c in clusters {
                    if c.get("id").and_then(|v| v.as_str()).unwrap_or("") == active_id {
                        if let Some(url) = c.get("url").and_then(|v| v.as_str()) {
                            let clean = url.trim_end_matches('/').to_string();
                            if !clean.is_empty() {
                                return clean;
                            }
                        }
                    }
                }
            }
        }
    }

    std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string())
}
