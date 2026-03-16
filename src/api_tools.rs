use axum::{
    extract::{State, Json},
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use crate::AppState;
use tracing::{info, warn, error};
use std::path::{PathBuf};
use tokio::fs;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ReadFileRequest {
    pub workspace_id: String,
    pub relative_path: String,
}

#[derive(Deserialize)]
pub struct CreateKanbanTaskRequest {
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
}

// --------------------------------------------------------
// Agentic Tools: Exposes System functionality to the LLM
// --------------------------------------------------------

pub async fn read_vault_file_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReadFileRequest>,
) -> impl IntoResponse {
    info!("🤖 Agentic Call: ReadFile -> {} / {}", payload.workspace_id, payload.relative_path);

    if let Ok(workspace) = sqlx::query_as::<_, crate::api_vault::WorkspaceRow>("SELECT id, name, absolute_path as path, created_at FROM workspaces WHERE id = ?")
        .bind(&payload.workspace_id)
        .fetch_optional(&state.db)
        .await
        && let Some(w) = workspace {
            let full_path = PathBuf::from(&w.path).join(&payload.relative_path);
            
            // Verificação Anti-Traversal
            if let Ok(canonical) = fs::canonicalize(&full_path).await {
                if !canonical.starts_with(&w.path) {
                    warn!("Acesso restrito abortado (Anti-Traversal)");
                    return Json(serde_json::json!({"error": "Path traversal detected"})).into_response();
                }
            } else {
                 return Json(serde_json::json!({"error": "File not found"})).into_response();
            }

            match fs::read_to_string(&full_path).await {
                Ok(content) => {
                    return Json(serde_json::json!({"status": "success", "content": content})).into_response();
                },
                Err(e) => {
                    return Json(serde_json::json!({"error": format!("Read error: {}", e)})).into_response();
                }
            }
        }
    
    Json(serde_json::json!({"error": "Workspace not found"})).into_response()
}

pub async fn create_kanban_task_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateKanbanTaskRequest>,
) -> impl IntoResponse {
    info!("🤖 Agentic Call: CreateTask -> [{}] {}", payload.project_id, payload.title);
    
    let task_id = Uuid::new_v4().to_string();
    let tenant = "default".to_string();
    let status = "To Do".to_string();
    
    // 1. Resolve Project ID (UUID ou Name)
    let mut final_project_id = payload.project_id.clone();
    
    // Tenta encontrar o projeto pelo ID ou Nome
    let existing_project = sqlx::query_as::<_, (String,)>("SELECT id FROM projects WHERE id = ? OR name = ? LIMIT 1")
        .bind(&final_project_id)
        .bind(&final_project_id)
        .fetch_optional(&state.db)
        .await;
        
    match existing_project {
        Ok(Some((id,))) => {
            final_project_id = id;
        },
        _ => {
            // Cria um novo projeto com o nome fornecido pelo Agent
            let new_project_id = Uuid::new_v4().to_string();
            let _ = sqlx::query("INSERT INTO projects (id, tenant_id, name, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(&new_project_id)
                .bind(&tenant)
                .bind(&final_project_id)
                .execute(&state.db)
                .await;
            final_project_id = new_project_id;
        }
    }

    let result = sqlx::query(
        "INSERT INTO tasks (id, project_id, tenant_id, title, description, status, priority, order_index) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&task_id)
    .bind(&final_project_id)
    .bind(&tenant)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&status)
    .bind(payload.priority.unwrap_or_else(|| "Low".to_string()))
    .bind(0)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({
            "status": "success",
            "task_id": task_id,
            "title": payload.title,
        })).into_response(),
        Err(e) => {
             error!("Agentic Tool SQL Error - Create Task: {}", e);
             Json(serde_json::json!({"error": "Failed to create task in DB"})).into_response()
        }
    }
}