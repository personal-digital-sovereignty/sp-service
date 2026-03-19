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

    // 4. Memory Probe (Linux Native `/proc/meminfo`)
    let mut ram_mb = 0;
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        ram_mb = kb / 1024;
                        break;
                    }
                }
            }
        }
    }

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
