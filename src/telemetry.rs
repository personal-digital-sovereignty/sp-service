use serde::{Deserialize, Serialize};
use std::collections::{VecDeque, HashMap};
use sysinfo::{System, Networks};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSnapshot {
    pub cpu_cores: Vec<f32>,
    pub ram_usage_mb: f64,
    pub ram_total_gb: f64,
    pub io_rx_bytes: u64,
    pub io_tx_bytes: u64,
    pub gpu_name: String,
    pub gpu_vram_total_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub total_tokens: usize,
    pub avg_tps: f64,
    pub avg_latency_ms: u128,
    pub estimated_cost: f64,
    pub avg_cloud_cost_per_1k: f64,
    pub models_usage: HashMap<String, usize>,
    pub hardware: HardwareSnapshot,
}

pub struct TelemetryState {
    pub total_tokens: usize,
    pub estimated_cost: f64,
    // Buffer para armazenar as ultimas N sessoes (tokens, millis)
    recent_sessions: VecDeque<(usize, u128)>,
    pub models_usage: HashMap<String, usize>,
    pub live_tps: f64,
    
    // Hardware Sensors (Requires mutable access for diffing)
    pub sys: System,
    pub networks: Networks,
    pub gpu_name: String,
    pub gpu_vram_total_mb: u64,
    pub avg_cloud_cost_per_1k: f64,
}

impl TelemetryState {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh(true);

        let mut gpu_name = String::from("GPU Compute");
        let mut gpu_vram_total_mb = 0;

        #[cfg(target_os = "linux")]
        if let Ok(output) = std::process::Command::new("glxinfo").arg("-B").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let text = line.trim();
                if text.starts_with("Device: ") {
                    if let Some(name) = text.split(" (").next() {
                        gpu_name = name.replace("Device: ", "").to_string();
                    }
                } else if text.starts_with("Dedicated video memory: ")
                    && let Some(mb_str) = text.split(':').nth(1)
                        && let Ok(val) = mb_str.replace("MB", "").trim().parse::<u64>() {
                            gpu_vram_total_mb = val;
                        }
            }
        }

        #[cfg(target_os = "macos")]
        if let Ok(output) = std::process::Command::new("system_profiler").arg("SPDisplaysDataType").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let text = line.trim();
                if text.starts_with("Chipset Model: ") {
                    gpu_name = text.replace("Chipset Model: ", "").to_string();
                } else if text.starts_with("VRAM (Total): ") || text.starts_with("VRAM (Dynamic, Max): ") {
                    if let Some(gb_str) = text.split(':').nth(1) {
                        if let Ok(val) = gb_str.replace("GB", "").trim().parse::<u64>() {
                            gpu_vram_total_mb = val * 1024;
                        }
                    }
                }
            }
        }

        Self {
            total_tokens: 0,
            estimated_cost: 0.0,
            recent_sessions: VecDeque::with_capacity(10),
            models_usage: HashMap::new(),
            live_tps: 0.0,
            sys,
            networks,
            gpu_name,
            gpu_vram_total_mb,
            avg_cloud_cost_per_1k: 0.00625, // Default fallback
        }
    }

    #[allow(unused_assignments)]
    pub fn record_session(&mut self, tokens: usize, duration_ms: u128, model: &str) {
        self.total_tokens += tokens;
        
        // Simula A Economia Diária: Se é modelo Local (livre de taxas), o custo que EXISTIRIA na Cloud 
        // conta como "Economia" baseada no market pricing matrix.
        let mut cost_per_1k = 0.0;
        if model.to_lowercase().contains("gpt-4") {
            cost_per_1k = 0.0300;
        } else if model.to_lowercase().contains("claude") {
            cost_per_1k = 0.0150;
        } else {
            // Local Sovereign Model -> Nós geramos ECONOMIA (Savings) baseada no Cloud Benchmark
            cost_per_1k = self.avg_cloud_cost_per_1k;
        }
        self.estimated_cost += (tokens as f64 / 1000.0) * cost_per_1k;

        *self.models_usage.entry(model.to_string()).or_insert(0) += tokens;
        
        // Mantém apenas as ultimas 10 interacoes para média móvel TPS
        if self.recent_sessions.len() >= 10 {
            self.recent_sessions.pop_front();
        }
        self.recent_sessions.push_back((tokens, duration_ms));
        
        self.live_tps = 0.0;
    }

    pub fn refresh_hardware(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.networks.refresh(true); // Refreshes network usage stats
    }

    pub fn get_snapshot(&self) -> TelemetrySnapshot {
        let mut tps = 0.0;
        let mut avg_latency = 0;
        if !self.recent_sessions.is_empty() {
            let mut sum_tps = 0.0;
            let mut sum_lat = 0;
            let mut count = 0;
            for (t, d) in &self.recent_sessions {
                if *d > 0 {
                    let sec = *d as f64 / 1000.0;
                    sum_tps += *t as f64 / sec;
                    sum_lat += *d;
                    count += 1;
                }
            }
            if count > 0 {
                tps = sum_tps / count as f64;
                avg_latency = sum_lat / count as u128;
            }
        }
        
        if self.live_tps > 0.0 {
            tps = self.live_tps;
        }

        // Extract CPU Cores Array
        let mut cpu_cores = Vec::new();
        for core in self.sys.cpus() {
            cpu_cores.push((core.cpu_usage() * 10.0).round() / 10.0);
        }

        // Memory Conversion
        let ram_usage_mb = (self.sys.total_memory() - self.sys.available_memory()) as f64 / 1024.0 / 1024.0;
        let ram_total_gb = self.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

        // Network Aggregate
        let mut io_rx_bytes = 0;
        let mut io_tx_bytes = 0;
        for (_interface_name, data) in &self.networks {
            io_rx_bytes += data.received();
            io_tx_bytes += data.transmitted();
        }

        TelemetrySnapshot {
            total_tokens: self.total_tokens,
            avg_tps: (tps * 100.0).round() / 100.0, // Arredonda 2 casas
            avg_latency_ms: avg_latency,
            estimated_cost: (self.estimated_cost * 10000.0).round() / 10000.0,
            avg_cloud_cost_per_1k: self.avg_cloud_cost_per_1k,
            models_usage: self.models_usage.clone(),
            hardware: HardwareSnapshot {
                cpu_cores,
                ram_usage_mb: (ram_usage_mb * 100.0).round() / 100.0,
                ram_total_gb: (ram_total_gb * 100.0).round() / 100.0,
                io_rx_bytes,
                io_tx_bytes,
                gpu_name: self.gpu_name.clone(),
                gpu_vram_total_mb: self.gpu_vram_total_mb,
            }
        }
    }
}
