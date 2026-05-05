#[cfg(test)]
mod tests {
    use crate::telemetry::TelemetryState;

    #[test]
    fn test_telemetry_state_new() {
        let state = TelemetryState::new();
        assert_eq!(state.total_tokens, 0);
        assert_eq!(state.estimated_cost, 0.0);
        assert!(state.models_usage.is_empty());
        assert_eq!(state.live_tps, 0.0);
        
        let snapshot = state.get_snapshot();
        assert_eq!(snapshot.total_tokens, 0);
        assert_eq!(snapshot.avg_tps, 0.0);
        assert_eq!(snapshot.avg_latency_ms, 0);
    }

    #[test]
    fn test_record_session_local_model() {
        let mut state = TelemetryState::new();
        
        // Simulating a local model (generates savings based on cloud fallback)
        state.record_session(1000, 500, "qwen2.5:latest");
        
        assert_eq!(state.total_tokens, 1000);
        assert!(state.estimated_cost > 0.0); // Should use avg_cloud_cost_per_1k
        assert_eq!(*state.models_usage.get("qwen2.5:latest").unwrap(), 1000);
        
        let snapshot = state.get_snapshot();
        assert_eq!(snapshot.total_tokens, 1000);
        assert_eq!(snapshot.avg_latency_ms, 500);
        // TPS = 1000 tokens / 0.5s = 2000 tps
        assert_eq!(snapshot.avg_tps, 2000.0);
    }

    #[test]
    fn test_record_session_cloud_models() {
        let mut state = TelemetryState::new();
        
        // Simulating GPT-4
        state.record_session(1000, 1000, "gpt-4-turbo");
        assert_eq!(state.estimated_cost, 0.0300);
        
        // Simulating Claude
        state.record_session(2000, 1000, "claude-3-opus");
        // Previous 0.03 + (2 * 0.0150) = 0.0600
        assert_eq!(state.estimated_cost, 0.0600);
        
        // Simulating OpenRouter
        state.record_session(1000, 1000, "openrouter/mistral");
        // Previous 0.06 + (1 * 0.0020) = 0.0620
        // Rounding issues might occur, we use epsilon check
        assert!((state.estimated_cost - 0.0620).abs() < f64::EPSILON);
        
        let snapshot = state.get_snapshot();
        assert_eq!(snapshot.total_tokens, 4000);
    }

    #[test]
    fn test_refresh_hardware() {
        let mut state = TelemetryState::new();
        // Just verify it runs without panicking
        state.refresh_hardware();
        
        let snapshot = state.get_snapshot();
        assert!(snapshot.hardware.ram_total_gb > 0.0);
        assert!(snapshot.hardware.cpu_cores.len() > 0);
    }
}
