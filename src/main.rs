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
mod api_mesh;
pub mod kms;
pub mod network;
pub mod rewoo;
pub mod ssh_gateway;
pub mod plan_execute;
pub mod mcp;
pub mod ssh_mesh_connector;
pub mod mesh_installer;
pub mod mesh_router;

use axum::{routing::post, Router, response::IntoResponse, http::{header, StatusCode, Uri}};
use reqwest::Client;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
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

// Estado Global (Músculo Cíbrido) compartilhado entre Threads
pub struct AppState {
    pub http_client: Client,
    pub vault_path: std::path::PathBuf,
    pub telemetry: Arc<RwLock<telemetry::TelemetryState>>,
    pub log_sender: broadcast::Sender<models::LogEntry>,
    pub sync_sender: broadcast::Sender<sync_engine::IngestionJob>,
    pub db: sqlx::SqlitePool,
}

#[tokio::main]
async fn main() {
    // Inicializa a Telemetria (Logs avançados estilo Uvicorn)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sovereign_core=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🦀 Sovereign Core (Rust) Initializing...");

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

    // Inicializa o Motor Físico RAG O.S Multi-Drive
    let r_sync_engine = sync_engine::SyncEngine::new(db_pool.clone());
    r_sync_engine.start_watcher().await;
    let sync_tx = r_sync_engine.tx.clone();

    // Inicializa o Corredor de Eventos Cíbridos (Capacidade p/ 100 Logs antes de lag)
    let (log_tx, _) = broadcast::channel(100);

    // Cria o Roteador Axum (A fundação dos Cíbridos) com Contexto Acoplado
    let state = Arc::new(AppState {
        http_client: Client::new(),
        vault_path: active_vault,
        telemetry: Arc::new(RwLock::new(telemetry::TelemetryState::new())),
        log_sender: log_tx,
        sync_sender: sync_tx,
        db: db_pool,
    });

    let app = Router::new()
        // ------------------ LLMOps Telemetry & Logs ------------------
        .route("/v1/analytics/telemetry", axum::routing::get(api::telemetry_snapshot_handler))
        .route("/v1/logs", axum::routing::get(api::realtime_logs_handler))
        // ------------------ RAG & SOVEREIGN DRIVES (Workspaces O.S) --------------------------
        .route("/v1/workspaces", axum::routing::get(api_vault::list_workspaces_handler)
            .post(api_vault::create_workspace_handler))
        .route("/v1/workspaces/:workspace_id", axum::routing::delete(api_vault::delete_workspace_handler))
        .route("/v1/workspaces/:workspace_id/tree", axum::routing::get(api_vault::workspace_tree_handler))
        .route("/v1/vault/graph", axum::routing::get(api_vault::vault_graph_handler))
        .route("/v1/vault/document/*id", axum::routing::get(api_vault::vault_document_read)
            .put(api_vault::vault_document_write))
        .route("/v1/vault/fs/create", axum::routing::post(api_vault::vault_fs_create_handler))
        .route("/v1/vault/fs/rename", axum::routing::put(api_vault::vault_fs_rename_handler))
        .route("/v1/vault/fs/move", axum::routing::put(api_vault::vault_fs_move_handler))
        .route("/v1/vault/fs/delete", axum::routing::delete(api_vault::vault_fs_delete_handler))
        // ------------------ Historical Chat API (Sovereign O.S) ------
        .route("/v1/sessions", axum::routing::get(api_chat::get_sessions_handler))
        .route("/v1/sessions/:id", axum::routing::get(api_chat::get_session_by_id_handler)
            .delete(api_chat::delete_session_handler))
        // ------------------ Projects & Tasks (Kanban O.S) ------------
        .route("/v1/projects", axum::routing::get(api_projects::get_projects_handler)
            .post(api_projects::create_project_handler))
        .route("/v1/projects/:id", axum::routing::delete(api_projects::delete_project_handler))
        .route("/v1/projects/:project_id/tasks", axum::routing::get(api_projects::get_project_tasks_handler).post(api_projects::create_task_handler))
        .route("/v1/tasks/:id", axum::routing::delete(api_projects::delete_task_handler)
            .put(api_projects::update_task_handler))
        // ------------------ Settings & Identity O.S -----------
        .route("/v1/settings", axum::routing::get(api_settings::get_system_settings_handler)
            .post(api_settings::set_system_settings_handler))
        .route("/v1/settings/ollama_clusters", axum::routing::get(api_settings::get_ollama_clusters_handler)
            .post(api_settings::set_ollama_clusters_handler))
        // ------------------ Chat Endpoints ------------------
        .route("/opencode/v1/chat/completions", post(api::chat_completions_handler))
        .route("/v1/chat/completions", post(api::chat_completions_handler))
        .route("/chat/completions", post(api::chat_completions_handler))
        .route("/v1/responses", post(realtime::realtime_responses_handler))
        // ------------------ Tools & Agentic Capabilities ----
        .route("/v1/tools/read_vault_file", post(api_tools::read_vault_file_handler))
        .route("/v1/tools/create_kanban_task", post(api_tools::create_kanban_task_handler))
        .route("/responses", post(realtime::realtime_responses_handler))
        // ------------------ MESH P2P ROOTS --------------------------
        .route("/v1/mesh/handshake", axum::routing::get(api_mesh::mesh_handshake_handler))
        .route("/v1/mesh/connect", post(api_mesh::mesh_connect_handler))
        .route("/v1/mesh/tunnels", axum::routing::get(api_mesh::mesh_tunnels_status_handler))
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

    // Parsing CLI arguments to allow dynamic Host binding (Desktop vs Hub Mode)
    let args: Vec<String> = std::env::args().collect();
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
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await.unwrap();
}
