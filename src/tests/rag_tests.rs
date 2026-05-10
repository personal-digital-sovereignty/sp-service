//! ============================================================
//! sp-service — RAG Tests
//! Tests for vault initialization
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::rag::init_vault;
    use std::env;

    #[test]
    fn test_init_vault_default_path() {
        // Should use HOME/Vault or fallback to current dir
        let vault = init_vault();
        // The vault path should either contain "Vault" or be a valid path
        let path_str = vault.to_string_lossy();
        assert!(!path_str.is_empty() || vault.exists(),
            "init_vault returned empty path");
    }

    #[test]
    fn test_init_vault_custom_env() {
        env::set_var("SOVEREIGN_VAULT_PATH", "/tmp/test-vault");
        let vault = init_vault();
        assert!(vault.to_string_lossy().contains("test-vault"));
        env::remove_var("SOVEREIGN_VAULT_PATH");
    }

    #[test]
    fn test_init_vault_creates_dir() {
        let temp = std::env::temp_dir();
        let test_path = temp.join("sp-rag-test-vault");
        env::set_var("SOVEREIGN_VAULT_PATH", test_path.to_str().unwrap());
        let vault = init_vault();
        assert!(vault.exists());
        let _ = std::fs::remove_dir_all(&test_path);
        env::remove_var("SOVEREIGN_VAULT_PATH");
    }
}
