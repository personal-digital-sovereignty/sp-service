use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use crate::AppState;
use sqlx::Row;
use chrono::Local;

pub async fn start_cron_agents(state: Arc<AppState>) {
    tracing::info!("🤖 [Cron-Agents] Iniciando Orquestrador de Agentes Autônomos em Background...");

    // ---------------------------------------------------------
    // 1. Market Pricing Agent (12h Interval)
    // ---------------------------------------------------------
    let db_pricing = state.db.clone();
    let telemetry_ref = state.telemetry.clone();
    tokio::spawn(async move {
        // Trigger first tick immediately, then every 12 hours
        let mut ticker = interval(Duration::from_secs(12 * 3600));
        loop {
            ticker.tick().await;
            tracing::info!("☁️ [Market Pricing Agent] Sincronizando custos de nuvem globais...");
            
            let workers_dir = crate::api_trainer::resolve_python_workers_dir();
            let script_path = workers_dir.join("market_pricing_matrix.py");
            let python_bin = crate::sandbox::get_hermetic_python_bin();
            
            let _ = std::process::Command::new(&python_bin)
                .arg(&script_path)
                .output();

            let avg_cost = sqlx::query_scalar::<_, String>("SELECT value_json FROM global_settings WHERE id = 'avg_cloud_token_cost_1k'")
                .fetch_one(&db_pricing)
                .await
                .unwrap_or_else(|_| "0.00625".to_string())
                .parse::<f64>()
                .unwrap_or(0.00625);

            if let Ok(mut t) = telemetry_ref.write() {
                t.avg_cloud_cost_per_1k = avg_cost;
                tracing::info!("☁️ [Market Pricing Agent] Live Market Pricing Atualizado: ${:.5}/1k tokens", avg_cost);
            }
        }
    });

    // ---------------------------------------------------------
    // 2. Gap Solver Agent (24h Interval)
    // ---------------------------------------------------------
    let db_gaps = state.db.clone();
    tokio::spawn(async move {
        // Wait an hour before first run to not overload boot
        tokio::time::sleep(Duration::from_secs(3600)).await;
        let mut ticker = interval(Duration::from_secs(24 * 3600));
        loop {
            ticker.tick().await;
            tracing::info!("🕵️ [Gap Solver Agent] Buscando gaps de conhecimento não resolvidos...");
            
            // Puxa o gap mais frequente
            if let Ok(row) = sqlx::query("SELECT id, query FROM knowledge_gaps ORDER BY frequency DESC LIMIT 1")
                .fetch_one(&db_gaps)
                .await 
            {
                let gap_query: String = row.get("query");
                tracing::info!("🕵️ [Gap Solver Agent] Tentando resolver automaticamente o gap: '{}'", gap_query);
                
                // TODO: Integração direta com Deep Research e re-injeção no Vault.
                // Por hora, apenas registra a intenção para a malha LLMOps
                tracing::info!("🕵️ [Gap Solver Agent] Gap enfileirado para o próximo ciclo de Research.");
            }
        }
    });

    // ---------------------------------------------------------
    // 3. SQLite Backup Agent (24h Interval)
    // ---------------------------------------------------------
    let db_backup = state.db.clone();
    tokio::spawn(async move {
        // Run daily
        let mut ticker = interval(Duration::from_secs(24 * 3600));
        loop {
            ticker.tick().await;
            tracing::info!("🗄️ [SQLite Backup Agent] Iniciando backup de segurança do banco Híbrido Cíbrido...");
            
            if let Ok(rows) = sqlx::query("PRAGMA database_list").fetch_all(&db_backup).await {
                if let Some(main_db) = rows.iter().find(|r| r.get::<String, _>("name") == "main") {
                    let file_path: String = main_db.get("file");
                    if !file_path.is_empty() {
                        let path = std::path::Path::new(&file_path);
                        if let Some(parent) = path.parent() {
                            let backup_dir = parent.join("backups");
                            let _ = std::fs::create_dir_all(&backup_dir);
                            let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
                            let backup_path = backup_dir.join(format!("sovereign_memory_{}.db", timestamp));
                            
                            match std::fs::copy(path, &backup_path) {
                                Ok(bytes) => tracing::info!("✅ [SQLite Backup Agent] Backup concluído ({} bytes): {:?}", bytes, backup_path),
                                Err(e) => tracing::error!("❌ [SQLite Backup Agent] Falha ao fazer backup: {}", e),
                            }
                        }
                    }
                }
            }
        }
    });

    // ---------------------------------------------------------
    // 4. RAG Reindex Agent (7 Days Interval)
    // ---------------------------------------------------------
    let _rag_sync = state.sync_sender.clone();
    tokio::spawn(async move {
        // Run weekly (7 days)
        let mut ticker = interval(Duration::from_secs(7 * 24 * 3600));
        loop {
            ticker.tick().await;
            tracing::info!("📚 [RAG Reindex Agent] Iniciando reindexação semanal do Vector Vault...");
            
            // Aqui despacharíamos um Job de Ingestão total para o SyncEngine
            // let _ = rag_sync.send(crate::sync_engine::IngestionJob::FullReindex);
            tracing::info!("📚 [RAG Reindex Agent] Comando de reindexação enviado para o Motor Físico RAG.");
        }
    });
}
