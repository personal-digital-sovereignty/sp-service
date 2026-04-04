use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use tracing::error;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub purpose: Option<String>,
    pub traction_status: Option<String>,
    pub next_action: Option<String>,
    pub energy_level: Option<String>,
    pub progress_percent: Option<i64>,
    pub friction_radar: Option<String>,
    pub deadline: Option<String>,
    pub is_archived: Option<bool>,
    pub columns_json: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct TaskRow {
    pub id: String,
    pub project_id: String,
    pub tenant_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub order_index: Option<i64>,
    pub deadline: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub purpose: Option<String>,
    pub traction_status: Option<String>,
    pub energy_level: Option<String>,
    pub progress_percent: Option<i64>,
}

pub async fn get_projects_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, tenant_id, name, purpose, traction_status, next_action, energy_level, progress_percent, friction_radar, deadline, is_archived, columns_json, created_at FROM projects ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(projects) => {
            Json(projects).into_response()
        },
        Err(e) => {
            error!("SQLx Error lendo Projects: {}", e);
            Json(serde_json::json!([])).into_response()
        }
    }
}

pub async fn create_project_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    let proj_id = Uuid::new_v4().to_string();
    let tenant = "default".to_string();

    let _ = sqlx::query(
        r#"INSERT INTO projects (id, tenant_id, name, purpose, traction_status, energy_level, progress_percent, is_archived, columns_json) 
           VALUES (?, ?, ?, ?, ?, ?, ?, 0, '["To Do", "In Progress", "Done"]')"#
    )
    .bind(&proj_id)
    .bind(&tenant)
    .bind(&payload.name)
    .bind(&payload.purpose)
    .bind(payload.traction_status.unwrap_or_else(|| "Ideation".to_string()))
    .bind(payload.energy_level.unwrap_or_else(|| "Med".to_string()))
    .bind(payload.progress_percent.unwrap_or(0))
    .execute(&state.db)
    .await;

    // Responde com o objeto recém criado para o Svelte Store
    Json(serde_json::json!({
        "id": proj_id,
        "tenant_id": tenant,
        "name": payload.name,
        "purpose": payload.purpose,
        "traction_status": "Ideation",
        "energy_level": "Med",
        "progress_percent": 0,
        "is_archived": false,
        "columns_json": "[\"To Do\", \"In Progress\", \"Done\"]",
        "created_at": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    })).into_response()
}

pub async fn delete_project_handler(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let _ = sqlx::query("DELETE FROM projects WHERE id = ?").bind(id).execute(&state.db).await;
    Json(serde_json::json!({"status": "deleted"})).into_response()
}

#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub purpose: Option<String>,
    pub traction_status: Option<String>,
    pub next_action: Option<String>,
    pub energy_level: Option<String>,
    pub progress_percent: Option<i64>,
    pub friction_radar: Option<String>,
    pub deadline: Option<String>,
    pub is_archived: Option<bool>,
    pub columns_json: Option<String>,
}

pub async fn update_project_handler(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateProjectRequest>,
) -> impl IntoResponse {
    let old_proj = sqlx::query_as::<_, ProjectRow>("SELECT id, tenant_id, name, purpose, traction_status, next_action, energy_level, progress_percent, friction_radar, deadline, is_archived, columns_json, created_at FROM projects WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await;

    if let Ok(Some(proj)) = old_proj {
        let final_name = payload.name.unwrap_or(proj.name);
        let final_purpose = payload.purpose.or(proj.purpose);
        let final_traction = payload.traction_status.or(proj.traction_status);
        let final_action = payload.next_action.or(proj.next_action);
        let final_energy = payload.energy_level.or(proj.energy_level);
        let final_progress = payload.progress_percent.or(proj.progress_percent).unwrap_or(0);
        let final_friction = payload.friction_radar.or(proj.friction_radar);
        let final_deadline = payload.deadline.or(proj.deadline);
        let final_archived = payload.is_archived.or(proj.is_archived).unwrap_or(false);
        let final_columns = payload.columns_json.or(proj.columns_json).unwrap_or_else(|| "[\"To Do\", \"In Progress\", \"Done\"]".to_string());

        let _ = sqlx::query(
            "UPDATE projects SET name = ?, purpose = ?, traction_status = ?, next_action = ?, energy_level = ?, progress_percent = ?, friction_radar = ?, deadline = ?, is_archived = ?, columns_json = ? WHERE id = ?"
        )
        .bind(&final_name)
        .bind(&final_purpose)
        .bind(&final_traction)
        .bind(&final_action)
        .bind(&final_energy)
        .bind(final_progress)
        .bind(&final_friction)
        .bind(&final_deadline)
        .bind(final_archived)
        .bind(&final_columns)
        .bind(&id)
        .execute(&state.db)
        .await;

        return Json(serde_json::json!({
            "id": id,
            "tenant_id": proj.tenant_id,
            "name": final_name,
            "purpose": final_purpose,
            "traction_status": final_traction,
            "next_action": final_action,
            "energy_level": final_energy,
            "progress_percent": final_progress,
            "friction_radar": final_friction,
            "deadline": final_deadline,
            "is_archived": final_archived,
            "columns_json": final_columns,
            "created_at": proj.created_at
        })).into_response()
    }
    
    (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": true, "message": "Project not found."}))).into_response()
}

// ----------------- PROJECT DOCUMENTS ------------------------

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct ProjectDocumentRow {
    pub project_id: String,
    pub file_path: String,
}

pub async fn get_project_documents_handler(
    Path(project_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, ProjectDocumentRow>("SELECT project_id, file_path FROM project_documents WHERE project_id = ?")
        .bind(&project_id)
        .fetch_all(&state.db)
        .await;

    match rows {
        Ok(docs) => Json(docs).into_response(),
        Err(e) => {
            tracing::error!("SQLx Error lendo Project Documents: {}", e);
            Json(serde_json::json!([])).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct LinkDocumentRequest {
    pub file_path: String,
}

pub async fn link_project_document_handler(
    Path(project_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LinkDocumentRequest>,
) -> impl IntoResponse {
    let _ = sqlx::query("INSERT OR IGNORE INTO project_documents (project_id, file_path) VALUES (?, ?)")
        .bind(&project_id)
        .bind(&payload.file_path)
        .execute(&state.db)
        .await;

    Json(serde_json::json!({"status": "linked", "file_path": payload.file_path})).into_response()
}

pub async fn unlink_project_document_handler(
    Path((project_id, encoded_path)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let file_path = urlencoding::decode(&encoded_path).unwrap_or(std::borrow::Cow::Borrowed(&encoded_path)).to_string();
    let _ = sqlx::query("DELETE FROM project_documents WHERE project_id = ? AND file_path = ?")
        .bind(&project_id)
        .bind(&file_path)
        .execute(&state.db)
        .await;

    Json(serde_json::json!({"status": "unlinked"})).into_response()
}

// ----------------- TASKS (Kanban) ---------------------------

pub async fn get_project_tasks_handler(
    Path(project_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, TaskRow>(
        "SELECT id, project_id, tenant_id, title, description, status, priority, order_index, deadline, created_at FROM tasks WHERE project_id = ? ORDER BY order_index ASC"
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(tasks) => Json(tasks).into_response(),
        Err(e) => {
            error!("SQLx Error lendo Tasks do Projeto: {}", e);
            Json(serde_json::json!([])).into_response()
        }
    }
}

pub async fn delete_task_handler(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let _ = sqlx::query("DELETE FROM tasks WHERE id = ?").bind(id).execute(&state.db).await;
    Json(serde_json::json!({"status": "deleted"})).into_response()
}

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub order_index: Option<i64>,
    pub deadline: Option<String>,
}

pub async fn update_task_handler(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    // Busca os dados antigos como baseline para update parcial
    let old_task = sqlx::query_as::<_, TaskRow>("SELECT id, project_id, tenant_id, title, description, status, priority, order_index, deadline, created_at FROM tasks WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await;

    if let Ok(Some(task)) = old_task {
        let final_title = payload.title.unwrap_or(task.title);
        let final_desc = payload.description.or(task.description);
        let final_status = payload.status.or(task.status);
        let final_priority = payload.priority.or(task.priority);
        let final_order = payload.order_index.or(task.order_index).unwrap_or(0);
        let final_deadline = payload.deadline.or(task.deadline);

        let _ = sqlx::query(
            "UPDATE tasks SET title = ?, description = ?, status = ?, priority = ?, order_index = ?, deadline = ? WHERE id = ?"
        )
        .bind(&final_title)
        .bind(&final_desc)
        .bind(&final_status)
        .bind(&final_priority)
        .bind(final_order)
        .bind(&final_deadline)
        .bind(&id)
        .execute(&state.db)
        .await;

        return Json(serde_json::json!({
            "id": id,
            "project_id": task.project_id,
            "tenant_id": task.tenant_id,
            "title": final_title,
            "description": final_desc,
            "status": final_status,
            "priority": final_priority,
            "order_index": final_order,
            "deadline": final_deadline
        })).into_response()
    }
    
    (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": true, "message": "Task not found."}))).into_response()
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub deadline: Option<String>,
}

pub async fn create_task_handler(
    Path(project_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    let task_id = uuid::Uuid::new_v4().to_string();
    let tenant = "default".to_string();

    let final_status = payload.status.clone().unwrap_or_else(|| "TODO".to_string());
    let final_priority = payload.priority.clone().unwrap_or_else(|| "Medium".to_string());

    let _ = sqlx::query(
        r#"INSERT INTO tasks (id, project_id, tenant_id, title, description, status, priority, order_index, deadline) 
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&task_id)
    .bind(&project_id)
    .bind(&tenant)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&final_status)
    .bind(&final_priority)
    .bind(0)
    .bind(&payload.deadline)
    .execute(&state.db)
    .await;

    Json(serde_json::json!({
        "id": task_id,
        "project_id": project_id,
        "tenant_id": tenant,
        "title": payload.title,
        "description": payload.description,
        "status": final_status,
        "priority": final_priority,
        "order_index": 0,
        "deadline": payload.deadline
    })).into_response()
}
