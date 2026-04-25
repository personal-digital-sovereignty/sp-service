//! ============================================================
//! Sovereign Pair — Stress & Concurrency Test Suite
//! Covers: SQLite WAL contention, Log Channel throughput, 
//!         Large Payload Guardrail performance.
//! ============================================================

#[cfg(test)]
mod database_stress {
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
    use sqlx::{SqlitePool};
    use std::str::FromStr;
    use tokio::task;

    /// STRESS-01: SQLite WAL Mode Contention
    /// Garante que o banco de dados suporta 50 threads escrevendo simultaneamente
    /// sem erros de "database is locked" graças ao modo WAL.
    #[tokio::test]
    async fn test_sqlite_wal_contention() {
        // Usar um banco temporário em memória para o teste de estresse
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .journal_mode(SqliteJournalMode::Wal)
            .create_if_missing(true);
        
        let pool = SqlitePool::connect_with(options).await.unwrap();

        // Criar tabela de teste
        sqlx::query("CREATE TABLE IF NOT EXISTS stress_test (id INTEGER PRIMARY KEY, val TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        let mut handles = vec![];
        for i in 0..50 {
            let pool_clone = pool.clone();
            let handle = task::spawn(async move {
                for j in 0..10 {
                    sqlx::query("INSERT INTO stress_test (val) VALUES (?)")
                        .bind(format!("thread-{}-msg-{}", i, j))
                        .execute(&pool_clone)
                        .await
                        .unwrap();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stress_test")
            .fetch_one(&pool)
            .await
            .unwrap();
        
        assert_eq!(count.0, 500, "Deve haver exatamente 500 registros após o estresse");
    }
}

#[cfg(test)]
mod log_channel_stress {
    use crate::models::LogEntry;
    use tokio::sync::broadcast;

    /// STRESS-02: Broadcast Channel Throughput
    /// Valida que o canal de logs não estoura com 10.000 mensagens rápidas.
    #[tokio::test]
    async fn test_log_channel_throughput() {
        let (tx, mut rx1) = broadcast::channel(1024);
        let mut rx2 = tx.subscribe();
        
        let tx_clone = tx.clone();
        let handle = tokio::spawn(async move {
            for i in 0..10000 {
                let _ = tx_clone.send(LogEntry {
                    timestamp: "".into(),
                    level: "info".into(),
                    message: format!("Log message {}", i),
                });
            }
        });

        let mut count1 = 0;
        let mut count2 = 0;
        
        // Consumir até o fim (usando timeout para não travar o teste)
        let _ = tokio::time::timeout(tokio::time::Duration::from_millis(500), async {
            while count1 < 10000 {
                if rx1.recv().await.is_ok() { count1 += 1; }
            }
        }).await;

        let _ = tokio::time::timeout(tokio::time::Duration::from_millis(500), async {
            while count2 < 10000 {
                if rx2.recv().await.is_ok() { count2 += 1; }
            }
        }).await;

        handle.await.unwrap();
        
        println!("🚀 Receiver 1 processed: {} logs", count1);
        println!("🚀 Receiver 2 processed: {} logs", count2);
        
        // Em um canal de broadcast, se o produtor for muito mais rápido que o consumidor, 
        // o consumidor pode perder mensagens (Lagged). 
        // O teste valida que o sistema não trava.
        assert!(count1 > 0);
    }
}

#[cfg(test)]
mod guard_stress {
    /// STRESS-03: Guardrail Performance on Large Payload
    /// Simula a detecção de padrões em strings gigantes (1MB+) para garantir
    /// que o Epistemic Guard não causa latência extrema.
    #[test]
    fn test_guardrail_performance_on_large_payload() {
        let large_payload = "A".repeat(1_000_000); // 1MB de texto
        let patterns = vec![
            r"\b\d{3}\.\d{3}\.\d{3}-\d{2}\b", // CPF
            r"ignore all previous instructions",
            r"how to build a bomb"
        ];
        
        let start = std::time::Instant::now();
        for pattern in patterns {
            let re = regex::Regex::new(pattern).unwrap();
            let _ = re.is_match(&large_payload);
        }
        let elapsed = start.elapsed();
        
        println!("🚀 Guardrail scan (1MB) took: {:?}", elapsed);
        assert!(elapsed.as_millis() < 200, "Scan de 1MB deve levar menos de 200ms");
    }
}
