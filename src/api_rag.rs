use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use sqlx::Row;

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct RoutingRule {
    pub id: String,
    pub name: String,
    pub target_model: String,
    pub latency_badge: String,
    pub icon: String,
    pub is_active: bool,
    pub order_index: i32,
}

#[derive(Deserialize)]
pub struct CreateRoutingRulePayload {
    pub name: String,
    pub target_model: String,
    pub latency_badge: String,
    pub icon: String,
}

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct RemoteModel {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub icon_url: Option<String>,
    pub latency_ms: i32,
    pub cost_per_1k: f64,
    pub success_rate: f64,
    pub status: String,
}

#[derive(Deserialize)]
pub struct CreateRemoteModelPayload {
    pub name: String,
    pub provider: String,
    pub latency_ms: i32,
    pub cost_per_1k: f64,
}

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct KnowledgeGap {
    pub id: String,
    pub query: String,
    pub frequency: i32,
    pub context: String,
    pub sentiment: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RadarMetrics {
    pub global_score: i32,
    pub faithfulness: i32,
    pub precision: i32,
}

pub async fn get_routing_rules_handler(State(state): State<Arc<AppState>>) -> Json<Vec<RoutingRule>> {
    let rules = sqlx::query_as::<_, RoutingRule>(
        r#"SELECT id, name, target_model, latency_badge, icon, is_active, order_index FROM routing_rules ORDER BY order_index ASC"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();


    Json(rules)
}

pub async fn create_routing_rule_handler(State(state): State<Arc<AppState>>, Json(payload): Json<CreateRoutingRulePayload>) -> Json<serde_json::Value> {
    let new_id = uuid::Uuid::new_v4().to_string();
    let order: (i32,) = sqlx::query_as("SELECT COALESCE(MAX(order_index), 0) + 1 FROM routing_rules").fetch_one(&state.db).await.unwrap_or((1,));
    
    let res = sqlx::query("INSERT INTO routing_rules (id, name, target_model, latency_badge, icon, is_active, order_index) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&new_id)
        .bind(&payload.name)
        .bind(&payload.target_model)
        .bind(&payload.latency_badge)
        .bind(&payload.icon)
        .bind(true)
        .bind(order.0)
        .execute(&state.db).await;

    if res.is_ok() { Json(serde_json::json!({"success": true, "id": new_id})) } else { Json(serde_json::json!({"success": false})) }
}

pub async fn delete_routing_rule_handler(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let res = sqlx::query("DELETE FROM routing_rules WHERE id = ?").bind(id).execute(&state.db).await;
    if res.is_ok() { Json(serde_json::json!({"success": true})) } else { Json(serde_json::json!({"success": false})) }
}

pub async fn get_remote_models_handler(State(state): State<Arc<AppState>>) -> Json<Vec<RemoteModel>> {
    let models = sqlx::query_as::<_, RemoteModel>(
        r#"SELECT id, name, provider, icon_url, latency_ms, cost_per_1k, success_rate, status FROM remote_models ORDER BY cost_per_1k DESC"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();


    Json(models)
}

pub async fn create_remote_model_handler(State(state): State<Arc<AppState>>, Json(payload): Json<CreateRemoteModelPayload>) -> Json<serde_json::Value> {
    let new_id = uuid::Uuid::new_v4().to_string();
    
    let res = sqlx::query("INSERT INTO remote_models (id, name, provider, icon_url, latency_ms, cost_per_1k, success_rate, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&new_id)
        .bind(&payload.name)
        .bind(&payload.provider)
        .bind(Option::<String>::None)
        .bind(payload.latency_ms)
        .bind(payload.cost_per_1k)
        .bind(1.00) // Default 100% success rate
        .bind("Operational")
        .execute(&state.db).await;

    if res.is_ok() { Json(serde_json::json!({"success": true, "id": new_id})) } else { Json(serde_json::json!({"success": false})) }
}

pub async fn delete_remote_model_handler(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let res = sqlx::query("DELETE FROM remote_models WHERE id = ?").bind(id).execute(&state.db).await;
    if res.is_ok() { Json(serde_json::json!({"success": true})) } else { Json(serde_json::json!({"success": false})) }
}

pub async fn get_knowledge_gaps_handler(State(state): State<Arc<AppState>>) -> Json<Vec<KnowledgeGap>> {
    let gaps = sqlx::query_as::<_, KnowledgeGap>(
        "SELECT id, query, frequency, context, sentiment FROM knowledge_gaps ORDER BY frequency DESC LIMIT 10"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();


    Json(gaps)
}

pub async fn get_radar_metrics_handler(State(state): State<Arc<AppState>>) -> Json<RadarMetrics> {
    let row_result = sqlx::query("SELECT AVG(faithfulness_score) as f_avg, AVG(precision_score) as p_avg FROM evaluations WHERE status = 'completed'")
        .fetch_one(&state.db)
        .await;

    let (mut f_avg, mut p_avg) = (0.0, 0.0);
    if let Ok(row) = row_result {
        f_avg = row.try_get::<f64, _>("f_avg").unwrap_or(0.0);
        p_avg = row.try_get::<f64, _>("p_avg").unwrap_or(0.0);
    }
    
    let f = f_avg as i32;
    let p = p_avg as i32;
    let global = if f == 0 && p == 0 { 0 } else { (f + p) / 2 };

    Json(RadarMetrics {
        global_score: global,
        faithfulness: f,
        precision: p,
    })
}
