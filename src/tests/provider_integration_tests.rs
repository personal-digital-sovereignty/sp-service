//! ============================================================
//! sp-service — Provider Integration Test Suite
//! Covers: Qwen, NVIDIA, OpenRouter settings structures
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::db::init_pool;
    use crate::models::{QwenSettings, NvidiaSettings, OpenRouterSettings};

    // ============================================================
    // Database Persistence Tests
    // ============================================================

    #[tokio::test]
    async fn test_db_pool_initialization() {
        let pool = init_pool().await;

        // Verify pool is functional
        let result = sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await;

        assert!(result.is_ok(), "Database pool should initialize successfully");
        println!("✅ DB pool initialization validated");
    }

    #[tokio::test]
    async fn test_db_settings_table_exists() {
        let pool = init_pool().await;

        // Verify global_settings table exists
        let result = sqlx::query("SELECT COUNT(*) FROM global_settings")
            .fetch_one(&pool)
            .await;

        assert!(result.is_ok(), "global_settings table should exist");
        println!("✅ DB global_settings table validated");
    }

    // ============================================================
    // Provider Settings Structure Tests
    // ============================================================

    #[test]
    fn test_qwen_settings_structure() {
        let settings = QwenSettings {
            enabled: true,
            api_key: "test-key".to_string(),
            default_model: "qwen-max".to_string(),
            base_url: "https://dashscope.aliyuncs.com/api/v1".to_string(),
        };

        assert!(settings.enabled);
        assert_eq!(settings.api_key, "test-key");
        assert_eq!(settings.default_model, "qwen-max");

        println!("✅ Qwen settings structure validated");
    }

    #[test]
    fn test_nvidia_settings_structure() {
        let settings = NvidiaSettings {
            enabled: true,
            api_key: "test-key".to_string(),
            default_model: "nvidia/llama-3".to_string(),
        };

        assert!(settings.enabled);
        assert_eq!(settings.api_key, "test-key");

        println!("✅ NVIDIA settings structure validated");
    }

    #[test]
    fn test_openrouter_settings_structure() {
        let settings = OpenRouterSettings {
            api_key: "test-key".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            site_url: "https://example.com".to_string(),
            site_name: "sp-service".to_string(),
            enabled: true,
            default_model: "meta-llama/llama-3-70b".to_string(),
            fallback_enabled: true,
        };

        assert!(settings.enabled);
        assert!(settings.fallback_enabled);
        assert_eq!(settings.default_model, "meta-llama/llama-3-70b");

        println!("✅ OpenRouter settings structure validated");
    }

    // ============================================================
    // Multi-Provider Configuration Test
    // ============================================================

    #[test]
    fn test_multi_provider_enabled() {
        let qwen = QwenSettings {
            enabled: true,
            api_key: "qwen-key".to_string(),
            default_model: "qwen-max".to_string(),
            base_url: "https://dashscope.aliyuncs.com/api/v1".to_string(),
        };

        let nvidia = NvidiaSettings {
            enabled: true,
            api_key: "nvidia-key".to_string(),
            default_model: "nvidia/llama-3".to_string(),
        };

        let openrouter = OpenRouterSettings {
            api_key: "or-key".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            site_url: "https://example.com".to_string(),
            site_name: "sp-service".to_string(),
            enabled: true,
            default_model: "meta-llama/llama-3-70b".to_string(),
            fallback_enabled: true,
        };

        assert!(qwen.enabled);
        assert!(nvidia.enabled);
        assert!(openrouter.enabled);

        println!("✅ Multi-provider configuration structure validated");
    }
}
