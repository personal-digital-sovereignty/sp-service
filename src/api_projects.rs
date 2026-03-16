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
    // Omitimos links e logs do frontend por performance bruta, Front aceita vazio.
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
        "SELECT id, tenant_id, name, purpose, traction_status, next_action, energy_level, progress_percent, friction_radar, deadline FROM projects ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(projects) => {
            // Frontend Pinia Espera 'links' e 'logs' no objeto, injectamos o MOCK pra evitar crash do VueJS
            let mut enhanced = Vec::new();
            for p in projects {
                enhanced.push(serde_json::json!({
                    "id": p.id,
                    "tenant_id": p.tenant_id,
                    "name": p.name,
                    "purpose": p.purpose,
                    "traction_status": p.traction_status,
                    "next_action": p.next_action,
                    "energy_level": p.energy_level,
                    "progress_percent": p.progress_percent,
                    "friction_radar": p.friction_radar,
                    "deadline": p.deadline,
                    "links": [],
                    "logs": []
                }));
            }
            Json(serde_json::Value::Array(enhanced)).into_response()
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
        r#"INSERT INTO projects (id, tenant_id, name, purpose, traction_status, energy_level, progress_percent) 
           VALUES (?, ?, ?, ?, ?, ?, ?)"#
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

    // Responde com o objeto recém criado simulado para o Pinia Add Store
    Json(serde_json::json!({
        "id": proj_id,
        "tenant_id": tenant,
        "name": payload.name,
        "purpose": payload.purpose,
        "traction_status": "Ideation",
        "energy_level": "Med",
        "progress_percent": 0,
        "links": [],
        "logs": []
    })).into_response()
}

pub async fn delete_project_handler(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let _ = sqlx::query("DELETE FROM projects WHERE id = ?").bind(id).execute(&state.db).await;
    Json(serde_json::json!({"status": "deleted"})).into_response()
}

// ----------------- TASKS (Kanban) ---------------------------

pub async fn get_project_tasks_handler(
    Path(project_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, TaskRow>(
        "SELECT id, project_id, tenant_id, title, description, status, priority, order_index, deadline FROM tasks WHERE project_id = ? ORDER BY order_index ASC"
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
    let old_task = sqlx::query_as::<_, TaskRow>("SELECT id, project_id, tenant_id, title, description, status, priority, order_index, deadline FROM tasks WHERE id = ?")
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
