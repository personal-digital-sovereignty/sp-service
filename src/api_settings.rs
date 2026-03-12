use axum::{extract::State, response::IntoResponse, Json};
use serde_json::Value;
use std::sync::Arc;
use crate::AppState;
use sqlx::Row;
use crate::kms;

/// Rota GET /v1/settings - Retorna Chaves Essenciais do Hub
pub async fn get_system_settings_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'system_settings'")
        .fetch_optional(&state.db)
        .await;

    match result {
        Ok(Some(row)) => {
            let val: String = row.get("value_json");
            let mut parsed: Value = serde_json::from_str(&val).unwrap_or(serde_json::json!({}));
            
            // Decifra as chaves sensíveis (Envelope Decryption p/ Uso em Memória da UI / Engrenagens)
            let keys_to_decrypt = vec!["openai_api_key", "anthropic_api_key", "groq_api_key", "gemini_api_key"];
            if let Some(obj) = parsed.as_object_mut() {
                for key in keys_to_decrypt {
                    if let Some(val) = obj.get(key) {
                        if let Some(str_val) = val.as_str() {
                            if !str_val.is_empty() {
                                if let Some(decrypted) = kms::decrypt_vault_secret(str_val) {
                                    obj.insert(key.to_string(), serde_json::json!(decrypted));
                                }
                            }
                        }
                    }
                }
            }
            
            Json(parsed).into_response()
        },
        _ => Json(serde_json::json!({})).into_response()
    }
}

/// Rota POST /v1/settings - Salva a Identidade da Inteligência
pub async fn set_system_settings_handler(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<Value>,
) -> impl IntoResponse {
    
    // Cifra chaves sensíveis (Envelope Encryption - AES-GCM At Rest)
    let keys_to_encrypt = vec!["openai_api_key", "anthropic_api_key", "groq_api_key", "gemini_api_key"];
    if let Some(obj) = payload.as_object_mut() {
        for key in keys_to_encrypt {
            if let Some(val) = obj.get(key) {
                if let Some(str_val) = val.as_str() {
                    if !str_val.is_empty() {
                        if let Some(encrypted) = kms::encrypt_vault_secret(str_val) {
                            obj.insert(key.to_string(), serde_json::json!(encrypted));
                        }
                    }
                }
            }
        }
    }

    let json_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    
    // SQLite UPSERT O.S (Rust Database Write)
    let res = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('system_settings', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(json_str)
        .execute(&state.db)
        .await;

    if res.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response();
    }
    Json(serde_json::json!({"status": "ok"})).into_response()
}

/// Rota GET /v1/settings/ollama_clusters - Lê Nós Ativos
pub async fn get_ollama_clusters_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'ollama_clusters'")
        .fetch_optional(&state.db)
        .await;

    match result {
        Ok(Some(row)) => {
            let val: String = row.get("value_json");
            let parsed: Value = serde_json::from_str(&val).unwrap_or(serde_json::json!({"clusters": [], "active_cluster_id": ""}));
            Json(parsed).into_response()
        },
        _ => Json(serde_json::json!({"clusters": [], "active_cluster_id": ""})).into_response()
    }
}

/// Rota POST /v1/settings/ollama_clusters - Define The Cluster Master
pub async fn set_ollama_clusters_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let json_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    
    let res = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('ollama_clusters', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(json_str)
        .execute(&state.db)
        .await;

    if res.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response();
    }
    Json(serde_json::json!({"status": "ok"})).into_response()
}
