use tokio::sync::{mpsc, oneshot};
use adblock::engine::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use base64::prelude::*;
use base64::Engine as Base64Engine;

const BLOCKLIST_SOURCES_B64: &[&str] = &[
    "aHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL0FkZ3VhcmRUZWFtL0FkZ3VhcmRGaWx0ZXJzL21hc3Rlci9CYXNlRmlsdGVyL3NlY3Rpb25zL2Fkc2VydmVycy50eHQ=",
    "aHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL0FkZ3VhcmRUZWFtL0FkZ3VhcmRGaWx0ZXJzL21hc3Rlci9TcHlGaWx0ZXIvc2VjdGlvbnMvdHJhY2tpbmdfc2VydmVycy50eHQ=",
];

pub enum AdblockMessage {
    Check { url: String, reply: oneshot::Sender<bool> },
    Reload
}

#[derive(Clone)]
pub struct AdblockHandle {
    pub sender: mpsc::Sender<AdblockMessage>,
}

impl AdblockHandle {
    pub async fn check_url(&self, url: &str) -> bool {
        let (tx, rx) = oneshot::channel();
        if self.sender.send(AdblockMessage::Check { url: url.to_string(), reply: tx }).await.is_ok() {
            return rx.await.unwrap_or(false);
        }
        false
    }
}

pub fn start_adblock_daemon(
    vault_path: std::path::PathBuf,
    db: sqlx::SqlitePool
) -> AdblockHandle {
    let (tx, mut rx) = mpsc::channel::<AdblockMessage>(500);
    let list_cache_path = vault_path.join("_agents").join("adblock_matrix.txt");
    
    // 1. Thread Sólida O.S (Actor) para encapsular a Engine Não-Send do Brave
    let list_cache_path_thread = list_cache_path.clone();
    std::thread::spawn(move || {
        tracing::info!("🛡️ [Pi-Hole Worker] Iniciando Thread Isolada para a Engine C/Rust.");
        let mut engine = build_engine_from_disk(&list_cache_path_thread);
        
        while let Some(msg) = rx.blocking_recv() {
            match msg {
                AdblockMessage::Check { url, reply } => {
                    // Passa 'document' para que filtros third-party (Analytics) não bloqueiem domínios raiz inteiros (Ex: cnn.com)
                    let check_req = adblock::request::Request::new(&url, &url, "document").unwrap_or_else(|_| adblock::request::Request::new("http://fallback", "http://fallback", "document").unwrap());
                    let result = engine.check_network_request(&check_req);
                    let _ = reply.send(result.matched);
                },
                AdblockMessage::Reload => {
                    tracing::info!("♻️ [Pi-Hole Worker] Recarregando assinaturas frescas do disco na Engine...");
                    engine = build_engine_from_disk(&list_cache_path_thread);
                    tracing::info!("🔒 Sovereign Shield engatilhado localmente com Sucesso.");
                }
            }
        }
    });

    // 2. Daemon Assíncrono (Atualizador Diário)
    let handler_tx = tx.clone();
    tokio::spawn(async move {
        // Boostrap Analytics Table
        let _ = sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS analytics (
                id TEXT PRIMARY KEY,
                val_int INTEGER DEFAULT 0,
                val_str TEXT
            );
        "#).execute(&db).await;
        let _ = sqlx::query("INSERT OR IGNORE INTO analytics (id, val_int) VALUES ('total_trackers_blocked', 0)").execute(&db).await;

        loop {
            let mut needs_update = true;
            if let Ok(metadata) = tokio::fs::metadata(&list_cache_path).await
                && let Ok(modified) = metadata.modified()
                    && let Ok(age) = std::time::SystemTime::now().duration_since(modified)
                        && age.as_secs() < 86400 { // 24h
                            needs_update = false;
                        }

            if needs_update {
                tracing::info!("📡 [Pi-Hole Updater] Cache obsoleto. Fazendo Download Assíncrono das matrizes Tracker...");
                let mut combined_rules = String::new();
                let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build().unwrap_or_default();
                
                for encoded_url in BLOCKLIST_SOURCES_B64 {
                    // WAF Evasion: Decode on the fly so DPI doesn't block the static binary
                    if let Ok(decoded_bytes) = BASE64_STANDARD.decode(encoded_url)
                        && let Ok(url) = String::from_utf8(decoded_bytes)
                            && let Ok(resp) = client.get(&url).send().await
                                && let Ok(text) = resp.text().await {
                                    combined_rules.push_str(&text);
                                    combined_rules.push('\n');
                                }
                }

                if !combined_rules.is_empty() {
                    let _ = tokio::fs::create_dir_all(list_cache_path.parent().unwrap()).await;
                    if tokio::fs::write(&list_cache_path, &combined_rules).await.is_ok() {
                        tracing::info!("✅ Subsistema Pi-Hole salvo fisicamente. Sinalizando o Worker para Recarga Nuclear.");
                        let _ = handler_tx.send(AdblockMessage::Reload).await;
                    }
                }
            }
            // Hiberna por 6 horas antes de reavaliar necessidades de download
            tokio::time::sleep(tokio::time::Duration::from_secs(21600)).await;
        }
    });

    AdblockHandle { sender: tx }
}

fn build_engine_from_disk(path: &std::path::Path) -> Engine {
    let mut filter_set = FilterSet::new(true);
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let chunk = line.trim();
            if !chunk.is_empty() && !chunk.starts_with('!') {
                let _ = filter_set.add_filter(chunk, ParseOptions::default());
            }
        }
    } else {
        tracing::warn!("⚠️ Cache de AdGuard vazio ou lido antes do download. Assinatura será ignorada até o Loader Assíncrono finalizar.");
    }
    Engine::from_filter_set(filter_set, true)
}
