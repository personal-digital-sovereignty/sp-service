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
    pub status: Option<String>,
    pub resolution_content: Option<String>,
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
        "SELECT id, query, frequency, context, sentiment, status, resolution_content FROM knowledge_gaps ORDER BY frequency DESC LIMIT 50"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();


    Json(gaps)
}

#[derive(Deserialize)]
pub struct ResolveGapPayload {
    pub resolution_content: String,
}

pub async fn resolve_knowledge_gap_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<ResolveGapPayload>
) -> Json<serde_json::Value> {
    // 1. Mark as resolved in SQLite
    let res = sqlx::query("UPDATE knowledge_gaps SET status = 'resolved', resolution_content = ? WHERE id = ?")
        .bind(&payload.resolution_content)
        .bind(&id)
        .execute(&state.db)
        .await;

    // 2. Fetch the original query to format the Markdown safely
    let query_str = sqlx::query_scalar::<_, String>("SELECT query FROM knowledge_gaps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "Unknown_Gap".to_string());

    // 3. Artifact Routing: Save the Markdown to [Vault]/gaps/
    if res.as_ref().map(|x| x.rows_affected() > 0).unwrap_or(false) {
        let safe_filename = query_str.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>()
            .split_at(std::cmp::min(50, query_str.len())).0.to_string();
            
        let gaps_dir = state.vault_path.join("gaps");
        let _ = tokio::fs::create_dir_all(&gaps_dir).await;
        
        let md_path = gaps_dir.join(format!("{}_{}.md", safe_filename, uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()));
        
        let md_content = format!(
            "# RAG Knowledge Gap Restored\n\n**Original Query:** {}\n**Gap ID:** {}\n\n## Resolution Context:\n\n{}\n", 
            query_str, id, payload.resolution_content
        );
        
        if let Err(e) = tokio::fs::write(&md_path, md_content).await {
            tracing::error!("❌ [Vault Router] Failed to write Gap artifact to {:?}: {}", md_path, e);
        } else {
            tracing::info!("✅ [Vault Router] Gap artifact persisted: {:?}", md_path);
        }
        
        Json(serde_json::json!({"status": "resolved", "artifact_path": md_path.to_string_lossy().to_string()}))
    } else {
        Json(serde_json::json!({"error": true, "message": "Gap not found or already deleted"}))
    }
}

pub async fn delete_knowledge_gap_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>
) -> Json<serde_json::Value> {
    let res = sqlx::query("DELETE FROM knowledge_gaps WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await;

    match res {
        Ok(exec) if exec.rows_affected() > 0 => Json(serde_json::json!({"status": "deleted"})),
        Ok(_) => Json(serde_json::json!({"error": true, "message": "Gap not found or already resolved"})),
        Err(e) => Json(serde_json::json!({"error": true, "message": format!("DB Error: {}", e)}))
    }
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

#[derive(Serialize, Deserialize, Clone)]
pub struct RagGraphNode {
    pub id: String,
    pub name: String,
    pub val: f64,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RagGraphLink {
    pub source: String,
    pub target: String,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RagGraphResponse {
    pub nodes: Vec<RagGraphNode>,
    pub links: Vec<RagGraphLink>,
}

pub async fn rag_graph_handler(
    State(state): State<Arc<AppState>>,
) -> Json<RagGraphResponse> {
    let docs = sqlx::query("SELECT id, file_path FROM sensus_documents LIMIT 50")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut nodes = Vec::new();
    let mut links = Vec::new();

    nodes.push(RagGraphNode {
        id: "core_brain".to_string(),
        name: "Sovereign Cognitive Core".to_string(),
        val: 18.0,
        r#type: "core".to_string(),
    });

    for doc in &docs {
        let doc_id: String = doc.get("id");
        let doc_path: String = doc.get("file_path");
        let filename = doc_path.split('/').next_back().unwrap_or(&doc_path).to_string();
        let node_id = format!("doc_{}", doc_id);
        
        nodes.push(RagGraphNode {
            id: node_id.clone(),
            name: filename,
            val: 5.0,
            r#type: "document".to_string(),
        });

        links.push(RagGraphLink {
            source: "core_brain".to_string(),
            target: node_id,
            r#type: "synapse".to_string(),
        });
    }

    let gaps = sqlx::query("SELECT id, query FROM knowledge_gaps LIMIT 5")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    for gap in &gaps {
        let gap_id: String = gap.get("id");
        let query: String = gap.get("query");
        let node_id = format!("gap_{}", gap_id);
        
        nodes.push(RagGraphNode {
            id: node_id.clone(),
            name: format!("Gap: {}", query),
            val: 8.0,
            r#type: "gap".to_string(),
        });

        links.push(RagGraphLink {
            source: "core_brain".to_string(),
            target: node_id,
            r#type: "unresolved_gap".to_string(),
        });
    }

    Json(RagGraphResponse { nodes, links })
}

