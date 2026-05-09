//! ============================================================
//! Sovereign Pair — Oracle Worker Tests
//! Covers: config parsing, key resolution, routing logic, shell_escape
//! ============================================================

#[cfg(test)]
mod oracle_worker_config {
    use crate::oracle_worker::{OracleNodeConfig, WorkerSite, shell_escape, WorkerResult};

    fn make_config(enabled: bool, ip: &str) -> OracleNodeConfig {
        OracleNodeConfig {
            ip: ip.to_string(),
            user: "ubuntu".to_string(),
            key_path: "~/.ssh/id_ed25519".to_string(),
            ollama_tunnel_port: 41434,
            enabled,
            cold_storage_enabled: false,
            workers_dir: "~/sovereign-workers".to_string(),
            venv_path: "~/sovereign-venv/bin/python".to_string(),
        }
    }

    /// Node disabled → not ready regardless of IP
    #[test]
    fn test_disabled_node_not_ready() {
        let config = make_config(false, "129.80.244.152");
        assert!(!config.is_ready(), "Disabled node should not be ready");
    }

    /// Node enabled but no IP → not ready
    #[test]
    fn test_enabled_no_ip_not_ready() {
        let config = make_config(true, "");
        assert!(!config.is_ready(), "Node with empty IP should not be ready");
    }

    /// Node enabled with valid IP → ready
    #[test]
    fn test_enabled_with_ip_ready() {
        let config = make_config(true, "129.80.244.152");
        assert!(config.is_ready(), "Properly configured node should be ready");
    }

    /// SSH target format
    #[test]
    fn test_ssh_target_format() {
        let config = make_config(true, "129.80.244.152");
        assert_eq!(config.ssh_target(), "ubuntu@129.80.244.152");
    }

    /// Tilde expansion in key path
    #[test]
    fn test_key_path_tilde_expansion() {
        let config = make_config(true, "129.80.244.152");
        let resolved = config.resolve_key_path();
        // HOME env should be set in test environment
        if let Ok(home) = std::env::var("HOME") {
            assert!(
                resolved.starts_with(&home),
                "Key path should expand ~ to HOME: got {}",
                resolved
            );
        }
        // At minimum, tilde should not remain as-is if HOME is defined
        assert!(!resolved.is_empty(), "Resolved key path should not be empty");
    }

    /// Key path without tilde is returned as-is
    #[test]
    fn test_key_path_no_tilde_unchanged() {
        let mut config = make_config(true, "129.80.244.152");
        config.key_path = "/home/ubuntu/.ssh/id_ed25519".to_string();
        let resolved = config.resolve_key_path();
        assert_eq!(resolved, "/home/ubuntu/.ssh/id_ed25519");
    }

    /// Cold storage disabled by default
    #[test]
    fn test_cold_storage_disabled_by_default() {
        let config = OracleNodeConfig::default();
        assert!(!config.cold_storage_enabled, "Cold storage should be disabled by default");
    }

    /// WorkerSite display
    #[test]
    fn test_worker_site_display() {
        assert_eq!(format!("{}", WorkerSite::Oracle), "Oracle Cloud");
        assert_eq!(format!("{}", WorkerSite::Local), "Local");
    }

    /// JSON deserialization roundtrip
    #[test]
    fn test_config_json_roundtrip() {
        let config = make_config(true, "129.80.244.152");
        let json = serde_json::to_string(&config).unwrap();
        let restored: OracleNodeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.ip, "129.80.244.152");
        assert!(restored.enabled);
        assert_eq!(restored.ollama_tunnel_port, 41434);
        assert!(!restored.cold_storage_enabled);
    }

    /// Default config has all safe defaults
    #[test]
    fn test_default_config_safe() {
        let config = OracleNodeConfig::default();
        assert!(!config.enabled, "Default should be disabled");
        assert!(config.ip.is_empty(), "Default IP should be empty");
        assert!(!config.is_ready(), "Default config should not be ready");
        assert_eq!(config.ollama_tunnel_port, 41434);
        assert_eq!(config.user, "ubuntu");
    }

    // ─────────────────────────────────────────────────────────
    // shell_escape Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[test]
    fn test_shell_escape_with_single_quotes() {
        assert_eq!(shell_escape("it's a test"), "'it'\\''s a test'");
    }

    #[test]
    fn test_shell_escape_with_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_shell_escape_injection_attempt() {
        assert_eq!(shell_escape("file; rm -rf /"), "'file; rm -rf /'");
    }

    #[test]
    fn test_shell_escape_empty() {
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn test_shell_escape_with_dollar() {
        assert_eq!(shell_escape("$HOME"), "'$HOME'");
    }

    // ─────────────────────────────────────────────────────────
    // WorkerResult Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_worker_result_success_oracle() {
        let result = WorkerResult {
            stdout: "output".to_string(),
            success: true,
            execution_site: WorkerSite::Oracle,
        };
        assert!(result.success);
        assert_eq!(result.execution_site, WorkerSite::Oracle);
    }

    #[test]
    fn test_worker_result_failure_local() {
        let result = WorkerResult {
            stdout: String::new(),
            success: false,
            execution_site: WorkerSite::Local,
        };
        assert!(!result.success);
        assert_eq!(result.execution_site, WorkerSite::Local);
    }
}
