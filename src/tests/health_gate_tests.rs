//! ============================================================
//! Sovereign Pair — Resilience Shield: Health Gate Tests
//! Covers: JSON parsing, summary building, entry validation
//! ============================================================

#[cfg(test)]
mod health_gate_parsing {
    use crate::health_gate::{ApiHealthEntry, build_summary};

    fn mock_entries() -> Vec<ApiHealthEntry> {
        serde_json::from_str(r#"[
            {"name": "IPCA", "type": "BCB_SGS", "status": "HEALTHY", "critical": true, "description": "Inflação", "latency_ms": 250, "records": 90},
            {"name": "SELIC", "type": "BCB_SGS", "status": "HEALTHY", "critical": false, "description": "Juros", "latency_ms": 180},
            {"name": "BRENT", "type": "Yahoo_Finance", "status": "DEAD", "critical": true, "description": "Petróleo", "latency_ms": 5000, "error": "timeout"},
            {"name": "PETROBRAS", "type": "Yahoo_Finance", "status": "SKIP", "critical": false, "description": "Ações", "latency_ms": 0, "error": "yfinance not installed"}
        ]"#).expect("test assertion failed — review test setup")
    }

    /// Verify JSON deserialization matches Python health_check_apis.py output
    #[test]
    fn test_health_entry_deserialization() {
        let entries = mock_entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].name, "IPCA");
        assert_eq!(entries[0].api_type, "BCB_SGS");
        assert_eq!(entries[0].status, "HEALTHY");
        assert!(entries[0].critical);
        assert_eq!(entries[0].latency_ms, 250);
    }

    /// Verify summary aggregation logic
    #[test]
    fn test_build_summary_counts() {
        let entries = mock_entries();
        let summary = build_summary(entries);
        
        assert_eq!(summary.total_apis, 4);
        assert_eq!(summary.healthy, 3, "HEALTHY + SKIP = healthy");
        assert_eq!(summary.degraded, 1, "Only DEAD counts as degraded");
        assert_eq!(summary.critical_failures, vec!["BRENT"]);
    }

    /// Verify all-healthy scenario produces no critical failures
    #[test]
    fn test_all_healthy_no_critical() {
        let entries: Vec<ApiHealthEntry> = serde_json::from_str(r#"[
            {"name": "A", "type": "BCB", "status": "HEALTHY", "critical": true, "description": "", "latency_ms": 100},
            {"name": "B", "type": "BCB", "status": "HEALTHY", "critical": false, "description": "", "latency_ms": 50}
        ]"#).expect("test assertion failed — review test setup");
        
        let summary = build_summary(entries);
        assert_eq!(summary.degraded, 0);
        assert!(summary.critical_failures.is_empty());
    }

    /// Verify empty results don't panic
    #[test]
    fn test_empty_results() {
        let summary = build_summary(vec![]);
        assert_eq!(summary.total_apis, 0);
        assert_eq!(summary.healthy, 0);
        assert!(summary.critical_failures.is_empty());
    }

    /// Verify non-critical failures don't count as critical
    #[test]
    fn test_non_critical_failure_not_flagged() {
        let entries: Vec<ApiHealthEntry> = serde_json::from_str(r#"[
            {"name": "SELIC", "type": "BCB", "status": "DEAD", "critical": false, "description": "", "latency_ms": 0, "error": "timeout"}
        ]"#).expect("test assertion failed — review test setup");
        
        let summary = build_summary(entries);
        assert_eq!(summary.degraded, 1);
        assert!(summary.critical_failures.is_empty(), "Non-critical failures should not be flagged");
    }

    /// Verify optional fields handle missing values
    #[test]
    fn test_optional_fields_default() {
        let entries: Vec<ApiHealthEntry> = serde_json::from_str(r#"[
            {"name": "TEST", "type": "BCB", "status": "HEALTHY"}
        ]"#).expect("test assertion failed — review test setup");
        
        assert_eq!(entries[0].latency_ms, 0);
        assert!(!entries[0].critical);
        assert!(entries[0].error.is_none());
        assert!(entries[0].records.is_none());
    }
}
