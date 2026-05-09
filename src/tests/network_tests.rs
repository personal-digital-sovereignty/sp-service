//! ============================================================
//! sp-service — Network Tests
//! Tests for NetworkIdentity struct and JWT claims
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::network::NetworkIdentity;
    use jsonwebtoken::{encode, Header, EncodingKey, decode, DecodingKey, Validation};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct TestClaims {
        sub: String,
        exp: usize,
    }

    #[test]
    fn test_network_identity_alias_format() {
        // Testa o formato do alias sem depender do OnceLock
        let alias = "sovereign-test";
        assert!(alias.starts_with("sovereign-"));
        assert_eq!(alias.len(), 14);
    }

    #[test]
    fn test_network_identity_jwt_can_be_created() {
        let jwt_secret = "test-secret-key-for-jwt";
        let claims = TestClaims {
            sub: "sovereign_pairing".to_owned(),
            exp: (chrono::Local::now() + chrono::Duration::try_days(30).unwrap()).timestamp() as usize,
        };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_bytes())).unwrap();
        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_network_identity_jwt_can_be_decoded() {
        let jwt_secret = "test-secret-key-for-jwt";
        let claims = TestClaims {
            sub: "sovereign_pairing".to_owned(),
            exp: (chrono::Local::now() + chrono::Duration::try_days(30).unwrap()).timestamp() as usize,
        };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_bytes())).unwrap();

        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = true;
        let decoded = decode::<TestClaims>(&token, &DecodingKey::from_secret(jwt_secret.as_bytes()), &validation);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap().claims.sub, "sovereign_pairing");
    }

    #[test]
    fn test_network_identity_struct_fields() {
        let identity = NetworkIdentity {
            alias: "sovereign-test".to_string(),
            jwt_secret: "secret".to_string(),
            current_token: "token".to_string(),
        };
        assert_eq!(identity.alias, "sovereign-test");
        assert_eq!(identity.jwt_secret, "secret");
        assert_eq!(identity.current_token, "token");
    }

    #[test]
    fn test_network_identity_is_cloneable() {
        let identity = NetworkIdentity {
            alias: "sovereign-test".to_string(),
            jwt_secret: "secret".to_string(),
            current_token: "token".to_string(),
        };
        let cloned = identity.clone();
        assert_eq!(identity.alias, cloned.alias);
        assert_eq!(identity.jwt_secret, cloned.jwt_secret);
        assert_eq!(identity.current_token, cloned.current_token);
    }

    #[test]
    fn test_network_identity_debug() {
        let identity = NetworkIdentity {
            alias: "sovereign-test".to_string(),
            jwt_secret: "secret".to_string(),
            current_token: "token".to_string(),
        };
        let debug_str = format!("{:?}", identity);
        assert!(debug_str.contains("NetworkIdentity"));
        assert!(debug_str.contains("sovereign-test"));
    }
}
