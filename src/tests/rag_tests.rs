#[cfg(test)]
mod tests {
    use crate::rag::{init_vault, parse_vault_documents, build_rag_context_message};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::env;

    #[test]
    fn test_init_vault() {
        // Temporarily set the env var to a temp directory
        let temp_dir = tempfile::tempdir().unwrap();
        unsafe {
            env::set_var("SOVEREIGN_VAULT_PATH", temp_dir.path().to_str().unwrap());
        }
        
        let path = init_vault();
        assert_eq!(path, temp_dir.path());
        assert!(path.exists());
        
        unsafe {
            env::remove_var("SOVEREIGN_VAULT_PATH");
        }
    }

    #[tokio::test]
    async fn test_parse_vault_documents_empty_db() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Needs the table to exist to avoid error, or if it errors it just returns an empty string
        // The implementation uses `.fetch_all()` which fails if the table doesn't exist,
        // but it's wrapped in `if let Ok(rows) = ...`, so it will gracefully return empty string.
        let content = parse_vault_documents("workspace-test", &pool).await;
        assert_eq!(content, "");
    }

    #[tokio::test]
    async fn test_build_rag_context_message_empty() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let context = build_rag_context_message("workspace-test", &pool).await;
        assert!(context.is_none());
    }
}
