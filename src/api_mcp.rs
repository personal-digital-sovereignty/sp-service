#![allow(clippy::collapsible_if)]

use axum::{
    extract::{Query, State},
    response::{IntoResponse, sse::{Event, Sse}},
    Json,
};
use futures_util::stream::Stream;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::AppState;

lazy_static! {
    /// Active MCP Sessions: maps a Session ID to an MPSC Sender that pushes JSON-RPC strings to the SSE stream.
    pub static ref MCP_SESSIONS: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<String>>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Deserialize)]
pub struct McpMessageQuery {
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

/// Endpoint 1: GET /v1/mcp/sse
/// Estabelece o túnel MCP Server-Sent Events prescrito pela Anthropic.
/// O primeiro evento emitido declara a URL de Endpoint (POST) para aceitar os requests.
pub async fn mcp_sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let session_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    
    // Armazena no state global de Sessões MCP
    {
        let mut sessions = MCP_SESSIONS.write().await;
        sessions.insert(session_id.clone(), tx);
    }

    let sid = session_id.clone();
    
    // Devolve o túnel
    let stream = async_stream::stream! {
        // Handshake obrigatório da Spec MCP
        let endpoint_url = format!("/v1/mcp/message?session_id={}", sid);
        yield Ok(Event::default().event("endpoint").data(&endpoint_url));

        // Aguarda os processamentos despachados na rota POST
        while let Some(msg) = rx.recv().await {
            yield Ok(Event::default().event("message").data(msg));
        }

        // Cleanup se o canal fechar
        let mut sessions = MCP_SESSIONS.write().await;
        sessions.remove(&sid);
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}

/// Endpoint 2: POST /v1/mcp/message
/// Recebe os comandos RPC vindos do Client MCP (ex: Claude Desktop) e roteia para as `Tools` no Rust.
pub async fn mcp_message_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<McpMessageQuery>,
    Json(payload): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let session_id = query.session_id;

    let tx = {
        let sessions = MCP_SESSIONS.read().await;
        sessions.get(&session_id).cloned()
    };

    if let Some(sender) = tx {
        // Processa o Método RPC
        let is_notification = payload.id.is_none();
        let response_payload = handle_rpc_method(&state, payload).await;
        
        // Dispara de volta pela SSE (Event: message) - Apenas se não for Notificação
        if !is_notification {
            if let Ok(json_str) = serde_json::to_string(&response_payload) {
                let _ = sender.send(json_str);
            }
        }
        
        (axum::http::StatusCode::ACCEPTED, "Accepted").into_response()
    } else {
        (axum::http::StatusCode::BAD_REQUEST, "MCP Session Expired or Invalid").into_response()
    }
}

/// Processador de Métodos do Model Context Protocol
async fn handle_rpc_method(state: &Arc<AppState>, req: JsonRpcRequest) -> JsonRpcResponse {
    let req_id = req.id.clone().unwrap_or(serde_json::json!(null));

    // Inicialização do Protocolo
    if req.method == "initialize" {
        return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req_id,
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "Sovereign-Core-MCP",
                    "version": "0.8.4"
                }
            })),
            error: None,
        };
    }

    if req.method == "notifications/initialized" {
        return JsonRpcResponse { jsonrpc: "2.0".to_string(), id: req_id, result: Some(serde_json::json!({})), error: None };
    }

    // Listagem de Capabilities (Tools)
    if req.method == "tools/list" {
        let raw_tools = crate::mcp::get_mcp_tools();
        // Converte as tools padrão OpenAI que usamos internamente para o standard do MCP JSON-RPC
        let mut mcp_tools = Vec::new();
        for t in raw_tools {
            if let Some(func) = t.get("function") {
                mcp_tools.push(serde_json::json!({
                    "name": func.get("name"),
                    "description": func.get("description"),
                    "inputSchema": func.get("parameters")
                }));
            }
        }
        
        return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req_id,
            result: Some(serde_json::json!({ "tools": mcp_tools })),
            error: None,
        };
    }

    // Execução Dinâmica de Ferramentas
    if req.method == "tools/call" {
        if let Some(params) = req.params {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
            
            // Invoca a Sandbox do Sovereign Core
            let result_str = crate::mcp::execute_mcp_tool(state, name, &arguments).await;
            
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: Some(serde_json::json!({
                    "content": [
                        { "type": "text", "text": result_str }
                    ]
                })),
                error: None,
            };
        }
    }

    // Ping e Respostas não suportadas
    if req.method == "ping" {
        return JsonRpcResponse { jsonrpc: "2.0".to_string(), id: req_id, result: Some(serde_json::json!({})), error: None };
    }

    // Método Desconhecido
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: req_id,
        result: None,
        error: Some(serde_json::json!({
            "code": -32601,
            "message": format!("Method {} not supported by Sovereign MCP Server.", req.method)
        })),
    }
}
