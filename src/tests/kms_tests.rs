//! ============================================================
//! sp-service — KMS Tests
//! Tests for vault secret encryption/decryption
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::kms::{encrypt_vault_secret, decrypt_vault_secret};

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let secret = "my-super-secret-api-key-12345";
        let encrypted = encrypt_vault_secret(secret).expect("encryption should succeed");
        let decrypted = decrypt_vault_secret(&encrypted).expect("decryption should succeed");
        assert_eq!(decrypted, secret);
    }

    #[test]
    fn test_encrypt_returns_base64() {
        let encrypted = encrypt_vault_secret("test").expect("encryption should succeed");
        // Should be valid base64 (no special chars except +/=)
        assert!(encrypted.chars().all(|c| c.is_alphanumeric() || "+/=".contains(c)));
    }

    #[test]
    fn test_decrypt_invalid_returns_none() {
        assert!(decrypt_vault_secret("not-valid-base64-data").is_none());
    }

    #[test]
    fn test_encrypt_empty_string() {
        // KMS may not support empty strings - test graceful handling
        let result = encrypt_vault_secret("");
        if let Some(encrypted) = result {
            let decrypted = decrypt_vault_secret(&encrypted).expect("decryption should succeed");
            assert_eq!(decrypted, "");
        }
        // If encryption returns None for empty string, that's also acceptable
    }

    #[test]
    fn test_encrypt_unicode() {
        let secret = "Chave secreta: 🗝️ São João 🎉";
        let encrypted = encrypt_vault_secret(secret).expect("encryption should succeed");
        let decrypted = decrypt_vault_secret(&encrypted).expect("decryption should succeed");
        assert_eq!(decrypted, secret);
    }

    #[test]
    fn test_encrypt_produces_different_output() {
        // Same input should produce different ciphertext each time (random IV/nonce)
        let e1 = encrypt_vault_secret("same").unwrap();
        let e2 = encrypt_vault_secret("same").unwrap();
        // AES-GCM with random nonce should produce different ciphertext
        // (unless extremely unlikely collision)
        assert!(e1 != e2, "Encryption should use random nonce");
    }

    #[test]
    fn test_decrypt_empty_string_returns_none() {
        assert!(decrypt_vault_secret("").is_none());
    }
}
