//! ============================================================
//! sp-service — Hardware Tests
//! Tests for context window calculation and hardware telemetry
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::hardware::{calculate_safe_context_window, HardwareTelemetry};

    fn make_telemetry(vram: f64, ram: f64) -> HardwareTelemetry {
        HardwareTelemetry {
            total_ram_gb: ram,
            used_ram_gb: 0.0,
            total_vram_gb: vram,
            used_vram_gb: 0.0,
            gpu_name: String::new(),
            unified_memory: false,
        }
    }

    #[test]
    fn test_context_window_low_ram() {
        let telemetry = make_telemetry(0.0, 4.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 8192);
    }

    #[test]
    fn test_context_window_8gb() {
        let telemetry = make_telemetry(0.0, 8.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 16384);
    }

    #[test]
    fn test_context_window_12gb() {
        let telemetry = make_telemetry(0.0, 12.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 32768);
    }

    #[test]
    fn test_context_window_16gb() {
        let telemetry = make_telemetry(0.0, 16.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 32768);
    }

    #[test]
    fn test_context_window_24gb() {
        let telemetry = make_telemetry(0.0, 24.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 98304);
    }

    #[test]
    fn test_context_window_48gb() {
        let telemetry = make_telemetry(0.0, 48.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 131072);
    }

    #[test]
    fn test_context_window_vram_takes_precedence() {
        let telemetry = make_telemetry(16.0, 64.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 32768);
    }

    #[test]
    fn test_context_window_zero_uses_ram() {
        let telemetry = make_telemetry(0.0, 32.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 98304);
    }

    #[test]
    fn test_context_window_boundary_11gb() {
        let telemetry = make_telemetry(11.9, 0.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 16384);
    }

    #[test]
    fn test_context_window_boundary_23gb() {
        let telemetry = make_telemetry(23.9, 0.0);
        assert_eq!(calculate_safe_context_window(&telemetry), 32768);
    }
}
