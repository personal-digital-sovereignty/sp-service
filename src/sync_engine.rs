use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config};
use std::path::Path;
use tokio::sync::broadcast;
use serde::Serialize;
use tracing::{info, warn, error};
use std::time::Duration;
use std::fs;
use uuid::Uuid;

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
            #[allow(dead_code)]
            struct PathRow { id: i64, path: String }

            if let Ok(rows) = sqlx::query_as::<_, PathRow>("SELECT id, path FROM workspaces").fetch_all(&db).await {
                for row in rows {
                    let ws_path = Path::new(&row.path);
                    if ws_path.exists() && ws_path.is_dir() {
                        info!("🚀 [Sensus Sync] Varrida Cíbrida Inicial do Workspace: {:?}", ws_path);
                        for entry in walkdir::WalkDir::new(ws_path).into_iter().filter_map(|e| e.ok()) {
                            let path = entry.path();
                            if path.is_file() {
                                let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                let path_str = path.to_string_lossy().to_string();
                                
                                if path_str.contains("node_modules") || path_str.contains(".git") || path_str.contains(".venv") || filename.starts_with('.') {
                                    continue;
                                }
                                
                                let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                                if ["png", "jpg", "jpeg", "gif", "webp", "svg", "pdf", "mp4", "mp3", "zip", "tar", "gz", "rar"].contains(&ext.as_str()) {
                                    continue;
                                }
                                
                                let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sensus_documents WHERE file_path = ?)")
                                    .bind(&path_str)
                                    .fetch_one(&db).await.unwrap_or(false);
                                
                                if !exists {
                                    let job = IngestionJob {
                                        id: Uuid::new_v4().to_string(),
                                        filename: filename.clone(),
                                        status: "queued".to_string(),
                                        current_step: 0,
                                        progress_ms: 0,
                                    };
                                    let _ = current_tx.send(job.clone());
                                    let process_tx = current_tx.clone();
                                    let process_db = db.clone();
                                    let file_path_clone = path_str;
                                    tokio::spawn(async move {
                                        Self::process_ingestion_pipeline(job, file_path_clone, process_tx, process_db).await;
                                    });
                                }
                            }
                        }

                        // 2. Proteção de Polling Limitado (Config)
                        // A crate notify resolve internamente se precisa fazer fallback p/ Polling (em NFS/Network Drives).
                        // Setamos explicitly o poll_interval para ser gentil com o IO IOPS do Hardware!
                        let config = Config::default()
                            .with_poll_interval(Duration::from_millis(5000))
                            .with_compare_contents(true);
                            
                        // Re-criando o watcher com a config gentil (se falhar, mantém)
                        let _ = watcher.configure(config);
                        
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
                            if let Some(root) = ignore_file
                                && let Ok(content) = fs::read_to_string(root.join(".sovereignignore")) {
                                    for line in content.lines() {
                                        let trimmed = line.trim();
                                        if !trimmed.is_empty() && !trimmed.starts_with('#') {
                                            ignored_patterns.push(trimmed.replace("/", ""));
                                        }
                                    }
                                }

                            // Verifica se o caminho absoluto contém qualquer diretório proibido
                            let is_ignored = ignored_patterns.iter().any(|pattern| path_str.contains(pattern));
                            
                            // Impede re-processamento de backups, arquivos ocultos e Pastas Bloqueadas
                            if is_ignored || filename.starts_with('.') || filename.ends_with('~') {
                                continue;
                            }
                            
                            let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                            if ["png", "jpg", "jpeg", "gif", "webp", "svg", "pdf", "mp4", "mp3", "zip", "tar", "gz", "rar"].contains(&ext.as_str()) {
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
                            
                            let process_tx = current_tx.clone();
                            let process_db = db.clone();
                            let file_path_clone = path_str.clone();
                            tokio::spawn(async move {
                                Self::process_ingestion_pipeline(job, file_path_clone, process_tx, process_db).await;
                            });
                        }
                    }
                }
            }
        });
    }

    async fn process_ingestion_pipeline(mut job: IngestionJob, file_path: String, tx: broadcast::Sender<IngestionJob>, db: sqlx::SqlitePool) {
        // Step 1: O OCR/Parse (File I/O Nativo)
        job.status = "processing".to_string();
        job.current_step = 0;
        let _ = tx.send(job.clone());
        
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                error!("❌ [The Dad] Falha Nativa ao ler {}: {}", file_path, e);
                job.status = "error".to_string();
                let _ = tx.send(job);
                return;
            }
        };

        // Step 2: Doc Chunking (Rayon Multithread Ciber-Paralelismo)
        job.current_step = 1;
        let _ = tx.send(job.clone());
        
        let content_for_rayon = content.clone();
        let (fallback_summary, chunks) = tokio::task::spawn_blocking(move || {
            use rayon::prelude::*;
            // Heurística de Chunking Multi-core: split por parágrafos longos
            let paragraphs: Vec<&str> = content_for_rayon.split("\n\n").filter(|s| !s.trim().is_empty()).collect();
            let processed_chunks: Vec<String> = paragraphs.into_par_iter()
                .map(|p| p.trim().to_string())
                .filter(|p| p.len() > 10)
                .collect();
            
            let summary = content_for_rayon.chars().take(500).collect::<String>();
            (summary, processed_chunks)
        }).await.unwrap_or_default();

        // Step 3: SLM Semantic Summary (The Dad Inference) via Socket Loopback
        job.current_step = 2;
        let _ = tx.send(job.clone());
        
        let client = reqwest::Client::new();
        let prompt = format!("Resuma em 1 único parágrafo objetivo a INTENÇÃO do seguinte texto de arquivo '{}':\n{}", job.filename, fallback_summary);
        
        let mut final_summary = fallback_summary.clone();
        
        // Chamada direta pro Ollama para desvio de fila do API Python Cíbrido
        if let Ok(resp) = client.post("http://127.0.0.1:11434/api/generate")
            .json(&serde_json::json!({
                "model": "llama3.2:latest", // Exemplo fixo por segurança
                "prompt": prompt,
                "stream": false
            }))
            .timeout(Duration::from_secs(30))
            .send().await
            && let Ok(json) = resp.json::<serde_json::Value>().await
                && let Some(res_text) = json.get("response").and_then(|r| r.as_str()) {
                    final_summary = res_text.trim().to_string();
                }

        // Step 4: SQLite Store (Atomic Write O(1))
        job.current_step = 3;
        let _ = tx.send(job.clone());
        
        // Recupera o Workspace ID com base no Caminho do Arquivo
        let workspace_id_raw: Option<i64> = sqlx::query_scalar("
            SELECT id FROM workspaces WHERE ? LIKE path || '%' LIMIT 1
        ")
        .bind(&file_path)
        .fetch_optional(&db).await.unwrap_or_default();
        let workspace_id = workspace_id_raw.map(|id| id.to_string()).unwrap_or_else(|| "default".to_string());

        for (i, chunk_text) in chunks.iter().enumerate() {
            let chunk_ref = format!("{}_{}", job.id, i);
            let meta_json = serde_json::json!({
                "summary": final_summary,
                "chunk_index": i,
                "total_chunks": chunks.len(),
            }).to_string();

            let _ = sqlx::query("
                INSERT INTO sovereign_chunks (uuid_reference, workspace_id, file_path, text_content, metadata_json)
                VALUES (?, ?, ?, ?, ?)
            ")
            .bind(&chunk_ref)
            .bind(&workspace_id)
            .bind(&file_path)
            .bind(chunk_text)
            .bind(&meta_json)
            .execute(&db).await;
        }

        // Injeta o documento principal na Tabela Mestre
        let _ = sqlx::query("
            INSERT INTO sensus_documents (id, workspace_id, file_path, content_raw, summary, last_modified)
            VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(file_path) DO UPDATE SET 
                content_raw = excluded.content_raw,
                summary = excluded.summary,
                last_modified = CURRENT_TIMESTAMP
        ")
        .bind(&job.id)
        .bind(&workspace_id)
        .bind(&file_path)
        .bind(&content)
        .bind(&final_summary)
        .execute(&db).await;

        job.status = "completed".to_string();
        job.current_step = 4;
        let _ = tx.send(job.clone());
        info!("🧬 [The Dad/Mom Rust] Conhecimento Engolido Atomicamente via Rayon: {}", job.filename);

        // ==========================================
        // MESH P2P: DISTRIBUIÇÃO DE CONHECIMENTO CÍBRIDO
        // ==========================================
        let workspace_name: String = sqlx::query_scalar("SELECT name FROM workspaces WHERE id = ?")
            .bind(&workspace_id)
            .fetch_optional(&db).await.unwrap_or_default().unwrap_or_else(|| "Sovereign Mesh Roaming".to_string());

        let mut mesh_chunks = Vec::new();
        for (i, chunk_text) in chunks.iter().enumerate() {
            let chunk_ref = format!("{}_{}", job.id, i);
            let meta_json = serde_json::json!({
                "summary": final_summary,
                "chunk_index": i,
                "total_chunks": chunks.len(),
            }).to_string();
            
            mesh_chunks.push(serde_json::json!({
                "uuid_reference": chunk_ref,
                "text_content": chunk_text,
                "metadata_json": meta_json
            }));
        }

        let sync_payload = serde_json::json!({
            "document_id": job.id,
            "workspace_name": workspace_name, // Permite que o outro Nó tente rotear pelo nome
            "file_path": file_path,
            "content_raw": content,
            "summary": final_summary,
            "chunks": mesh_chunks
        });

        let tunnels = crate::ssh_mesh_connector::ACTIVE_MESH_TUNNELS.lock().await;
        if !tunnels.is_empty() {
            info!("📡 [Sovereign Mesh P2P] Transmitindo Conhecimento recém-ingerido para {} Nós pares...", tunnels.len());
            let client = reqwest::Client::new();

            for (port, (uri, _)) in tunnels.iter() {
                let target_url = format!("http://127.0.0.1:{}/v1/mesh/sync/document", port);
                let payload_clone = sync_payload.clone();
                let uri_clone = uri.clone();
                let client_clone = client.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = client_clone.post(&target_url).json(&payload_clone).timeout(std::time::Duration::from_secs(10)).send().await {
                        warn!("⚠️ [Sovereign Mesh] Falha ao sincronizar com Nó Pareado '{}': {}", uri_clone, e);
                    } else {
                        info!("✅ [Sovereign Mesh] Ingestão sincronizada com Nó Pareado '{}'.", uri_clone);
                    }
                });
            }
        }
    }
}
