mod api;
mod models;
mod realtime;
mod rag;
mod telemetry;

use axum::{routing::post, Router};
use reqwest::Client;
use std::sync::{Arc, RwLock};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Estado Global (Músculo Cíbrido) compartilhado entre Threads
pub struct AppState {
    pub http_client: Client,
    pub vault_path: std::path::PathBuf,
    pub telemetry: Arc<RwLock<telemetry::TelemetryState>>,
}

#[tokio::main]
async fn main() {
    // Inicializa a Telemetria (Logs avançados estilo Uvicorn)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sovereign_core=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🦀 Sovereign Core (Rust) Initializing...");

    // Inicializa as Engrenagens do RAG Indexer nativo (Std::fs)
    let active_vault = rag::init_vault();

    // Cria o Roteador Axum (A fundação dos Cíbridos) com Contexto Acoplado
    let state = Arc::new(AppState {
        http_client: Client::new(),
        vault_path: active_vault,
        telemetry: Arc::new(RwLock::new(telemetry::TelemetryState::new())),
    });

    let app = Router::new()
        // ------------------ LLMOps Telemetry ------------------
        .route("/v1/analytics/telemetry", axum::routing::get(api::telemetry_snapshot_handler))
        // ------------------ Chat Endpoints ------------------
        .route("/opencode/v1/chat/completions", post(api::chat_completions_handler))
        .route("/v1/chat/completions", post(api::chat_completions_handler))
        .route("/chat/completions", post(api::chat_completions_handler))
        .route("/v1/responses", post(realtime::realtime_responses_handler)) // Rota secreta da Vercel AI SDK! (Zod Protocol)
        .route("/responses", post(realtime::realtime_responses_handler))
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
        // Radar de Captura de Escapada (Se bater 404, logaremos a URL exata requisitada pelo OpenCode)
        .fallback(|uri: axum::http::Uri| async move {
            tracing::error!("🚨 404 NOT FOUND: O OpenCode tentou acessar o endpoint camuflado -> {}", uri);
            (reqwest::StatusCode::NOT_FOUND, format!("404 Sovereign Fallback: {} não existe no local.", uri))
        })
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Configura o TcpListener (Roda na porta 8001 para não colidir com o FastAPI se estiver aberto)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8001")
        .await
        .unwrap();
    
    tracing::info!("🚀 Core Listening on {}", listener.local_addr().unwrap());
    
    // Inicia o Servidor Nativo
    axum::serve(listener, app).await.unwrap();
}
