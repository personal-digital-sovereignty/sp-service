use sqlx::SqlitePool;
use tracing::{info, warn};
use std::time::Duration;

/// Inicia o Garbage Collector do Sovereign Vault em Background
pub async fn spawn_ephemeral_garbage_collector(pool: SqlitePool) {
    tokio::spawn(async move {
        info!("🗑️ [Garbage Collector] Sistema de purga de Memória Volátil (Notícias) ativado!");
        
        loop {
            // Roda a verificação de limpeza a cada 1 hora (3600 segundos)
            tokio::time::sleep(Duration::from_secs(3600)).await;
            
            info!("🗑️ [Garbage Collector] Identificando tokens mortos na memória temporal...");

            // Temporal coherence maintenance (routine integrity sweep)
            crate::prompt_vault::temporal_coherence_sweep(&pool).await;
            
            // Delete CASCADE remove as chunks filhas atreladas graças a FOREIGN KEY ON DELETE CASCADE
            match sqlx::query("DELETE FROM ephemeral_knowledge WHERE expires_at < CURRENT_TIMESTAMP")
                .execute(&pool)
                .await
            {
                Ok(res) => {
                    if res.rows_affected() > 0 {
                        info!("🧹 [Garbage Collector] Memória higienizada! {} documentos de notícias obsoletas purgados.", res.rows_affected());
                        
                        // Opcional: A Tabela Virtual vec_ephemeral_chunks precisa ser sincronizada (apenas apagar chunks que não tem parent)
                        let cleanup_res = sqlx::query("DELETE FROM vec_ephemeral_chunks WHERE chunk_id NOT IN (SELECT id FROM ephemeral_chunks)").execute(&pool).await;
                        if let Ok(c_res) = cleanup_res {
                            if c_res.rows_affected() > 0 {
                                info!("🧹 [Garbage Collector] Limpeza Profunda: {} vetores neurais desfiliados destruídos da memória virtual vec0.", c_res.rows_affected());
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("⚠️ [Garbage Collector] Falha ao tentar limpar o cérebro temporal: {}", e);
                }
            }
        }
    });
}
