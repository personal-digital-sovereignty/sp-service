use axum::{
    extract::{Json, State},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
};
use futures_util::{stream, StreamExt};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::Arc;
use tracing::{error, info};

use crate::models::{OpenAIChatRequest};
use crate::AppState;

/// O Segundo Controlador Cíbrido: Mocking do Protocolo Proprietário Vercel AI SDK Realtime.
/// As extensões corporativas impõem a serialização de Handshakes rígidos do Runtime TS (ZOD Validator).
pub async fn realtime_responses_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<OpenAIChatRequest>,
) -> Response {
    let requested_model = payload.model.clone();
    info!("🔥 [Sovereign Core] Realtime Vercel Hack para o modelo: [{}]", requested_model);

    // 1. Transpilação Dinâmica
    let mut db_model_fallback = "llama3.2".to_string();
    if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'system_settings'").fetch_optional(&state.db).await {
        let val: String = sqlx::Row::get(&row, "value_json");
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val)
            && let Some(m) = parsed.get("llm_model").and_then(|v| v.as_str()) {
                db_model_fallback = m.to_string();
            }
    }

    let ollama_model = if requested_model.to_lowercase().contains("gpt") {
        db_model_fallback
    } else {
        requested_model.clone()
    };

    // 2. Transcrição Multimodal Hostil para Texto Limpo + Injeção RAG Nativa
    let mut purified_messages: Vec<Value> = Vec::new();

    if let Some(rag_cortex) = crate::rag::build_rag_context_message(&state.vault_path) {
        purified_messages.push(rag_cortex);
    }

    purified_messages.extend(payload.messages.into_iter().map(|msg| {
        let content_str = match msg.content {
            Some(crate::models::MessageContent::Text(t)) => t,
            Some(crate::models::MessageContent::Multimodal(parts)) => {
                let mut full = String::new();
                for part in parts {
                    if let Some(txt) = part.get("text").and_then(|t| t.as_str()) {
                        full.push_str(txt);
                    }
                }
                full
            },
            None => "".to_string(),
        };
        json!({"role": msg.role, "content": content_str})
    }));

    // 3. Montar Payload do Ollama
    let ollama_payload = json!({
        "model": ollama_model,
        "messages": purified_messages,
        "stream": true
    });

    let res = match state.http_client.post("http://127.0.0.1:11434/api/chat").json(&ollama_payload).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("🚨 Motor Cognitivo Offline: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Motor Offline").into_response()
        }
    };

    // Protocolo Zod: Handshakes Iniciais (Exigidos antes do primeiro Delta de Texto)
    let initial_events = stream::iter(vec![
        Ok::<Event, Infallible>(Event::default().data(
            serde_json::to_string(&json!({"type": "response.created", "response": {}})).unwrap(),
        )),
        Ok::<Event, Infallible>(Event::default().data(
            serde_json::to_string(&json!({"type": "response.output_item.added", "output_index": 0, "item": {"id": "msg_cibrido", "type": "message", "role": "assistant"}})).unwrap(),
        )),
    ]);

    // O corpo principal do Stream (Tokens sendo vomitados)
    let tail_stream = res.bytes_stream().map(move |result| {
        match result {
            Ok(bytes) => {
                if let Ok(chunk_str) = String::from_utf8(bytes.to_vec()) {
                    for line in chunk_str.lines() {
                        let line = line.trim();
                        if line.is_empty() { continue; }

                        if let Ok(ollama_resp) = serde_json::from_str::<Value>(line) {
                            if let Some(msg_obj) = ollama_resp.get("message")
                                && let Some(content) = msg_obj.get("content").and_then(|c| c.as_str()) {
                                    // Zod Array Mutation ("response.output_text.delta")
                                    let json_str = serde_json::to_string(&json!({
                                        "type": "response.output_text.delta",
                                        "item_id": "msg_cibrido",
                                        "output_index": 0,
                                        "delta": content
                                    })).unwrap_or_default();
                                    return Ok::<Event, Infallible>(Event::default().data(json_str));
                                }
                            
                            // Sinalizadores de Fim
                            if let Some(done) = ollama_resp.get("done").and_then(|d| d.as_bool())
                                && done {
                                    // Zod Closure Unions
                                    let done_event = serde_json::to_string(&json!({
                                        "type": "response.output_item.done",
                                        "output_index": 0,
                                        "item": {"id": "msg_cibrido", "type": "message", "role": "assistant"}
                                    })).unwrap_or_default();

                                    let completed_event = serde_json::to_string(&json!({
                                        "type": "response.completed",
                                        "response": {}
                                    })).unwrap_or_default();

                                    // Retorna um evento combo (SSE Data Multiline, Vercel Parse tolerará se os unirmos ou, idealmente, separar)
                                    // A maneira correta do SSE é enviar Multi-Data no mesmo bloco se for um evento, 
                                    // ou um Bloco com a primeira, e o resto vai ignorar se for solto. Mas string pura de múltiplos datas funciona no browser EventSource:
                                    let magic_combo = format!("{}\n\ndata: {}", done_event, completed_event);

                                    return Ok::<Event, Infallible>(Event::default().data(magic_combo));
                                }
                        }
                    }
                }
                Ok::<Event, Infallible>(Event::default())
            }
            Err(_) => Ok::<Event, Infallible>(Event::default())
        }
    });
    
    // Mesclamos os Handshakes Preliminares aos Deltas de Produção Contínua
    let final_stream = initial_events.chain(tail_stream);

    Sse::new(final_stream).keep_alive(axum::response::sse::KeepAlive::new()).into_response()
}
