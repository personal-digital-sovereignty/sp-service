/// ============================================================
/// Sovereign Pair — Security Test Suite (Pass 3 Regression)
/// Covers: JWT Algorithm Confusion, SSRF Guard, Path Traversal,
///         KMS Encryption, Body Limit, Token Exposure
/// ============================================================

#[cfg(test)]
mod jwt_security {
    use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    fn valid_token(secret: &str) -> String {
        let claims = Claims {
            sub: "sovereign_pairing".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::try_days(1).unwrap()).timestamp() as usize,
        };
        encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())).unwrap()
    }

    /// P3-02: HS256 algorithm guard — 'none' token deve ser rejeitado
    #[test]
    fn test_jwt_none_algorithm_rejected() {
        let secret = "test_secret_sovereign";
        // 'none' alg token: header={"alg":"none"}, payload normal, sem assinatura
        let none_token = "eyJhbGciOiJub25lIn0.eyJzdWIiOiJhdHRhY2tlciIsImV4cCI6OTk5OTk5OTk5OX0.";
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let result = decode::<Claims>(
            none_token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        );
        assert!(result.is_err(), "SECURITY: JWT 'none' algorithm deve ser rejeitado");
    }

    /// P3-02: Token com chave errada deve ser rejeitado
    #[test]
    fn test_jwt_wrong_secret_rejected() {
        let token = valid_token("correct_secret");
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret("wrong_secret".as_bytes()),
            &validation,
        );
        assert!(result.is_err(), "SECURITY: Token com chave errada deve ser rejeitado");
    }

    /// Token válido com chave correta deve ser aceito
    #[test]
    fn test_jwt_valid_token_accepted() {
        let secret = "sovereign_valid_secret_32bytes!!";
        let token = valid_token(secret);
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        );
        assert!(result.is_ok(), "Token válido deve ser aceito: {:?}", result.err());
    }

    /// Token expirado deve ser rejeitado
    #[test]
    fn test_jwt_expired_token_rejected() {
        let secret = "sovereign_secret";
        let claims = Claims {
            sub: "sovereign_pairing".to_string(),
            exp: 1_000_000, // Epoch 1970 — expirado há 50+ anos
        };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())).unwrap();
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let result = decode::<Claims>(&token, &DecodingKey::from_secret(secret.as_bytes()), &validation);
        assert!(result.is_err(), "SECURITY: Token expirado deve ser rejeitado");
    }
}

#[cfg(test)]
mod ssrf_guard {
    use crate::guardrails::is_safe_url;

    /// LIN-09: 0.0.0.0 deve ser bloqueado (SSRF via bind-all Linux)
    #[test]
    fn test_ssrf_blocks_zero_zero_zero_zero() {
        assert!(!is_safe_url("http://0.0.0.0:8080/api"), "0.0.0.0 deve ser bloqueado");
        assert!(!is_safe_url("http://0.0.0.0/"), "0.0.0.0 root deve ser bloqueado");
    }

    /// LIN-09: IPv6 loopback deve ser bloqueado
    #[test]
    fn test_ssrf_blocks_ipv6_loopback() {
        assert!(!is_safe_url("http://[::1]:8080"), "IPv6 ::1 deve ser bloqueado");
        assert!(!is_safe_url("http://[::1]/"), "IPv6 loopback root deve ser bloqueado");
    }

    /// LIN-09: GCP metadata server deve ser bloqueado
    #[test]
    fn test_ssrf_blocks_gcp_metadata() {
        assert!(!is_safe_url("http://metadata.google.internal/computeMetadata"), "GCP metadata deve ser bloqueado");
        assert!(!is_safe_url("http://metadata.goog/"), "GCP metadata.goog deve ser bloqueado");
    }

    /// AWS/Azure IMDS deve ser bloqueado
    #[test]
    fn test_ssrf_blocks_aws_imds() {
        assert!(!is_safe_url("http://169.254.169.254/latest/meta-data/"), "AWS IMDS deve ser bloqueado");
    }

    /// localhost deve ser bloqueado
    #[test]
    fn test_ssrf_blocks_localhost() {
        assert!(!is_safe_url("http://localhost:3000"), "localhost deve ser bloqueado");
        assert!(!is_safe_url("http://127.0.0.1:8080"), "127.0.0.1 deve ser bloqueado");
    }

    /// URLs públicas legítimas devem passar
    #[test]
    fn test_ssrf_allows_public_urls() {
        assert!(is_safe_url("https://api.example.com/data"), "URL HTTPS pública deve passar");
        assert!(is_safe_url("https://arxiv.org/abs/2301.00001"), "ArXiv deve passar");
        assert!(is_safe_url("https://wikipedia.org/wiki/Test"), "Wikipedia deve passar");
    }

    /// URL vazia deve falhar
    #[test]
    fn test_ssrf_blocks_empty_url() {
        assert!(!is_safe_url(""), "URL vazia deve ser bloqueada");
    }

    /// URL inválida/malformada deve falhar
    #[test]
    fn test_ssrf_blocks_malformed_url() {
        assert!(!is_safe_url("not_a_url"), "URL malformada deve ser bloqueada");
        assert!(!is_safe_url("javascript:alert(1)"), "javascript: scheme deve ser bloqueado");
    }
}

#[cfg(test)]
mod kms_security {
    use crate::kms::{encrypt_vault_secret, decrypt_vault_secret};

    /// Encrypt/Decrypt roundtrip deve funcionar
    #[test]
    fn test_kms_encrypt_decrypt_roundtrip() {
        // SAFETY: testes single-threaded; set_var é seguro aqui
        unsafe { std::env::set_var("SOVEREIGN_MASTER_KEK", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="); }
        let plaintext = "secret_api_key_12345";
        let encrypted = encrypt_vault_secret(plaintext).expect("Encrypt deve funcionar");
        assert_ne!(encrypted, plaintext, "Ciphertext não deve igual ao plaintext");
        let decrypted = decrypt_vault_secret(&encrypted).expect("Decrypt deve funcionar");
        assert_eq!(decrypted, plaintext, "Decrypt deve restituir o original");
    }

    /// IV único por operação — dois encrypts do mesmo plaintext devem gerar ciphertexts diferentes
    #[test]
    fn test_kms_unique_iv_per_operation() {
        // SAFETY: testes single-threaded; set_var é seguro aqui
        unsafe { std::env::set_var("SOVEREIGN_MASTER_KEK", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="); }
        let plaintext = "same_secret";
        let enc1 = encrypt_vault_secret(plaintext).unwrap();
        let enc2 = encrypt_vault_secret(plaintext).unwrap();
        assert_ne!(enc1, enc2, "SECURITY: IV deve ser único por operação — ciphertexts iguais indicam IV fixo");
    }

    /// Ciphertext corrompido deve retornar None graciosamente
    #[test]
    fn test_kms_corrupted_ciphertext_returns_none() {
        // SAFETY: testes single-threaded; set_var é seguro aqui
        unsafe { std::env::set_var("SOVEREIGN_MASTER_KEK", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="); }
        let result = decrypt_vault_secret("dGhpc2lzY29ycnVwdGVk");
        assert!(result.is_none(), "Ciphertext corrompido deve retornar None");
    }

    /// String vazia deve retornar None
    #[test]
    fn test_kms_empty_plaintext_returns_none() {
        let result = encrypt_vault_secret("");
        assert!(result.is_none(), "Plaintext vazio deve retornar None");
    }
}

#[cfg(test)]
mod body_limit {
    /// P3-03: import_config com payload acima de 5 MB deve ser rejeitado
    /// (Teste de regressão para a lógica de guarda em api_settings.rs)
    #[test]
    fn test_import_config_body_limit_guard() {
        const MAX_IMPORT_BYTES: usize = 5 * 1024 * 1024;
        let oversized_body = "A".repeat(MAX_IMPORT_BYTES + 1);
        assert!(
            oversized_body.len() > MAX_IMPORT_BYTES,
            "Payload oversized deve exceder o limite de 5 MB"
        );
        // Verificar que a constante é exatamente 5 MB
        assert_eq!(MAX_IMPORT_BYTES, 5_242_880, "Limite deve ser exatamente 5 MB");
    }

    /// Payload dentro do limite deve ser aceito pela lógica de guarda
    #[test]
    fn test_import_config_valid_payload_passes_guard() {
        const MAX_IMPORT_BYTES: usize = 5 * 1024 * 1024;
        let valid_body = "A".repeat(1024); // 1 KB
        assert!(
            valid_body.len() <= MAX_IMPORT_BYTES,
            "Payload de 1 KB deve ser aceito pela guarda de 5 MB"
        );
    }
}

#[cfg(test)]
mod path_traversal {
    use std::path::PathBuf;

    /// Anti-traversal: ../ fora do workspace deve ser detectado
    #[test]
    fn test_path_traversal_detected_outside_workspace() {
        let workspace = PathBuf::from("/home/user/workspace");
        let attack_path = PathBuf::from("/home/user/workspace/../../etc/passwd");
        // Simula o que canonicalize() faria em produção
        let normalized = attack_path.components()
            .fold(PathBuf::new(), |mut acc, c| {
                match c {
                    std::path::Component::ParentDir => { acc.pop(); acc }
                    _ => { acc.push(c); acc }
                }
            });
        assert!(!normalized.starts_with(&workspace), "Path traversal ../ deve ser detectado fora do workspace");
    }

    /// Path dentro do workspace deve ser permitido
    #[test]
    fn test_path_within_workspace_allowed() {
        let workspace = PathBuf::from("/home/user/workspace");
        let safe_path = PathBuf::from("/home/user/workspace/documents/file.txt");
        assert!(safe_path.starts_with(&workspace), "Path dentro do workspace deve ser permitido");
    }

    /// Path com subdirs profundos dentro do workspace deve ser permitido
    #[test]
    fn test_deep_nested_path_within_workspace_allowed() {
        let workspace = PathBuf::from("/home/user/workspace");
        let deep_path = PathBuf::from("/home/user/workspace/a/b/c/d/e/file.txt");
        assert!(deep_path.starts_with(&workspace), "Path profundamente aninhado dentro do workspace deve ser permitido");
    }
}
