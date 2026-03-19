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
        available_ram_mb: ram_mb,
    })
}
