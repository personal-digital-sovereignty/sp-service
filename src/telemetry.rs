use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub total_tokens: usize,
    pub avg_tps: f64,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone)]
pub struct TelemetryState {
    pub total_tokens: usize,
    pub estimated_cost: f64,
    // Buffer para armazenar as ultimas N sessoes (tokens, millis)
    recent_sessions: VecDeque<(usize, u128)>,
}

impl TelemetryState {
    pub fn new() -> Self {
        Self {
            total_tokens: 0,
            estimated_cost: 0.0,
            recent_sessions: VecDeque::with_capacity(10),
        }
    }

    pub fn record_session(&mut self, tokens: usize, duration_ms: u128, model: &str) {
        self.total_tokens += tokens;
        
        let mut cost_per_1k = 0.0150;
        if model.to_lowercase().contains("gpt-4") {
            cost_per_1k = 0.0300;
        }
        self.estimated_cost += (tokens as f64 / 1000.0) * cost_per_1k;
        
        // Mantém apenas as ultimas 10 interacoes para média móvel TPS
        if self.recent_sessions.len() >= 10 {
            self.recent_sessions.pop_front();
        }
        self.recent_sessions.push_back((tokens, duration_ms));
    }

    pub fn get_snapshot(&self) -> TelemetrySnapshot {
        let mut tps = 0.0;
        if !self.recent_sessions.is_empty() {
            let mut sum_tps = 0.0;
            let mut count = 0;
            for (t, d) in &self.recent_sessions {
                if *d > 0 {
                    let sec = *d as f64 / 1000.0;
                    sum_tps += *t as f64 / sec;
                    count += 1;
                }
            }
            if count > 0 {
                tps = sum_tps / count as f64;
            }
        }
        
        TelemetrySnapshot {
            total_tokens: self.total_tokens,
            avg_tps: (tps * 100.0).round() / 100.0, // Arredonda 2 casas
            estimated_cost: (self.estimated_cost * 10000.0).round() / 10000.0,
        }
    }
}
