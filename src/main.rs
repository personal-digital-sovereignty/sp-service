mod api;
mod models;
mod realtime;
mod rag;
mod telemetry;
mod sync_engine;
mod db;
mod api_chat;
mod api_vault;
mod api_projects;
mod api_settings;
mod api_tools;
mod api_rag;
mod api_trainer;
mod auto_evaluator;
mod api_mesh;
mod api_mcp;
mod api_multimodal;
pub mod api_gateway;
pub mod kms;
pub mod network;
pub mod rewoo;
pub mod ssh_gateway;
pub mod plan_execute;
pub mod mcp;
pub mod ssh_mesh_connector;
pub mod mesh_installer;
pub mod mesh_router;
pub mod os_installer;
pub mod guardrails;
pub mod research;
pub mod adblocker;
pub mod multimodal;
pub mod office_parser;
pub mod sandbox;

use axum::{routing::post, Router, response::IntoResponse, http::{header, StatusCode, Uri}};
use reqwest::Client;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct LocalTimer;

impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{} ", chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%z"))
    }
}
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../svelte-ui/build/"]
struct BackendWebUI;

async fn spa_static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        path = "index.html";
    }

    match BackendWebUI::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if path.contains('.') {
                tracing::warn!("🚨 404: Static asset não encontrado -> {}", uri);
                return (StatusCode::NOT_FOUND, "404 Sovereign Fallback (Asset Not Found)").into_response();
            }
            if let Some(index) = BackendWebUI::get("index.html") {
                ([(header::CONTENT_TYPE, "text/html")], index.data).into_response()
            } else {
                (StatusCode::NOT_FOUND, "Sovereign Web-UI Offline").into_response()
            }
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Sovereign falhou ao monitorar Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Sovereign falhou ao conectar listener SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::warn!("🛑 SOVEREIGN SHUTDOWN ACTIVATE: Parada forçada via POSIX. Desocupando todas as portas e limpando memória RAM!");
    
    // Extirpa o processo na raiz. Garante que as dezenas de `tokio::spawn` em Background 
    // (SyncEngine, Daemons) morram instantaneamente e a Porta seja liberada para o Kernel O.S.
    std::process::exit(0);
}

/// Invoca dinamicamente o Binário Local C++ de Visão (sd.cpp) se ele estiver presente,
/// garantindo uma subida atrelada ao backend da aplicação "Zero-Config".
fn spawn_vision_daemon() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/jefersonlopes".to_string());
    let vision_path = std::path::PathBuf::from(&home).join("Sovereign_LLM/Vision");
    
    // Determina o caminho exato independente do script de compilação
    let bin1 = vision_path.join("sd_bin/sd-server");
    let bin2 = vision_path.join("stable-diffusion.cpp/build/bin/sd-server");
    let bin3 = vision_path.join("sd_bin/sd"); // Retro-compatibilidade com versões pré-2024
    let bin4 = vision_path.join("stable-diffusion.cpp/build/bin/sd");
    
    let target_bin = if bin1.exists() {
        Some(bin1)
    } else if bin2.exists() {
        Some(bin2)
    } else if bin3.exists() {
        Some(bin3)
    } else if bin4.exists() {
        Some(bin4)
    } else {
        None
    };

    if let Some(bin) = target_bin {
        let models_dir = vision_path.join("models");
        
        // Scan the directory for the first file ending in .gguf
        let model = std::fs::read_dir(&models_dir)
            .ok()
            .and_then(|mut entries| {
                entries.find_map(|entry| {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                            return Some(path);
                        }
                    }
                    None
                })
            });

        if let Some(model_path) = model {
            tracing::info!("🎨 [Multimodal Vision] Cérebro Visual ({}) Detectado. Iniciando Daemon (Porta 7860)...", model_path.file_name().unwrap_or_default().to_string_lossy());
            
            std::thread::spawn(move || {
                let _ = std::process::Command::new(bin)
                    .arg("--listen-port")
                    .arg("7860")
                    .arg("-m")
                    .arg(model_path)
                    // Isolamento acústico de Log (SD.cpp pode ser muito verboso, mantemos o TUI limpo)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            });
        }
    }
}

// Estado Global (Músculo Cíbrido) compartilhado entre Threads
pub struct AppState {
    pub http_client: Client,
    pub vault_path: std::path::PathBuf,
    pub telemetry: Arc<RwLock<telemetry::TelemetryState>>,
    pub log_sender: broadcast::Sender<models::LogEntry>,
    pub sync_sender: broadcast::Sender<sync_engine::IngestionJob>,
    pub db: sqlx::SqlitePool,
    pub adblock_engine: adblocker::AdblockHandle,
}

#[tokio::main]
async fn main() {
    // Inicializa a Telemetria (Logs avançados estilo Uvicorn)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sovereign_core=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_timer(LocalTimer))
        .init();

    tracing::info!("🦀 Sovereign Core (Rust) Initializing...");

    // BOOT ASYNC DAEMONS PARALELOS
    spawn_vision_daemon();
    
    // BOOT SOVEREIGN HERMETIC SANDBOX (PYTHON)
    sandbox::setup_python_sandbox().await;

    // ------------------------------------------------------------------------------------------
    // [MESH ENTRYPOINT] Zero-Touch Auto-Deployment Catch (Interactive CLI Installer Bypass)
    // ------------------------------------------------------------------------------------------
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--deploy-mesh") {
        mesh_installer::run_interactive_installer().await;
        std::process::exit(0);
    }

    // Instancia a Identidade de Rede (O Alias e Segredos JWT para a Sessão LAN)
    let identity = network::init_network_identity();

    // Inicializa as Engrenagens do RAG Indexer nativo (Std::fs)
    let active_vault = rag::init_vault();

    // Invoca o SQLite Master O.S
    let db_pool = db::init_pool().await;

    // Hotfix 0.9.4: Windows DOS Extended Path Cleanup (Remove \\?\ quebrando a interface e prompts)
    tracing::info!("🧹 [Sovereign DB] Higienizando possíveis rastros de prefixos DOS (\\?\\) em Workspaces...");
    let _ = sqlx::query(r#"UPDATE workspaces SET path = REPLACE(path, '\\?\', '')"#).execute(&db_pool).await;
    let _ = sqlx::query(r#"UPDATE sensus_documents SET path = REPLACE(path, '\\?\', '')"#).execute(&db_pool).await;

    // Inicializa o Motor Físico RAG O.S Multi-Drive
    let r_sync_engine = sync_engine::SyncEngine::new(db_pool.clone());
    r_sync_engine.start_watcher().await;
    let sync_tx = r_sync_engine.tx.clone();

    // Inicializa o Corredor de Eventos Cíbridos (Capacidade p/ 100 Logs antes de lag)
    let (log_tx, _) = broadcast::channel(100);

    // Despacha o Daemon Assíncrono do AdGuard (Toda a lógica RAG depende das assinaturas dele)
    let adblock_handle = adblocker::start_adblock_daemon(active_vault.clone(), db_pool.clone());

    // Cria o Roteador Axum (A fundação dos Cíbridos) com Contexto Acoplado
    let state = Arc::new(AppState {
        http_client: Client::new(),
        vault_path: active_vault,
        telemetry: Arc::new(RwLock::new(telemetry::TelemetryState::new())),
        log_sender: log_tx,
        sync_sender: sync_tx,
        db: db_pool,
        adblock_engine: adblock_handle,
    });

    // Boot the Auto-Evaluator (LLM-as-a-Judge Mesh Loop)
    auto_evaluator::start_evaluator_loop(state.clone()).await;

    let app = Router::new()
        // ------------------ System Actions & Launchers ---------------
        .route("/v1/system/launch-gui", axum::routing::post(api::launch_gui_handler))
        // ------------------ LLMOps Telemetry & Logs ------------------
        .route("/v1/analytics/telemetry", axum::routing::get(api::telemetry_snapshot_handler))
        .route("/v1/analytics/hallucinations", axum::routing::get(api_trainer::get_hallucinations_ledger_handler))
        .route("/v1/logs", axum::routing::get(api::realtime_logs_handler))
        // ------------------ RAG & SOVEREIGN DRIVES (Workspaces O.S) --------------------------
        .route("/v1/workspaces", axum::routing::get(api_vault::list_workspaces_handler)
            .post(api_vault::create_workspace_handler))
        .route("/v1/workspaces/:workspace_id", axum::routing::delete(api_vault::delete_workspace_handler))
        .route("/v1/workspaces/:workspace_id/tree", axum::routing::get(api_vault::workspace_tree_handler))
        .route("/v1/vault/graph", axum::routing::get(api_vault::vault_graph_handler))
        .route("/v1/vault/document/*id", axum::routing::get(api_vault::vault_document_read)
            .put(api_vault::vault_document_write))
        .route("/v1/vault/media", axum::routing::get(api_vault::vault_media_handler))
        .route("/v1/vault/office_chart", axum::routing::get(api_vault::vault_office_chart_handler))
        .route("/v1/vault/fs/create", axum::routing::post(api_vault::vault_fs_create_handler))
        .route("/v1/vault/fs/rename", axum::routing::put(api_vault::vault_fs_rename_handler))
        .route("/v1/vault/fs/move", axum::routing::put(api_vault::vault_fs_move_handler))
        .route("/v1/vault/fs/delete", axum::routing::delete(api_vault::vault_fs_delete_handler))
        .route("/v1/vault/documents", axum::routing::get(api_vault::vault_documents_handler))
        .route("/v1/vault/search", axum::routing::get(api_vault::vault_documents_search_handler))
        // ------------------ Historical Chat API (Sovereign O.S) ------
        .route("/v1/sessions", axum::routing::get(api_chat::get_sessions_handler))
        .route("/v1/sessions/:id", axum::routing::get(api_chat::get_session_by_id_handler)
            .put(api_chat::update_session_handler)
            .delete(api_chat::delete_session_handler))
        // ------------------ P2P Sovereign Mesh Router ----------------
        .route("/v1/mesh/handshake", axum::routing::get(api_mesh::mesh_handshake_handler))
        .route("/v1/mesh/connect", axum::routing::post(api_mesh::mesh_connect_handler))
        .route("/v1/mesh/tunnels", axum::routing::get(api_mesh::mesh_tunnels_status_handler))
        .route("/v1/mesh/sync/document", axum::routing::post(api_mesh::mesh_sync_document_handler))
        // ------------------ Projects & Tasks (Kanban O.S) ------------
        .route("/v1/projects", axum::routing::get(api_projects::get_projects_handler)
            .post(api_projects::create_project_handler))
        .route("/v1/projects/:id", axum::routing::delete(api_projects::delete_project_handler).put(api_projects::update_project_handler))
        .route("/v1/projects/:project_id/tasks", axum::routing::get(api_projects::get_project_tasks_handler).post(api_projects::create_task_handler))
        .route("/v1/projects/:project_id/documents", axum::routing::get(api_projects::get_project_documents_handler).post(api_projects::link_project_document_handler))
        .route("/v1/projects/:project_id/documents/:encoded_path", axum::routing::delete(api_projects::unlink_project_document_handler))
        .route("/v1/tasks/:id", axum::routing::delete(api_projects::delete_task_handler)
            .put(api_projects::update_task_handler))
        // ------------------ Settings & Identity O.S -----------
        .route("/v1/settings", axum::routing::get(api_settings::get_system_settings_handler)
            .post(api_settings::set_system_settings_handler))
        .route("/v1/settings/ollama_clusters", axum::routing::get(api_settings::get_ollama_clusters_handler)
            .post(api_settings::set_ollama_clusters_handler))
        .route("/v1/settings/searxng", axum::routing::get(api_settings::get_searxng_nodes_handler)
            .post(api_settings::set_searxng_nodes_handler))
        .route("/v1/settings/cold_storage", axum::routing::get(api_settings::get_cold_storage_handler)
            .post(api_settings::set_cold_storage_handler))
        .route("/v1/system/export_config", axum::routing::get(api_settings::export_config_handler))
        .route("/v1/system/import_config", axum::routing::post(api_settings::import_config_handler))
        .route("/v1/system/available_models", axum::routing::get(api_settings::get_available_models_handler))
        .route("/v1/system/docs/user_guide", axum::routing::get(api_settings::get_user_guide_handler))
        // ------------------ RAG Engine Command Center ----------
        .route("/v1/engineer/rag/rules", axum::routing::get(api_rag::get_routing_rules_handler)
            .post(api_rag::create_routing_rule_handler))
        .route("/v1/engineer/rag/rules/:id", axum::routing::delete(api_rag::delete_routing_rule_handler))
        .route("/v1/engineer/rag/models", axum::routing::get(api_rag::get_remote_models_handler)
            .post(api_rag::create_remote_model_handler))
        .route("/v1/engineer/rag/models/:id", axum::routing::delete(api_rag::delete_remote_model_handler))
        .route("/v1/engineer/rag/gaps", axum::routing::get(api_rag::get_knowledge_gaps_handler))
        .route("/v1/engineer/rag/gaps/:id", axum::routing::delete(api_rag::delete_knowledge_gap_handler).put(api_rag::resolve_knowledge_gap_handler))
        .route("/v1/engineer/rag/radar", axum::routing::get(api_rag::get_radar_metrics_handler))
        // ------------------ Model Trainer Engine ----------------
        .route("/v1/engineer/trainer/stats", axum::routing::get(api_trainer::trainer_stats_handler))
        .route("/v1/engineer/trainer/control", axum::routing::post(api_trainer::trainer_control_handler))
        .route("/v1/engineer/trainer/distill", axum::routing::post(api_trainer::run_distillation_handler))
        .route("/v1/engineer/trainer/finetune", axum::routing::post(api_trainer::run_finetuning_handler))
        .route("/v1/engineer/trainer/deep-research", axum::routing::post(api_trainer::run_deep_research_handler))
        .route("/v1/engineer/trainer/deep-research/cancel", axum::routing::post(api_trainer::cancel_deep_research_handler))
        .route("/v1/engineer/trainer/unsloth-monitor", axum::routing::get(api_trainer::unsloth_monitor_sse_handler))
        .route("/v1/research/staging", axum::routing::get(api_trainer::get_staged_research_handler))
        .route("/v1/research/staging/:id", axum::routing::delete(api_trainer::discard_staged_research_handler))
        .route("/v1/research/staging/:id/commit", axum::routing::post(api_trainer::commit_staged_research_handler))
        // ------------------ Multimodal Endpoints ------------------
        .route("/v1/images/generations", post(api_multimodal::generate_image_handler))
        // ------------------ Chat Endpoints ------------------
        .route("/opencode/v1/chat/completions", post(api::chat_completions_handler))
        .route("/v1/chat/completions", post(api::chat_completions_handler))
        .route("/chat/completions", post(api::chat_completions_handler))
        .route("/v1/responses", post(realtime::realtime_responses_handler))
        .route("/v1/feedback", post(api::feedback_handler))
        // ------------------ Tools & Agentic Capabilities ----
        .route("/v1/tools/read_vault_file", post(api_tools::read_vault_file_handler))
        .route("/v1/tools/create_kanban_task", post(api_tools::create_kanban_task_handler))
        // ------------------ Master Control Program (MCP Server) -------
        .route("/v1/mcp/sse", axum::routing::get(api_mcp::mcp_sse_handler))
        .route("/v1/mcp/message", axum::routing::post(api_mcp::mcp_message_handler))
        .route("/v1/multimodal/audio/transcribe", axum::routing::post(api_multimodal::audio_transcribe_handler))
        // Bypass pacificador para TUI que tenta carregar modelos disponíveis antes da call
        .route("/v1/models", axum::routing::get(|| async {
            axum::Json(serde_json::json!({
                "object": "list",
                "data": [
                    {"id": "gpt-4", "object": "model", "owned_by": "openai"},
                    {"id": "gpt-3.5-turbo", "object": "model", "owned_by": "openai"}
                ]
            }))
        }))
        // Emparelhamento Mágico de Rede (QR Code Endpoint)
        .route("/v1/network/pair", axum::routing::get(network::get_pairing_info_handler))
        // Static Web-UI Fallback Server (SPA HTML5 History Mode)
        .fallback(spa_static_handler)
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn(network::lan_auth_guard))
        .with_state(state);

    // Parsing CLI arguments to allow dynamic Host binding or Headless Installation
    let args: Vec<String> = std::env::args().collect();
    
    if args.iter().any(|a| a == "--setup") {
        tracing::info!("🛠️ [Headless Wizard] Iniciando rotina nativa de instalação OS/Daemons...");
        os_installer::run_headless_setup().await;
        std::process::exit(0);
    }

    let mut host_address = "127.0.0.1:38001".to_string(); // Default to secure localhost

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--host" && i + 1 < args.len() {
            host_address = format!("{}:38001", args[i + 1]);
            i += 1;
        }
        i += 1;
    }

    // Configura o TcpListener com Port Escaping progressivo (Evita colisão EADDRINUSE no Desktop)
    let mut listener = None;
    let mut final_port = 38001;

    for port in 38001..=38010 {
        let bind_target = if host_address.contains(":") {
            // Se via CLI vier "0.0.0.0:38001", a gente fatia e substitui pela porta da iteração
            let base_ip = host_address.split(':').next().unwrap_or("127.0.0.1");
            format!("{}:{}", base_ip, port)
        } else {
            format!("{}:{}", host_address, port)
        };

        match tokio::net::TcpListener::bind(&bind_target).await {
            Ok(l) => {
                listener = Some(l);
                final_port = port;
                break;
            }
            Err(e) => {
                tracing::warn!("Port {} ocupada. Tentando próxima... ({})", port, e);
            }
        }
    }

    let listener = listener.expect("Sovereign Error: Todas as portas de 38001 a 38010 estão ocupadas!");
    
    tracing::info!("🚀 Sovereign Core Listening Resiliently on {}", listener.local_addr().unwrap());
    
    // Inicia o Beacon mDNS apenas depois da porta confirmada
    network::start_mdns_beacon(&identity.alias, final_port);
    
    // Inicia o Servidor Nativo (Conector tipado pra permitir extração de IP pela LAN Guard)
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
