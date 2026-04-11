use axum::{extract::State, response::IntoResponse, Json};
use serde_json::Value;
use std::sync::Arc;
use crate::AppState;
use sqlx::Row;
use crate::kms;

use serde::{Serialize, Deserialize};

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
                    if let Some(val) = obj.get(key)
                        && let Some(str_val) = val.as_str()
                            && !str_val.is_empty()
                                && let Some(decrypted) = kms::decrypt_vault_secret(str_val) {
                                    obj.insert(key.to_string(), serde_json::json!(decrypted));
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
            if let Some(val) = obj.get(key)
                && let Some(str_val) = val.as_str()
                    && !str_val.is_empty()
                        && let Some(encrypted) = kms::encrypt_vault_secret(str_val) {
                            obj.insert(key.to_string(), serde_json::json!(encrypted));
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

    // Auto-Roaming: Mesh Broadcast
    let s_clone = state.clone();
    tokio::spawn(async move {
        broadcast_profile_to_mesh(s_clone).await;
    });

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

    // Auto-Roaming: Mesh Broadcast
    let s_clone = state.clone();
    tokio::spawn(async move {
        broadcast_profile_to_mesh(s_clone).await;
    });

    Json(serde_json::json!({"status": "ok"})).into_response()
}

/// Rota GET /v1/settings/searxng - Carrega instâncias P2P
pub async fn get_searxng_nodes_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'searxng_nodes'")
        .fetch_optional(&state.db)
        .await;

    match result {
        Ok(Some(row)) => {
            let val: String = row.get("value_json");
            let parsed: Value = serde_json::from_str(&val).unwrap_or(serde_json::json!([]));
            Json(parsed).into_response()
        },
        _ => {
            // Emite o Array Vazio para evitar Crash Parser
            Json(serde_json::json!([])).into_response()
        }
    }
}

/// Rota POST /v1/settings/searxng - Salva Frota de Crawlers Híbridos
pub async fn set_searxng_nodes_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let json_str = serde_json::to_string(&payload).unwrap_or_else(|_| "[]".to_string());
    let _ = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('searxng_nodes', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(json_str)
        .execute(&state.db)
        .await;
        
    Json(serde_json::json!({"status": "ok"})).into_response()
}

// ==========================================
// THE ROAMING ARCHITECTURE: O.S IMP/EXP
// ==========================================

#[derive(Serialize, Deserialize)]
pub struct CybridConfigExport {
    pub global_settings: Vec<Value>,
    pub workspaces: Vec<Value>,
}

pub async fn generate_cybrid_payload(db: &sqlx::SqlitePool) -> String {
    let mut payload = CybridConfigExport {
        global_settings: vec![],
        workspaces: vec![],
    };

    if let Ok(rows) = sqlx::query("SELECT id, value_json FROM global_settings").fetch_all(db).await {
        for row in rows {
            payload.global_settings.push(serde_json::json!({
                "id": row.get::<String, _>("id"),
                "value_json": row.get::<String, _>("value_json")
            }));
        }
    }

    if let Ok(rows) = sqlx::query("SELECT id, name, absolute_path FROM workspaces").fetch_all(db).await {
        for row in rows {
            payload.workspaces.push(serde_json::json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "absolute_path": row.get::<String, _>("absolute_path")
            }));
        }
    }

    let json_str = serde_json::to_string(&payload).unwrap_or_default();
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.encode(&json_str)
}

pub async fn broadcast_profile_to_mesh(state: Arc<AppState>) {
    let tunnels = crate::ssh_mesh_connector::ACTIVE_MESH_TUNNELS.lock().await;
    if tunnels.is_empty() { return; }

    tracing::info!("📡 [Sovereign Roaming] Transmitindo mutação de Identidade/Config para '{}' Nós pares...", tunnels.len());
    
    let encoded_config = generate_cybrid_payload(&state.db).await;
    let client = reqwest::Client::new();

    for (port, (uri, _)) in tunnels.iter() {
        let target_url = format!("http://127.0.0.1:{}/v1/system/import_config", port);
        let config_clone = encoded_config.clone();
        let uri_clone = uri.clone();
        let client_clone = client.clone();
        
        tokio::spawn(async move {
            if let Err(e) = client_clone.post(&target_url).body(config_clone).timeout(std::time::Duration::from_secs(10)).send().await {
                tracing::warn!("⚠️ [Sovereign Roaming] Falha ao sincronizar identidade com Nó Pareado '{}': {}", uri_clone, e);
            } else {
                tracing::info!("✅ [Sovereign Roaming] Identidade sincronizada ininterruptamente com Nó '{}'.", uri_clone);
            }
        });
    }
}

/// Rota GET /v1/system/export_config - Empacota o Nó atual em um arquivo .cybrid
pub async fn export_config_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let encoded = generate_cybrid_payload(&state.db).await;

    // Force browser download headers
    let headers = axum::response::AppendHeaders([
        (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"identity.cybrid\""),
        (axum::http::header::CONTENT_TYPE, "application/octet-stream"),
    ]);

    (headers, encoded).into_response()
}

/// Rota POST /v1/system/import_config - Engole um .cybrid e reescreve o Nó O.S
pub async fn import_config_handler(
    State(state): State<Arc<AppState>>,
    body: String
) -> impl IntoResponse {
    use base64::{Engine as _, engine::general_purpose};
    let decoded = match general_purpose::STANDARD.decode(&body) {
        Ok(b) => b,
        Err(_) => return (axum::http::StatusCode::BAD_REQUEST, "Arquivo Cíbrido Corrompido").into_response()
    };

    let payload: CybridConfigExport = match serde_json::from_slice(&decoded) {
        Ok(p) => p,
        Err(_) => return (axum::http::StatusCode::BAD_REQUEST, "Formato incompatível do Córtex Cíbrido").into_response()
    };

    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "DB Transaction Error").into_response()
    };

    // Reseta e Importa Workspaces
    let _ = sqlx::query("DELETE FROM workspaces").execute(&mut *tx).await;
    for ws in payload.workspaces {
        if let Some(id) = ws.get("id").and_then(|v| v.as_str())
            && let Some(name) = ws.get("name").and_then(|v| v.as_str())
                && let Some(abs_path) = ws.get("absolute_path").and_then(|v| v.as_str()) {
                    let _ = sqlx::query("INSERT INTO workspaces (id, name, absolute_path) VALUES (?, ?, ?)")
                        .bind(id)
                        .bind(name)
                        .bind(abs_path)
                        .execute(&mut *tx).await;
                }
    }

    // Importa (Upsert) Global Settings
    for st in payload.global_settings {
        if let Some(id) = st.get("id").and_then(|v| v.as_str())
            && let Some(val) = st.get("value_json").and_then(|v| v.as_str()) {
                let _ = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
                    .bind(id)
                    .bind(val)
                    .execute(&mut *tx).await;
            }
    }

    if tx.commit().await.is_ok() {
        Json(serde_json::json!({"status": "Córtex Absorvido com Sucesso. Reinicie a Interface."})).into_response()
    } else {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "O Nó Rejeitou o Coração Cíbrido.").into_response()
    }
}

/// Varre a tabela `ollama_clusters` para descobrir a URL ativa e lista os modelos (/api/tags)
pub async fn get_available_models_handler(State(state): State<Arc<crate::AppState>>) -> impl IntoResponse {
    let mut ollama_base_url = std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()).to_string();

    if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'ollama_clusters'").fetch_optional(&state.db).await {
        let val: String = sqlx::Row::get(&row, "value_json");
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
            let active_id = parsed.get("active_cluster_id").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(clusters) = parsed.get("clusters").and_then(|v| v.as_array()) {
                for c in clusters {
                    if c.get("id").and_then(|v| v.as_str()).unwrap_or("") == active_id
                        && let Some(url) = c.get("url").and_then(|v| v.as_str()) {
                            let clean_url = url.trim_end_matches('/').to_string();
                            if !clean_url.is_empty() {
                                ollama_base_url = clean_url;
                            }
                        }
                }
            }
        }
    }

    if ollama_base_url == "http://host.docker.internal:11434" && std::env::var("SOVEREIGN_RUN_ENV").unwrap_or_default() == "native" {
        ollama_base_url = std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()).to_string();
    }

    let endpoint = format!("{}/api/tags", ollama_base_url);
    
    match state.http_client.get(&endpoint).send().await {
        Ok(res) if res.status().is_success() => {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                Json(json).into_response()
            } else {
                Json(serde_json::json!({"models": []})).into_response()
            }
        },
        _ => Json(serde_json::json!({"models": []})).into_response() // Retorna vazio se o nó remoto estiver offline
    }
}

/// Rota GET /v1/system/docs/user_guide - Retorna o README / Oficial Manual em Raw Markdown
pub async fn get_user_guide_handler() -> impl IntoResponse {
    let guide_path = "docs/user_guide.md";
    match tokio::fs::read_to_string(guide_path).await {
        Ok(content) => (
            axum::http::StatusCode::OK,
            [("Content-Type", "text/plain; charset=utf-8")],
            content,
        ).into_response(),
        Err(_) => (
            axum::http::StatusCode::NOT_FOUND,
            "Sovereign User Guide (docs/user_guide.md) não encontrado nativamente no disco.",
        ).into_response()
    }
}

/// Rota GET /v1/settings/cold_storage
pub async fn get_cold_storage_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'cold_storage'")
        .fetch_optional(&state.db)
        .await;

    match result {
        Ok(Some(row)) => {
            let val: String = row.get("value_json");
            let parsed: Value = serde_json::from_str(&val).unwrap_or(serde_json::json!({
                "corporaVaultPath": "/Vault/Offline_Corpus",
                "offlineCorpora": []
            }));
            Json(parsed).into_response()
        },
        _ => Json(serde_json::json!({
                "corporaVaultPath": "/Vault/Offline_Corpus",
                "offlineCorpora": []
            })).into_response()
    }
}

/// Rota POST /v1/settings/cold_storage
pub async fn set_cold_storage_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let json_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    
    let res = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('cold_storage', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(json_str)
        .execute(&state.db)
        .await;

    if res.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response();
    }

    Json(serde_json::json!({"status": "ok"})).into_response()
}

// ---------------------------------------------------------
// SECOPS VAULT: TENANT API KEYS (CRUD)
// ---------------------------------------------------------

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct TenantApiKeyRow {
    pub id: String,
    pub provider_name: String,
    pub created_at: Option<chrono::NaiveDateTime>,
    // Nós NUNCA retornamos a chave em texto plano para o Frontend
}

#[derive(Deserialize)]
pub struct CreateTenantKeyReq {
    pub provider_name: String,
    pub api_key_value: String,
}

/// GET /v1/settings/tenant_keys
pub async fn get_tenant_keys_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, TenantApiKeyRow>("SELECT id, provider_name, created_at FROM tenant_api_keys ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await;

    match rows {
        Ok(keys) => Json(keys).into_response(),
        Err(_) => Json(Vec::<TenantApiKeyRow>::new()).into_response()
    }
}

/// POST /v1/settings/tenant_keys
pub async fn create_tenant_key_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTenantKeyReq>,
) -> impl IntoResponse {
    let new_id = uuid::Uuid::new_v4().to_string();
    
    // Criptografa a chave usando o KMS nativo (AES-GCM at rest)
    let encrypted_key = match kms::encrypt_vault_secret(&req.api_key_value) {
        Some(cipher) => cipher,
        None => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true, "message": "KMS Encryption failed"}))).into_response(),
    };

    let res = sqlx::query("INSERT INTO tenant_api_keys (id, provider_name, api_key_value) VALUES (?, ?, ?)")
        .bind(&new_id)
        .bind(&req.provider_name)
        .bind(&encrypted_key)
        .execute(&state.db)
        .await;

    match res {
        Ok(_) => Json(serde_json::json!({"status": "created", "id": new_id})).into_response(),
        Err(e) => {
            if e.to_string().contains("UNIQUE") {
                // Upsert se já existir
                let _ = sqlx::query("UPDATE tenant_api_keys SET api_key_value = ?, updated_at = CURRENT_TIMESTAMP WHERE provider_name = ?")
                    .bind(&encrypted_key)
                    .bind(&req.provider_name)
                    .execute(&state.db)
                    .await;
                Json(serde_json::json!({"status": "updated"})).into_response()
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true, "message": "Database Error"}))).into_response()
            }
        }
    }
}

/// DELETE /v1/settings/tenant_keys/:id
pub async fn delete_tenant_key_handler(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let res = sqlx::query("DELETE FROM tenant_api_keys WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await;

    match res {
        Ok(_) => Json(serde_json::json!({"status": "deleted"})).into_response(),
        Err(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true, "message": "Database Error"}))).into_response(),
    }
}

// ---------------------------------------------------------
// MODEL CAPABILITIES MATRIX API (EPIC 4)
// ---------------------------------------------------------

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ModelMatrixRow {
    pub model_name: String,
    pub parameter_size: f32,
    pub supports_tools: bool,
    pub is_reasoner: bool,
    pub is_master: bool,
    pub is_scribe: bool,
    pub is_agent: bool,
    pub is_coder: bool,
    pub is_chat: bool,
    pub is_project: bool,
    pub is_installed: bool,
}

/// GET /v1/settings/model_capabilities
pub async fn get_matrix_capabilities_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let q = "SELECT model_name, parameter_size, supports_tools, is_reasoner, is_master, is_scribe, is_agent, is_coder, is_chat, is_project, is_installed FROM model_capabilities ORDER BY parameter_size DESC";
    match sqlx::query_as::<_, ModelMatrixRow>(q).fetch_all(&state.db).await {
        Ok(rows) => Json(rows).into_response(),
        Err(_) => Json(Vec::<ModelMatrixRow>::new()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UpdateMatrixReq {
    pub model_name: String,
    pub is_master: bool,
    pub is_scribe: bool,
    pub is_agent: bool,
    pub is_coder: bool,
    pub is_chat: bool,
    pub is_project: bool,
}

/// POST /v1/settings/model_capabilities/toggles
pub async fn update_matrix_toggles_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateMatrixReq>,
) -> impl IntoResponse {
    let res = sqlx::query("UPDATE model_capabilities SET is_master = ?, is_scribe = ?, is_agent = ?, is_coder = ?, is_chat = ?, is_project = ? WHERE model_name = ?")
        .bind(req.is_master)
        .bind(req.is_scribe)
        .bind(req.is_agent)
        .bind(req.is_coder)
        .bind(req.is_chat)
        .bind(req.is_project)
        .bind(&req.model_name)
        .execute(&state.db)
        .await;

    match res {
        Ok(_) => Json(serde_json::json!({"status": "success", "message": "Matrix Updated"})).into_response(),
        Err(e) => {
            tracing::error!("Matrix Update Error: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true}))).into_response()
        }
    }
}
