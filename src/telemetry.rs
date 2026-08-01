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
    pub gpu_vram_used_mb: u64,
    pub unified_memory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub total_tokens: usize,
    pub avg_tps: f64,
    pub avg_latency_ms: u128,
    pub avg_ttft_ms: u128,
    pub estimated_cost: f64,
    pub avg_cloud_cost_per_1k: f64,
    pub models_usage: HashMap<String, usize>,
    pub hardware: HardwareSnapshot,
}

pub struct TelemetryState {
    pub total_tokens: usize,
    pub estimated_cost: f64,
    // Buffer para armazenar as ultimas N sessoes (tokens, millis, ttft_ms)
    recent_sessions: VecDeque<(usize, u128, u128)>,
    pub models_usage: HashMap<String, usize>,
    pub live_tps: f64,
    
    // Hardware Sensors (Requires mutable access for diffing)
    pub sys: System,
    pub networks: Networks,
    pub gpu_name: String,
    pub gpu_vram_total_mb: u64,
    pub gpu_vram_used_mb: u64,
    pub unified_memory: bool,
    pub avg_cloud_cost_per_1k: f64,
}

impl TelemetryState {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh(true);

        let hardware_data = crate::hardware::capture_hardware_telemetry();
        let gpu_name = hardware_data.gpu_name;
        let gpu_vram_total_mb = (hardware_data.total_vram_gb * 1024.0) as u64;
        let gpu_vram_used_mb = (hardware_data.used_vram_gb * 1024.0) as u64;
        let unified_memory = hardware_data.unified_memory;

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
            gpu_vram_used_mb,
            unified_memory,
            avg_cloud_cost_per_1k: 0.00625, // Default fallback
        }
    }

    #[allow(unused_assignments)]
    pub fn record_session(&mut self, tokens: usize, duration_ms: u128, ttft_ms: u128, model: &str) {
        self.total_tokens += tokens;
        
        // Simula A Economia Diária: Se é modelo Local (livre de taxas), o custo que EXISTIRIA na Cloud 
        // conta como "Economia" baseada no market pricing matrix.
        let mut cost_per_1k = 0.0;
        let m_lower = model.to_lowercase();
        if m_lower.contains("gpt-4") {
            cost_per_1k = 0.0300;
        } else if m_lower.contains("claude") {
            cost_per_1k = 0.0150;
        } else if m_lower.contains("openrouter") {
            // Estimativa conservadora para modelos variados via OpenRouter
            cost_per_1k = 0.0020; 
        } else {
            // Local Sovereign Model -> Nós geramos ECONOMIA (Savings) baseada no Cloud Benchmark
            cost_per_1k = self.avg_cloud_cost_per_1k;
        }
        self.estimated_cost += (tokens as f64 / 1000.0) * cost_per_1k;

        *self.models_usage.entry(model.to_string()).or_insert(0) += tokens;
        
        // Mantém apenas as ultimas 10 interacoes para média móvel TPS e TTFT
        if self.recent_sessions.len() >= 10 {
            self.recent_sessions.pop_front();
        }
        self.recent_sessions.push_back((tokens, duration_ms, ttft_ms));
        
        self.live_tps = 0.0;
    }

    pub fn refresh_hardware(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.networks.refresh(true); // Refreshes network usage stats

        // GAP-01: Refresh dynamic VRAM usage from sysfs on each poll cycle
        let hw = crate::hardware::capture_hardware_telemetry();
        self.gpu_vram_used_mb = (hw.used_vram_gb * 1024.0) as u64;
        self.gpu_vram_total_mb = (hw.total_vram_gb * 1024.0) as u64;
        self.gpu_name = hw.gpu_name;
        self.unified_memory = hw.unified_memory;
    }

    pub fn get_snapshot(&self) -> TelemetrySnapshot {
        let mut tps = 0.0;
        let mut avg_latency = 0;
        let mut avg_ttft = 0;
        if !self.recent_sessions.is_empty() {
            let mut sum_tps = 0.0;
            let mut sum_lat = 0;
            let mut sum_ttft = 0;
            let mut count = 0;
            for (t, d, ttft) in &self.recent_sessions {
                if *d > 0 {
                    let sec = *d as f64 / 1000.0;
                    sum_tps += *t as f64 / sec;
                    sum_lat += *d;
                    sum_ttft += *ttft;
                    count += 1;
                }
            }
            if count > 0 {
                tps = sum_tps / count as f64;
                avg_latency = sum_lat / count as u128;
                avg_ttft = sum_ttft / count as u128;
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
            avg_ttft_ms: avg_ttft,
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
                gpu_vram_used_mb: self.gpu_vram_used_mb,
                unified_memory: self.unified_memory,
            }
        }
    }
}
