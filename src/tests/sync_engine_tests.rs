#[cfg(test)]
mod tests {
    use crate::sync_engine::{IngestionJob, SyncEngine};

    #[test]
    fn test_ingestion_job_serialization() {
        let job = IngestionJob {
            id: "test-123".to_string(),
            filename: "doc.txt".to_string(),
            status: "queued".to_string(),
            current_step: 0,
            progress_ms: 100,
        };

        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains(r#""id":"test-123""#));
        assert!(json.contains(r#""filename":"doc.txt""#));
        assert!(json.contains(r#""status":"queued""#));
        assert!(json.contains(r#""currentStep":0"#)); // Using serde(rename = "currentStep")
        assert!(json.contains(r#""progress_ms":100"#));
    }

    #[tokio::test]
    async fn test_sync_engine_new() {
        // We use a dummy sqlite memory pool for testing
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let engine = SyncEngine::new(pool);
        
        // Ensure broadcast channel is created
        assert_eq!(engine.tx.receiver_count(), 0);
    }
}
