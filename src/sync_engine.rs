use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher, Config};
use std::path::{Path, PathBuf};
use tokio::sync::broadcast;
use serde::Serialize;
use tracing::{info, warn, error};
use std::time::Duration;
use std::fs;
use uuid::Uuid;
use tokio::time::sleep;

#[derive(Serialize, Clone, Debug)]
pub struct IngestionJob {
    pub id: String,
    pub filename: String,
    pub status: String,
    #[serde(rename = "currentStep")]
    pub current_step: u8,
    pub progress_ms: u64,
}

pub struct SyncEngine {
    pub tx: broadcast::Sender<IngestionJob>,
    db: sqlx::SqlitePool,
}

impl SyncEngine {
    pub fn new(db: sqlx::SqlitePool) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx, db }
    }

    pub async fn start_watcher(&self) {
        let db = self.db.clone();
        let current_tx = self.tx.clone();

        tokio::spawn(async move {
            info!("🔬 [Sensus Sync Engine] Acordando o Motor Multi-Drive Watcher...");
            
            let (watcher_tx, mut watcher_rx) = tokio::sync::mpsc::channel(100);

            // Closure síncrona do Notify
            let mut watcher: RecommendedWatcher = Watcher::new(
                move |res| {
                    if let Ok(event) = res {
                        let _ = watcher_tx.blocking_send(event);
                    }
                },
                Config::default(),
            ).expect("Sovereign falhou ao criar FSEvent Watcher");

            // 1. Coletar e Atrelar todos os Workspaces do Banco de Dados Dinâmico
            #[derive(sqlx::FromRow)]
            struct PathRow { path: String }

            if let Ok(rows) = sqlx::query_as::<_, PathRow>("SELECT path FROM workspaces").fetch_all(&db).await {
                for row in rows {
                    let ws_path = Path::new(&row.path);
                    if ws_path.exists() && ws_path.is_dir() {
                        // 2. Proteção de Polling Limitado (Config)
                        // A crate notify resolve internamente se precisa fazer fallback p/ Polling (em NFS/Network Drives).
                        // Setamos explicitly o poll_interval para ser gentil com o IO IOPS do Hardware!
                        let config = Config::default()
                            .with_poll_interval(Duration::from_millis(5000))
                            .with_compare_contents(true);
                            
                        // Re-criando o watcher com a config gentil (se falhar, mantém)
                        let _ = watcher.configure(config.clone());
                        
                        if let Err(e) = watcher.watch(ws_path, RecursiveMode::Recursive) {
                            error!("🚨 [Sensus Sync] Falha ao assistir drive secundário {:?}: {}", ws_path, e);
                        } else {
                            info!("✅ [Sensus Sync] Vigilância Periférica ativada em: {:?}", ws_path);
                        }
                    }
                }
            } else {
                warn!("⚠️ [Sensus Sync] Falha ao ler Tabela de Workspaces para The Watcher");
            }

            // Loop assíncrono recebendo eventos
            while let Some(event) = watcher_rx.recv().await {
                if let notify::event::EventKind::Create(_) | notify::event::EventKind::Modify(notify::event::ModifyKind::Data(_)) = event.kind {
                    for path in event.paths {
                        if path.is_file() {
                            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            let path_str = path.to_string_lossy().to_string();
                            
                            // 3. Parser do .sovereignignore on-the-fly + Hardcoded (Segurança Extrema)
                            let mut ignored_patterns = vec!["node_modules".to_string(), ".venv".to_string(), ".git".to_string(), "target".to_string()];
                            let ignore_file = path.ancestors().find(|a| a.join(".sovereignignore").exists());
                            if let Some(root) = ignore_file {
                                if let Ok(content) = fs::read_to_string(root.join(".sovereignignore")) {
                                    for line in content.lines() {
                                        let trimmed = line.trim();
                                        if !trimmed.is_empty() && !trimmed.starts_with('#') {
                                            ignored_patterns.push(trimmed.replace("/", ""));
                                        }
                                    }
                                }
                            }

                            // Verifica se o caminho absoluto contém qualquer diretório proibido
                            let is_ignored = ignored_patterns.iter().any(|pattern| path_str.contains(pattern));
                            
                            // Impede re-processamento de backups, arquivos ocultos e Pastas Bloqueadas
                            if is_ignored || filename.starts_with('.') || filename.ends_with('~') {
                                continue;
                            }

                            info!("📄 [Sensus Sync Engine] Novo artefato detectado: {}", filename);
                            
                            let job_id = Uuid::new_v4().to_string();
                            let job = IngestionJob {
                                id: job_id.clone(),
                                filename: filename.clone(),
                                status: "queued".to_string(),
                                current_step: 0,
                                progress_ms: 0,
                            };
                            
                            let _ = current_tx.send(job.clone());
                            
                            // Lança simulador de processamento assíncrono para o Frontend RAG Core
                            let sim_tx = current_tx.clone();
                            tokio::spawn(async move {
                                Self::simulate_ingestion_pipeline(job, sim_tx).await;
                            });
                        }
                    }
                }
            }
        });
    }

    async fn simulate_ingestion_pipeline(mut job: IngestionJob, tx: broadcast::Sender<IngestionJob>) {
        // Step 1: O OCR/Parse
        job.status = "processing".to_string();
        job.current_step = 0;
        let _ = tx.send(job.clone());
        sleep(Duration::from_millis(1500)).await;

        // Step 2: Doc Chunking
        job.current_step = 1;
        let _ = tx.send(job.clone());
        sleep(Duration::from_millis(1200)).await;

        // Step 3: Embedding Vector
        job.current_step = 2;
        let _ = tx.send(job.clone());
        sleep(Duration::from_millis(2500)).await;

        // Step 4: SQLite Store
        job.current_step = 3;
        let _ = tx.send(job.clone());
        sleep(Duration::from_millis(800)).await;

        // Completed
        job.status = "completed".to_string();
        job.current_step = 4;
        let _ = tx.send(job.clone());
        info!("✅ [Sensus Sync] Ingestão Concluída no Vault: {}", job.filename);
    }
}
