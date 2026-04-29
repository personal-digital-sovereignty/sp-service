use axum::{extract::State, response::IntoResponse, Json};
use serde_json::Value;
use std::sync::Arc;
use std::net::IpAddr;
use crate::AppState;
use sqlx::Row;
use crate::kms;

use serde::{Serialize, Deserialize};
use crate::models::{QwenSettings, NvidiaSettings};

/// Limites configuráveis de scraping por contexto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeLimits {
    pub max_links_chat: usize,           // Tool-call no Chat (default: 6)
    pub max_links_deep_research: usize,  // Deep Research pipeline (default: 7)
    pub max_links_per_search: usize,     // Links por query individual (default: 7)
}

impl Default for ScrapeLimits {
    fn default() -> Self {
        Self {
            max_links_chat: 6,
            max_links_deep_research: 7,
            max_links_per_search: 7,
        }
    }
}

/// Carrega os limites de scraping do banco (hot-reload). Fallback: defaults seguros.
pub async fn load_scrape_limits(pool: &sqlx::SqlitePool) -> ScrapeLimits {
    if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'scrape_limits'")
        .fetch_optional(pool).await
    {
        let val: String = row.get("value_json");
        if let Ok(parsed) = serde_json::from_str::<ScrapeLimits>(&val) {
            return parsed;
        }
    }
    ScrapeLimits::default()
}

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
    // P3-03: Body size guard — max 5 MB para prevenir DoS por payload gigante
    const MAX_IMPORT_BYTES: usize = 5 * 1024 * 1024;
    if body.len() > MAX_IMPORT_BYTES {
        return (axum::http::StatusCode::PAYLOAD_TOO_LARGE, "Arquivo Cíbrido excede o limite de 5 MB").into_response();
    }

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

/// Rota GET /v1/settings/openrouter
pub async fn get_openrouter_settings_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'openrouter'")
        .fetch_optional(&state.db)
        .await;

    match result {
        Ok(Some(row)) => {
            let val: String = row.get("value_json");
            let mut parsed: Value = serde_json::from_str(&val).unwrap_or(serde_json::json!({}));
            
            // Decifra a API Key se presente
            if let Some(obj) = parsed.as_object_mut() {
                if let Some(val) = obj.get("api_key")
                    && let Some(str_val) = val.as_str()
                        && !str_val.is_empty()
                            && let Some(decrypted) = kms::decrypt_vault_secret(str_val) {
                                obj.insert("api_key".to_string(), serde_json::json!(decrypted));
                            }
            }
            
            Json(parsed).into_response()
        },
        _ => Json(serde_json::json!({})).into_response()
    }
}

/// Rota POST /v1/settings/openrouter
pub async fn set_openrouter_settings_handler(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<Value>,
) -> impl IntoResponse {
    // Cifra a API Key se enviada (AES-GCM at rest)
    if let Some(obj) = payload.as_object_mut() {
        if let Some(val) = obj.get("api_key")
            && let Some(str_val) = val.as_str()
                && !str_val.is_empty()
                    && let Some(encrypted) = kms::encrypt_vault_secret(str_val) {
                        obj.insert("api_key".to_string(), serde_json::json!(encrypted));
                    }
    }

    let json_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    
    let res = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('openrouter', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(json_str)
        .execute(&state.db)
        .await;

    if res.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response();
    }

    Json(serde_json::json!({"status": "ok"})).into_response()
}

// ==========================================
// Qwen DashScope Settings (Epic 2)
// ==========================================

pub async fn get_qwen_settings_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let row = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'qwen'")
        .fetch_optional(&state.db)
        .await;

    match row {
        Ok(Some(r)) => {
            let val: String = sqlx::Row::get(&r, "value_json");
            if let Ok(mut settings) = serde_json::from_str::<QwenSettings>(&val) {
                // Decifra a chave para exibição na UI
                if let Some(decrypted) = crate::kms::decrypt_vault_secret(&settings.api_key) {
                    settings.api_key = decrypted;
                }
                return Json(settings).into_response();
            }
            Json(QwenSettings::default()).into_response()
        },
        _ => Json(QwenSettings::default()).into_response(),
    }
}

pub async fn set_qwen_settings_handler(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<QwenSettings>,
) -> impl IntoResponse {
    // Cifra a API Key antes de salvar
    if !payload.api_key.is_empty() {
        if let Some(encrypted) = crate::kms::encrypt_vault_secret(&payload.api_key) {
            payload.api_key = encrypted;
        }
    }

    let val_json = serde_json::to_string(&payload).unwrap_or_default();
    let res = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('qwen', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(val_json)
        .execute(&state.db)
        .await;

    match res {
        Ok(_) => (axum::http::StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ==========================================
// NVIDIA NIM Settings (Epic 3)
// ==========================================

pub async fn get_nvidia_settings_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let row = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'nvidia'")
        .fetch_optional(&state.db)
        .await;

    match row {
        Ok(Some(r)) => {
            let val: String = sqlx::Row::get(&r, "value_json");
            if let Ok(mut settings) = serde_json::from_str::<NvidiaSettings>(&val) {
                // Decifra a chave para exibição na UI
                if let Some(decrypted) = crate::kms::decrypt_vault_secret(&settings.api_key) {
                    settings.api_key = decrypted;
                }
                return Json(settings).into_response();
            }
            Json(NvidiaSettings::default()).into_response()
        },
        _ => Json(NvidiaSettings::default()).into_response(),
    }
}

pub async fn set_nvidia_settings_handler(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<NvidiaSettings>,
) -> impl IntoResponse {
    // Cifra a API Key antes de salvar (AES-GCM at rest)
    if !payload.api_key.is_empty() {
        if let Some(encrypted) = crate::kms::encrypt_vault_secret(&payload.api_key) {
            payload.api_key = encrypted;
        }
    }

    let val_json = serde_json::to_string(&payload).unwrap_or_default();
    let res = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('nvidia', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(val_json)
        .execute(&state.db)
        .await;

    match res {
        Ok(_) => (axum::http::StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
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
    pub is_auditor: bool,
    pub is_agent: bool,
    pub is_coder: bool,
    pub is_chat: bool,
    pub is_project: bool,
    pub is_installed: bool,
}

/// GET /v1/settings/model_capabilities
pub async fn get_matrix_capabilities_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Auto-update capabilities matrix before serving it
    crate::api::sync_model_capabilities(&state.db).await;

    let q = "SELECT model_name, parameter_size, supports_tools, is_reasoner, is_master, is_scribe, is_auditor, is_agent, is_coder, is_chat, is_project, is_installed FROM model_capabilities ORDER BY parameter_size DESC";
    match sqlx::query_as::<_, ModelMatrixRow>(q).fetch_all(&state.db).await {
        Ok(rows) => Json(rows).into_response(),
        Err(_) => Json(Vec::<ModelMatrixRow>::new()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UpdateMatrixReq {
    pub model_name: String,
    pub supports_tools: bool,
    pub is_reasoner: bool,
    pub is_master: bool,
    pub is_scribe: bool,
    pub is_auditor: bool,
    pub is_agent: bool,
    pub is_coder: bool,
    pub is_chat: bool,
    pub is_project: bool,
    pub is_installed: bool,
}

/// POST /v1/settings/model_capabilities/toggles
pub async fn update_matrix_toggles_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateMatrixReq>,
) -> impl IntoResponse {
    let res = sqlx::query("UPDATE model_capabilities SET supports_tools = ?, is_reasoner = ?, is_master = ?, is_scribe = ?, is_auditor = ?, is_agent = ?, is_coder = ?, is_chat = ?, is_project = ?, is_installed = ? WHERE model_name = ?")
        .bind(req.supports_tools)
        .bind(req.is_reasoner)
        .bind(req.is_master)
        .bind(req.is_scribe)
        .bind(req.is_auditor)
        .bind(req.is_agent)
        .bind(req.is_coder)
        .bind(req.is_chat)
        .bind(req.is_project)
        .bind(req.is_installed)
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

/// DELETE /v1/settings/model_capabilities/:model_name
/// Remove manualmente uma entrada da Matrix de Capacidades.
/// O Discover automático pode re-cadastrar o modelo ao reiniciar, se estiver instalado.
pub async fn delete_matrix_entry_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(model_name): axum::extract::Path<String>,
) -> impl IntoResponse {
    // O model_name vem URL-encoded (ex: qwen3%3A8b → qwen3:8b) — axum faz decode automaticamente.
    let res = sqlx::query("DELETE FROM model_capabilities WHERE model_name = ?")
        .bind(&model_name)
        .execute(&state.db)
        .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => {
            tracing::info!("🗑 [Matrix] Entrada '{}' removida manualmente da Model Capabilities.", model_name);
            Json(serde_json::json!({"status": "deleted", "model_name": model_name})).into_response()
        },
        Ok(_) => {
            (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Model not found in matrix"}))).into_response()
        },
        Err(e) => {
            tracing::error!("Matrix Delete Error: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true}))).into_response()
        }
    }
}

// =============================================
// SCRAPE LIMITS — Configuração de Iterações
// =============================================

/// GET /v1/settings/scrape_limits
pub async fn get_scrape_limits_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let limits = load_scrape_limits(&state.db).await;
    Json(limits).into_response()
}

/// POST /v1/settings/scrape_limits
pub async fn set_scrape_limits_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ScrapeLimits>,
) -> impl IntoResponse {
    // Guardrails de sanidade: mínimo 1, máximo 30
    let clamped = ScrapeLimits {
        max_links_chat: req.max_links_chat.clamp(1, 30),
        max_links_deep_research: req.max_links_deep_research.clamp(1, 30),
        max_links_per_search: req.max_links_per_search.clamp(1, 30),
    };

    let json_str = serde_json::to_string(&clamped).unwrap_or_default();
    let res = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('scrape_limits', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(&json_str)
        .execute(&state.db)
        .await;

    match res {
        Ok(_) => Json(serde_json::json!({"status": "success", "limits": clamped})).into_response(),
        Err(e) => {
            tracing::error!("Scrape Limits Save Error: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true}))).into_response()
        }
    }
}

// =============================================
// SOVEREIGN PROMPT VAULT — CRUD Handlers
// =============================================

/// GET /v1/settings/prompts
pub async fn get_prompts_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let q = "SELECT id, slug, category, title, prompt_text, placeholders, is_core, is_active, version, integrity_hash, created_at, updated_at, created_by FROM sovereign_prompts ORDER BY id ASC";
    match sqlx::query_as::<_, crate::prompt_vault::PromptRow>(q).fetch_all(&state.db).await {
        Ok(rows) => Json(rows).into_response(),
        Err(_) => Json(Vec::<crate::prompt_vault::PromptRow>::new()).into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct UpsertPromptReq {
    pub slug: String,
    pub title: String,
    pub category: String,
    pub prompt_text: String,
    pub placeholders: Option<Vec<String>>,
}

/// POST /v1/settings/prompts
pub async fn upsert_prompt_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpsertPromptReq>,
) -> impl IntoResponse {
    // Guard: bloquear IDs no namespace reservado SP-9xxx
    if req.slug.starts_with("SP-9") || req.slug.starts_with("sp-9") {
        return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({
            "error": "Namespace reservado SP-9xxx. Prompts core são gerenciados pelo sistema."
        }))).into_response();
    }

    // Guard: não permitir overwrite de prompts is_core=1
    if let Ok(Some(is_core)) = sqlx::query_scalar::<_, bool>(
        "SELECT is_core FROM sovereign_prompts WHERE slug = ?"
    ).bind(&req.slug).fetch_optional(&state.db).await {
        if is_core {
            return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({
                "error": "Este prompt é protegido pelo Cognitive Firewall e não pode ser alterado."
            }))).into_response();
        }
    }

    // LLM Validation: verificar se o novo prompt conflita com regras core
    match crate::prompt_vault::validate_prompt_with_llm(&state.db, &req.prompt_text).await {
        Err(reason) => {
            return (axum::http::StatusCode::CONFLICT, Json(serde_json::json!({
                "error": "Prompt rejeitado pelo Cognitive Firewall",
                "reason": reason
            }))).into_response();
        }
        Ok(_) => {}
    }

    let placeholders_json = serde_json::to_string(&req.placeholders.unwrap_or_default()).unwrap_or("[]".to_string());
    let id = uuid::Uuid::new_v4().to_string();
    let hash = {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(req.prompt_text.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let res = sqlx::query(
        "INSERT INTO sovereign_prompts (id, slug, category, title, prompt_text, placeholders, is_core, is_active, version, integrity_hash, created_by)
         VALUES (?, ?, ?, ?, ?, ?, 0, 1, 1, ?, 'user')
         ON CONFLICT(slug) DO UPDATE SET
            title = excluded.title,
            category = excluded.category,
            prompt_text = excluded.prompt_text,
            placeholders = excluded.placeholders,
            integrity_hash = excluded.integrity_hash,
            version = sovereign_prompts.version + 1,
            updated_at = CURRENT_TIMESTAMP"
    )
    .bind(&id)
    .bind(&req.slug)
    .bind(&req.category)
    .bind(&req.title)
    .bind(&req.prompt_text)
    .bind(&placeholders_json)
    .bind(&hash)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => Json(serde_json::json!({"status": "success", "slug": req.slug})).into_response(),
        Err(e) => {
            tracing::error!("Prompt Vault Upsert Error: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true}))).into_response()
        }
    }
}

/// DELETE /v1/settings/prompts/:slug (soft-delete)
pub async fn delete_prompt_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> impl IntoResponse {
    // Guard: não permitir delete de prompts core
    if let Ok(Some(is_core)) = sqlx::query_scalar::<_, bool>(
        "SELECT is_core FROM sovereign_prompts WHERE slug = ?"
    ).bind(&slug).fetch_optional(&state.db).await {
        if is_core {
            return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({
                "error": "Prompts core não podem ser desativados."
            }))).into_response();
        }
    }

    // Soft-delete: apenas desativa
    let res = sqlx::query("UPDATE sovereign_prompts SET is_active = 0, updated_at = CURRENT_TIMESTAMP WHERE slug = ?")
        .bind(&slug)
        .execute(&state.db)
        .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => {
            Json(serde_json::json!({"status": "deactivated", "slug": slug})).into_response()
        },
        Ok(_) => (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Prompt not found"}))).into_response(),
        Err(e) => {
            tracing::error!("Prompt Vault Delete Error: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true}))).into_response()
        }
    }
}

// ---------------------------------------------------------
// P2P MESH & CLOUD TARGET SANDBOXING
// ---------------------------------------------------------

/// GAP-O01: SSRF Guard robusto.
/// Normaliza o host (remove schema), parsea como IpAddr e rejeita:
/// loopback (127.x, ::1), unspecified (0.x), link-local (169.254.x),
/// e todo o espaço RFC1918 privado (10.x, 172.16-31.x, 192.168.x).
fn is_ssrf_target(raw: &str) -> bool {
    // Normaliza: remove schema http:// ou https://
    let host = raw
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(raw);

    // Extrai o host isolado (lida corretamente com portas e IPv6)
    // [::1]:8080 -> ::1 | 127.0.0.1:8080 -> 127.0.0.1
    let extracted = if host.starts_with('[') {
        if let Some(end) = host.find(']') {
            &host[1..end]
        } else {
            host
        }
    } else {
        host.split(':').next().unwrap_or(host)
    };

    // Bloqueia hosts literais conhecidos
    if extracted.eq_ignore_ascii_case("localhost") { return true; }

    // Tenta parsear como IP
    if let Ok(ip) = extracted.parse::<IpAddr>() {
        if ip.is_loopback() || ip.is_unspecified() { return true; }
        match ip {
            IpAddr::V4(v4) => {
                let octets = v4.octets();
                // 169.254.x.x - link-local / AWS/GCP metadata
                if octets[0] == 169 && octets[1] == 254 { return true; }
                // 10.x.x.x - RFC1918 Class A
                if octets[0] == 10 { return true; }
                // 172.16.x.x – 172.31.x.x - RFC1918 Class B
                if octets[0] == 172 && (16..=31).contains(&octets[1]) { return true; }
                // 192.168.x.x - RFC1918 Class C
                if octets[0] == 192 && octets[1] == 168 { return true; }
            }
            IpAddr::V6(_) => {
                // IPv6 fc00::/7 (Unique Local, equivalente ao RFC1918)
                if let IpAddr::V6(v6) = ip {
                    let seg = v6.segments()[0];
                    if (seg & 0xfe00) == 0xfc00 { return true; }
                }
            }
        }
    }
    false
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct P2PMeshConfig {
    pub target_ip: String,
    pub port: u16,
    pub mesh_key: String,
}

/// GET /v1/settings/p2p_mesh
pub async fn get_p2p_mesh_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'p2p_mesh'")
        .fetch_optional(&state.db)
        .await;

    match result {
        Ok(Some(row)) => {
            let val: String = row.get("value_json");
            let mut parsed: Value = serde_json::from_str(&val).unwrap_or(serde_json::json!({
                "target_ip": "",
                "port": 38001,
                "mesh_key": ""
            }));
            
            // MASK mesh_key (GAP-C03)
            if let Some(obj) = parsed.as_object_mut() {
                if let Some(mk) = obj.get("mesh_key").and_then(|v| v.as_str()) {
                    if !mk.is_empty() {
                        obj.insert("mesh_key".to_string(), serde_json::json!("••••••••••••••••"));
                    }
                }
            }

            Json(parsed).into_response()
        },
        _ => Json(serde_json::json!({
            "target_ip": "",
            "port": 38001,
            "mesh_key": ""
        })).into_response()
    }
}

/// POST /v1/settings/p2p_mesh
pub async fn set_p2p_mesh_handler(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<P2PMeshConfig>,
) -> impl IntoResponse {
    // GAP-O01: SSRF Guard robusto (Opus audit)
    if is_ssrf_target(&payload.target_ip) {
        return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": true, "message": "SSRF Guard: Target IP is a protected internal/loopback/private address."}))).into_response();
    }

    // Zero Trust Handshake Validador
    if !payload.target_ip.is_empty() {
        let host = payload.target_ip.trim_start_matches("https://").trim_start_matches("http://");
        let url = format!("http://{}:{}/v1/mesh/handshake", host, payload.port);

        // GAP-O01: redirect-none client prevents HTTP redirect SSRF bypass
        let no_redirect_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        match no_redirect_client.get(&url).send().await {
            Ok(res) if res.status().is_success() => {
                tracing::info!("✅ [Sovereign Mesh] Handshake físico efetuado com nó pareado.");
            },
            Err(e) => {
                tracing::warn!("⚠️ [Sovereign Mesh] Handshake falhou: {}", e);
                return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": true, "message": "Failed to connect to Sovereign Mesh Node."}))).into_response();
            },
            _ => {
                return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": true, "message": "Sovereign Mesh Node rejected the handshake."}))).into_response();
            }
        }
    }

    // GAP-C01 & C03: Encrypt or Preserve mesh_key
    if payload.mesh_key == "••••••••••••••••" {
        if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'p2p_mesh'").fetch_optional(&state.db).await {
            let val: String = row.get("value_json");
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&val) {
                if let Some(old_key) = v.get("mesh_key").and_then(|k| k.as_str()) {
                    payload.mesh_key = old_key.to_string();
                }
            }
        }
    } else if !payload.mesh_key.is_empty() {
        if let Some(enc) = crate::kms::encrypt_vault_secret(&payload.mesh_key) {
            payload.mesh_key = enc;
        } else {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true, "message": "Mesh Key KMS Encryption Failed"}))).into_response();
        }
    }

    let json_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    
    let res = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('p2p_mesh', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(json_str)
        .execute(&state.db)
        .await;

    if res.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response();
    }

    // GAP-O03: Broadcast topology change to all active mesh peers
    let s_clone = state.clone();
    tokio::spawn(async move {
        broadcast_profile_to_mesh(s_clone).await;
    });

    Json(serde_json::json!({"status": "ok"})).into_response()
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CloudTargetConfig {
    pub host_ip: String,
    pub pem_key: String,
}

/// GET /v1/settings/cloud_target
pub async fn get_cloud_target_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'cloud_target'")
        .fetch_optional(&state.db)
        .await;

    match result {
        Ok(Some(row)) => {
            let val: String = row.get("value_json");
            let mut parsed: Value = serde_json::from_str(&val).unwrap_or(serde_json::json!({
                "host_ip": "",
                "pem_key": ""
            }));
            
            // GAP-C03: MASK PEM at runtime to display/use
            if let Some(obj) = parsed.as_object_mut() {
                if let Some(pem) = obj.get("pem_key").and_then(|v| v.as_str()) {
                    if !pem.is_empty() {
                        obj.insert("pem_key".to_string(), serde_json::json!("••••••••••••••••"));
                    }
                }
            }

            Json(parsed).into_response()
        },
        _ => Json(serde_json::json!({
            "host_ip": "",
            "pem_key": ""
        })).into_response()
    }
}

/// POST /v1/settings/cloud_target
pub async fn set_cloud_target_handler(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<CloudTargetConfig>,
) -> impl IntoResponse {
    
    // GAP-O01: SSRF Guard robusto (Opus audit)
    if is_ssrf_target(&payload.host_ip) {
        return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": true, "message": "SSRF Guard: OCI Host IP is a protected internal/loopback/private address."}))).into_response();
    }

    // GAP-C03: Maintain old PEM if received placeholder, otherwise encrypt
    if payload.pem_key == "••••••••••••••••" {
        if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'cloud_target'").fetch_optional(&state.db).await {
            let val: String = row.get("value_json");
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&val) {
                if let Some(old_key) = v.get("pem_key").and_then(|k| k.as_str()) {
                    payload.pem_key = old_key.to_string();
                }
            }
        }
    } else if !payload.pem_key.is_empty() {
        if let Some(encrypted) = crate::kms::encrypt_vault_secret(&payload.pem_key) {
            payload.pem_key = encrypted;
        } else {
             return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true, "message": "Failed to encrypt PEM key with AES-GCM"}))).into_response();
        }
    }

    let json_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    
    let res = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('cloud_target', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(json_str)
        .execute(&state.db)
        .await;

    if res.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response();
    }

    // GAP-O03: Broadcast topology change to all active mesh peers
    let s_clone = state.clone();
    tokio::spawn(async move {
        broadcast_profile_to_mesh(s_clone).await;
    });

    Json(serde_json::json!({"status": "ok"})).into_response()
}
