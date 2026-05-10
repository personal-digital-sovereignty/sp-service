//! ============================================================
//! sp-service — Guardrails Tests
//! Tests for SSRF validation, security event detection
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::guardrails::{is_safe_url, SecurityEvent};

    // ─────────────────────────────────────────────────────────
    // is_safe_url Tests — SSRF Prevention
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_is_safe_url_public_https() {
        assert!(is_safe_url("https://example.com"));
        assert!(is_safe_url("https://google.com/search?q=test"));
        assert!(is_safe_url("https://api.github.com/repos"));
    }

    #[test]
    fn test_is_safe_url_localhost_blocked() {
        assert!(!is_safe_url("http://localhost:38001"));
        assert!(!is_safe_url("http://127.0.0.1:11434"));
        assert!(!is_safe_url("http://localhost"));
    }

    #[test]
    fn test_is_safe_url_loopback_variants() {
        assert!(!is_safe_url("http://127.0.0.1"));
        assert!(!is_safe_url("http://127.0.0.1:8080"));
        assert!(!is_safe_url("http://[::1]:3000"));
    }

    #[test]
    fn test_is_safe_url_ipv6_loopback() {
        assert!(!is_safe_url("http://::1"));
        assert!(!is_safe_url("http://[::1]"));
    }

    #[test]
    fn test_is_safe_url_zero_ip() {
        assert!(!is_safe_url("http://0.0.0.0:38001"));
        assert!(!is_safe_url("http://0.0.0.0"));
    }

    #[test]
    fn test_is_safe_url_private_ranges() {
        assert!(!is_safe_url("http://10.0.0.1"));
        assert!(!is_safe_url("http://192.168.1.1"));
        assert!(!is_safe_url("http://172.16.0.1"));
        assert!(!is_safe_url("http://172.31.255.255"));
    }

    #[test]
    fn test_is_safe_url_public_ip() {
        assert!(is_safe_url("http://1.1.1.1"));
        assert!(is_safe_url("http://8.8.8.8"));
        assert!(is_safe_url("https://104.16.132.229"));
    }

    #[test]
    fn test_is_safe_url_172_mid_range_allowed() {
        // 172.15.x.x and 172.32.x.x are NOT private
        assert!(is_safe_url("http://172.15.0.1"));
        assert!(is_safe_url("http://172.32.0.1"));
    }

    #[test]
    fn test_is_safe_url_cloud_metadata_blocked() {
        assert!(!is_safe_url("http://metadata.google.internal"));
        assert!(!is_safe_url("http://metadata.goog"));
    }

    #[test]
    fn test_is_safe_url_docker_internal_blocked() {
        assert!(!is_safe_url("http://host.docker.internal"));
        assert!(!is_safe_url("http://sub.localhost"));
    }

    #[test]
    fn test_is_safe_url_invalid_returns_false() {
        assert!(!is_safe_url("not-a-url"));
        assert!(!is_safe_url(""));
        assert!(!is_safe_url("://missing-protocol.com"));
    }

    #[test]
    fn test_is_safe_url_with_path_and_query() {
        assert!(is_safe_url("https://example.com/api/v1?key=value"));
        assert!(is_safe_url("https://api.example.com/path/to/resource?query=1"));
    }

    #[test]
    fn test_is_safe_url_localhost_subdomain() {
        assert!(!is_safe_url("http://test.localhost:8080"));
        assert!(!is_safe_url("http://anything.localhost"));
    }

    // ─────────────────────────────────────────────────────────
    // SecurityEvent Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_security_event_serialization() {
        let event = SecurityEvent {
            event_type: "Test".to_string(),
            severity: "High".to_string(),
            blocked: true,
            message: "Test event".to_string(),
            source: "Test".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("High"));
    }

    #[test]
    fn test_security_event_clone() {
        let event = SecurityEvent {
            event_type: "Clone Test".to_string(),
            severity: "Low".to_string(),
            blocked: false,
            message: "Cloneable".to_string(),
            source: "Test".to_string(),
        };
        let cloned = event.clone();
        assert_eq!(event.event_type, cloned.event_type);
        assert_eq!(event.severity, cloned.severity);
    }

    #[test]
    fn test_security_event_debug() {
        let event = SecurityEvent {
            event_type: "Debug".to_string(),
            severity: "Medium".to_string(),
            blocked: true,
            message: "Debug test".to_string(),
            source: "Guard".to_string(),
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("SecurityEvent"));
        assert!(debug_str.contains("Debug"));
    }
}
