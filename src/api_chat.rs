use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use tracing::error;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatSessionRow {
    pub id: i64,
    pub title: Option<String>,
    pub folder_name: Option<String>,
    pub workspace_id: Option<String>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatMessageRow {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub thumbs_up: Option<i32>,
    pub thumbs_down: Option<i32>,
    pub created_at: Option<chrono::NaiveDateTime>,
}

#[derive(Serialize)]
pub struct ChatSessionResponse {
    pub id: i64,
    pub title: Option<String>,
    pub folder_name: Option<String>,
    pub tags: Vec<String>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub messages: Vec<ChatMessageRow>,
}

#[derive(Deserialize)]
pub struct SessionQuery {
    pub workspace_id: Option<String>,
}

/// A Rota V1 Legada (Recriada em Rust) que alimenta o Menu Esquerdo c/ Historico
pub async fn get_sessions_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>
) -> impl IntoResponse {
    let ws_param = query.workspace_id.unwrap_or_else(|| "default".to_string());
    let rows = sqlx::query_as::<_, ChatSessionRow>(
        "SELECT id, title, folder_name, workspace_id, created_at, updated_at FROM chat_sessions WHERE coalesce(workspace_id, 'default') = ? ORDER BY updated_at DESC"
    )
    .bind(ws_param)
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(sessions) => Json(sessions).into_response(),
        Err(e) => {
            error!("Erro SQLx Cíbrido ao ler chat_sessions: {}", e);
            Json(serde_json::json!([])).into_response()
        }
    }
}

/// Rota Singular: Carga Cognitiva Completa ao selecionar uma Sessão
pub async fn get_session_by_id_handler(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let session = sqlx::query_as::<_, ChatSessionRow>(
        "SELECT id, title, folder_name, workspace_id, created_at, updated_at FROM chat_sessions WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await;

    match session {
        Ok(Some(s)) => {
            let msgs = sqlx::query_as::<_, ChatMessageRow>(
                r#"SELECT id, session_id, role, content, thumbs_up, thumbs_down, created_at 
                   FROM chat_messages 
                   WHERE session_id = ? 
                   ORDER BY created_at ASC"#
            )
            .bind(s.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let res = ChatSessionResponse {
                id: s.id,
                title: s.title,
                folder_name: s.folder_name,
                tags: vec![], // Bypass temporal Cíbrido 
                created_at: s.created_at,
                updated_at: s.updated_at,
                messages: msgs,
            };

            Json(res).into_response()
        }
        _ => {
            let empty = serde_json::json!({
                "id": id,
                "title": "Sessão Órfã",
                "messages": []
            });
            Json(empty).into_response()
        }
    }
}

/// Rota Deletar: Obliterando Conversas Passadas do SQLite
pub async fn delete_session_handler(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!("♻️ Lixeira: Tentando Obliterar Sessão {}", id);
    
    // Purga agressiva das mensagens atreladas (Foreign Key Safety Drop)
    if let Err(e) = sqlx::query("DELETE FROM chat_messages WHERE session_id = ?")
        .bind(id)
        .execute(&state.db)
        .await 
    {
        tracing::error!("🚨 Falha do lado SQLite ao deletar chat_messages: {}", e);
    }

    // Remove o Container Cíbrido Pai
    if let Err(e) = sqlx::query("DELETE FROM chat_sessions WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await 
    {
        tracing::error!("🚨 Falha do lado SQLite ao deletar chat_sessions pai: {}", e);
    }

    tracing::info!("✅ Sessão {} completamente evaporada via Node Mesh.", id);

    Json(serde_json::json!({ "status": "deleted" })).into_response()
}

#[derive(Deserialize)]
pub struct UpdateSessionRequest {
    pub title: String,
    pub folder_name: Option<String>,
}

pub async fn update_session_handler(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    axum::Json(payload): axum::Json<UpdateSessionRequest>,
) -> impl IntoResponse {
    let _ = sqlx::query("UPDATE chat_sessions SET title = ?, folder_name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&payload.title)
        .bind(&payload.folder_name)
        .bind(id)
        .execute(&state.db)
        .await;

    Json(serde_json::json!({ "status": "updated" })).into_response()
}

// -------------------------------------------------------------
// Core Engine Helpers - Persistência Atômica do Event Loop LLM
// -------------------------------------------------------------

/// Valida um ID de sessão recebido ou cria uma Nova Sessão derivando Título Atômico
pub async fn get_or_create_session(
    db: &sqlx::SqlitePool,
    session_id: Option<i64>,
    first_msg: &str,
    workspace_id: &str,
) -> i64 {
    if let Some(id) = session_id {
        return id;
    }

    // Título autogerado baseado no Início do Prompt
    let title = first_msg.chars().take(40).collect::<String>();

    let res = sqlx::query("INSERT INTO chat_sessions (title, workspace_id) VALUES (?, ?)")
        .bind(&title)
        .bind(workspace_id)
        .execute(db)
        .await;

    match res {
        Ok(exec) => exec.last_insert_rowid(),
        Err(e) => {
            error!("🚨 Falha FATAL ao instanciar Sovereign Chat Session: {}", e);
            0 // ID De Queda
        }
    }
}

/// Crava fisicamente As Memórias (Humano e Agente) no Sovereign Memory (DB)
pub async fn save_message(
    db: &sqlx::SqlitePool,
    session_id: i64,
    role: &str,
    content: &str,
) {
    if session_id <= 0 {
        return;
    }

    let _ = sqlx::query(
        "INSERT INTO chat_messages (session_id, role, content) VALUES (?, ?, ?)"
    )
    .bind(session_id)
    .bind(role)
    .bind(content)
    .execute(db)
    .await;

    // Engatilha Atualização do Relógio Temporal na Sessão Mãe
    let _ = sqlx::query("UPDATE chat_sessions SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(session_id)
        .execute(db)
        .await;
}