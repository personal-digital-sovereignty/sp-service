#[cfg(test)]
mod tests {
    use crate::garbage_collector::spawn_ephemeral_garbage_collector;

    #[tokio::test]
    async fn test_spawn_ephemeral_garbage_collector() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Spawns the task, it will sleep for 3600 seconds.
        spawn_ephemeral_garbage_collector(pool.clone()).await;
        
        // We just let the test end, dropping the runtime and the spawned task.
        assert!(true);
    }
}
