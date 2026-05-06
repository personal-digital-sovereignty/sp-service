use axum::Json;
use serde::{Deserialize, Serialize};
use std::env::consts::OS;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HardwareProfile {
    pub os_name: String,
    pub has_gpu: bool,
    pub has_npu: bool,
    pub is_sandbox_isolated: bool,
    pub accepts_agent_delegation: bool,
    pub available_ram_mb: u64,
}

pub async fn mesh_handshake_handler() -> Json<HardwareProfile> {
    // 1. GPU Check (Probing PCIe for NVIDIA presence)
    let has_gpu = Command::new("nvidia-smi")
        .arg("--query-gpu=name")
        .arg("--format=csv,noheader")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // 2. NPU Check (Probing standard Linux NPU devices)
    let has_npu = std::path::Path::new("/dev/accel/accel0").exists(); 

    // 3. Sandbox Isolation (Via Engine ENV)
    let is_sandbox = std::env::var("SOVEREIGN_RUN_ENV").unwrap_or_default() == "sandbox";

    // O.S Manual Override (Anti-Interferência Humana)
    // Permite que o operador humano tranque o nó para evitar lags em processamentos massivos
    let accepts_jobs = std::env::var("SOVEREIGN_REJECT_MESH_ROUTING").unwrap_or_default() != "true";

    // 4. Memory Probe — cross-platform
    // Linux: lê /proc/meminfo diretamente (mais barato que sysinfo para este caso)
    // MacOS/Windows: fallback via sysinfo (já instanciado pelo telemetry.rs)
    let ram_mb = {
        #[cfg(target_os = "linux")]
        {
            let mut mem = 0u64;
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<u64>() {
                                mem = kb / 1024;
                                break;
                            }
                        }
                    }
                }
            }
            mem
        }
        #[cfg(not(target_os = "linux"))]
        {
            use sysinfo::System;
            let mut sys = System::new();
            sys.refresh_memory();
            sys.total_memory() / (1024 * 1024)
        }
    };

    Json(HardwareProfile {
        os_name: OS.to_string(),
        has_gpu,
        has_npu,
        is_sandbox_isolated: is_sandbox,
        accepts_agent_delegation: accepts_jobs,
        available_ram_mb: ram_mb,
    })
}

#[derive(Deserialize)]
pub struct MeshConnectRequest {
    pub remote_ip: String,
    pub remote_user: String,
    pub key_path: String,
    pub local_port: u16,
    pub remote_port: u16, // usually 38001
}

pub async fn mesh_connect_handler(axum::Json(payload): axum::Json<MeshConnectRequest>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    match crate::ssh_mesh_connector::MeshConnector::establish_mesh_tunnel(
        payload.remote_ip,
        payload.remote_user,
        payload.key_path,
        payload.local_port,
        payload.remote_port
    ).await {
        Ok(_) => Ok(Json(serde_json::json!({"status": "Tunneling Initiated", "local_bind_port": payload.local_port}))),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub async fn mesh_tunnels_status_handler() -> Json<serde_json::Value> {
    let tunnels = crate::ssh_mesh_connector::ACTIVE_MESH_TUNNELS.lock().await;
    let mut response = Vec::new();
    for (port, (uri, _key)) in tunnels.iter() {
        response.push(serde_json::json!({
            "local_port": port,
            "target_uri": uri,
            "status": "established"
        }));
    }
    Json(serde_json::json!({"active_tunnels": response}))
}

// ==========================================
// MESH P2P: SYNCHRONIZATION ENGINE
// ==========================================

#[derive(Deserialize, Clone)]
pub struct MeshSyncChunk {
    pub uuid_reference: String,
    pub text_content: String,
    pub metadata_json: String,
}

#[derive(Deserialize, Clone)]
pub struct MeshSyncDocument {
    pub document_id: String,
    pub workspace_name: String,
    pub file_path: String,
    pub content_raw: String,
    pub summary: String,
    pub chunks: Vec<MeshSyncChunk>,
}

pub async fn mesh_sync_document_handler(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::AppState>>,
    axum::Json(payload): axum::Json<MeshSyncDocument>
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    tracing::info!("📡 [Mesh P2P] Inbound Sync: Received document '{}' from Sovereign Peer.", payload.file_path);

    let db = &state.db;

    // Resolve or Create Workspace automatically based on peering Name
    let workspace_id: String = sqlx::query_scalar("SELECT id FROM workspaces WHERE name = ? LIMIT 1")
        .bind(&payload.workspace_name)
        .fetch_optional(db).await.unwrap_or_default()
        .unwrap_or_else(|| {
            // Se o nó não possui um Workspace com este nome explícito, usamos o MESH Default
            tracing::warn!("⚠️ [Mesh P2P] Workspace '{}' inexistente no nó local. Injetando no Mesh Pool.", payload.workspace_name);
            "mesh_roaming".to_string()
        });

    if workspace_id == "mesh_roaming" {
        let _ = sqlx::query("
            INSERT OR IGNORE INTO workspaces (id, name, absolute_path) 
            VALUES ('mesh_roaming', 'Sovereign Mesh Roaming', '/dev/null/mesh')
        ").execute(db).await;
    }

    // Upsert Document
    let _ = sqlx::query("
        INSERT INTO sensus_documents (id, workspace_id, file_path, content_raw, summary, last_modified)
        VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(file_path) DO UPDATE SET 
            content_raw = excluded.content_raw,
            summary = excluded.summary,
            last_modified = CURRENT_TIMESTAMP
    ")
    .bind(&payload.document_id)
    .bind(&workspace_id)
    .bind(&payload.file_path)
    .bind(&payload.content_raw)
    .bind(&payload.summary)
    .execute(db).await;

    // Delete existing chunks for this document on Peer override
    let _ = sqlx::query("DELETE FROM sovereign_chunks WHERE file_path = ?").bind(&payload.file_path).execute(db).await;

    // Insert Chunks
    for chunk in payload.chunks {
        let _ = sqlx::query("
            INSERT INTO sovereign_chunks (uuid_reference, workspace_id, file_path, text_content, metadata_json)
            VALUES (?, ?, ?, ?, ?)
        ")
        .bind(&chunk.uuid_reference)
        .bind(&workspace_id)
        .bind(&payload.file_path)
        .bind(&chunk.text_content)
        .bind(&chunk.metadata_json)
        .execute(db).await;
    }

    Ok(Json(serde_json::json!({"status": "synchronized"})))
}
