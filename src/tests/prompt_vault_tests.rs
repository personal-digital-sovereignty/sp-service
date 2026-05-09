//! ============================================================
//! sp-service — Prompt Vault Tests
//! Tests for compute_hash, VaultEntry parsing, PromptRow
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::prompt_vault::compute_hash;

    // ─────────────────────────────────────────────────────────
    // compute_hash Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_compute_hash_basic() {
        let hash = compute_hash("hello");
        // SHA-256 de "hello" é conhecido
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_compute_hash_empty() {
        let hash = compute_hash("");
        // SHA-256 de string vazia
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let h1 = compute_hash("same input");
        let h2 = compute_hash("same input");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_compute_hash_different_inputs() {
        let h1 = compute_hash("input1");
        let h2 = compute_hash("input2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_compute_hash_length() {
        let hash = compute_hash("any text");
        // SHA-256 = 64 hex chars
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_compute_hash_is_hex() {
        let hash = compute_hash("test");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_hash_unicode() {
        let hash = compute_hash("Português: São João 🎉");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_hash_long_text() {
        let long_text = "a".repeat(10000);
        let hash = compute_hash(&long_text);
        assert_eq!(hash.len(), 64);
    }
}
