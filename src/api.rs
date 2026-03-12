use axum::{
    extract::{Json, State},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
};
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::models::{
    OpenAIChatChunkChoice, OpenAIChatChunkDelta, OpenAIChatChunkResponse, OpenAIChatRequest,
};
use crate::AppState;

/// O Primeiro Controlador Cíbrido: Recebendo os Pensamentos do VS Code.
pub async fn chat_completions_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<OpenAIChatRequest>,
) -> Response {
    
    // Fallback/Extrator: Se 'stream' não vier especificado, assumimos True em respeito aos IDs nativos
    let is_stream = payload.stream.unwrap_or(true);
    let requested_model = payload.model.clone();
    
    info!("🔥 [Sovereign Core] Interceptando requisição OpenCode para o modelo: [{}] | Streaming: {}", requested_model, is_stream);

    // Broadcast Log (Cíbrido Live)
    let _ = state.log_sender.send(crate::models::LogEntry {
        timestamp: "".to_string(), // O Frontend popula no JS puro
        level: "agent".to_string(),
        message: format!("The Nurse acordou (Requisição de Inferência OpenCode para {})", requested_model),
    });

    // O Roteamento de Conversão (OpenAI -> Ollama)
    // 1. Transpilar Nomes de Modelos Proprietários para Modelos Locais
    let ollama_model = if requested_model.to_lowercase().contains("gpt") {
        "qwen2.5:3b".to_string() // Hardcode forçado do modelo cognitivo soberano
    } else {
        requested_model.clone()
    };

    // Extraindo o Prompt do User p/ Persistir
    let human_prompt = payload.messages.last()
        .map(|msg| match &msg.content {
            Some(crate::models::MessageContent::Text(t)) => t.clone(),
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
        })
        .unwrap_or_else(|| "Interação O.S".to_string());

    let active_session_id = crate::api_chat::get_or_create_session(&state.db, payload.session_id, &human_prompt).await;
    
    // Grava no Banco a pergunta Humana
    crate::api_chat::save_message(&state.db, active_session_id, "user", &human_prompt).await;

    // 2. Transcrever Mensagens Complexas (Multimodal/Arrays) para Strict Strings + Injeção de RAG Nativo
    let mut purified_messages: Vec<Value> = Vec::new();

    // Injeta o Contexto Físico do Usuário (Se o Vault conter arquivos válidos)
    if let Some(rag_cortex) = crate::rag::build_rag_context_message(&state.vault_path) {
        tracing::debug!("📚 RAG Context successfully injected into Prompt.");
        purified_messages.push(rag_cortex);
    }

    // Injeta as Mensagens da própria TUI (Código e Prompts)
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

        json!({
            "role": msg.role,
            "content": content_str
        })
    }));

    // 3. Empacotar para o Servidor Local com Controle Rigoroso de VRAM (Sovereign Enterprise - B2B)
    let mut ollama_payload = json!({
        "model": ollama_model,
        "messages": purified_messages,
        "stream": true,
        "keep_alive": -1, // Previne Cold Boots brutais da The Mom em Rotações
        "options": {
            "num_keep": 4, // Forçar Lock do System Prompt na VRAM
            "num_ctx": 16384 // Teto Estendido Seguro p/ Workflows Agentic Cíbridos
        }
    });

    // Injeção de Tools Requisitadas pelo Frontend (Vercel AI SDK JSON Schema)
    if let Some(tools) = payload.tools {
        ollama_payload["tools"] = json!(tools);
    }
    if let Some(tool_choice) = payload.tool_choice {
        ollama_payload["tool_choice"] = tool_choice;
    }

    let res = match state
        .http_client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&ollama_payload)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("🚨 Falha FATAL ao encontrar o motor LLM: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Motor Cognitive Air-Gapped não está respondendo na porta 11434.",
            ).into_response();
        }
    };

    if !res.status().is_success() {
        error!("❌ Ollama recusou a requisição HTTP. Status: {}", res.status());
        return (StatusCode::BAD_GATEWAY, "Erro no gateway interno LLM.").into_response();
    }

    // Criamos o Túnel de Transmissão contínua em Rust
    // Variáveis locais puras para contabilização na Closure do Stream
    let tracking_telemetry = state.telemetry.clone();
    let tracking_db = state.db.clone();
    let tracking_session = active_session_id;
    let tracking_model = ollama_model.clone();
    let start_time = std::time::Instant::now();
    let mut session_tokens = 0;
    let mut accumulator = String::new(); // Memory Builder da Resposta do Agente

    // Extraímos os Bytes Chunk a Chunk e mapeamos pro formato OpenAI SSE:
    let stream = res.bytes_stream().map(move |result| {
        match result {
            Ok(bytes) => {
                // Tenta transformar os bytes em string (pode vir linha cortada)
                if let Ok(chunk_str) = String::from_utf8(bytes.to_vec()) {
                    // Pra cada linha (Event), tentamos fazer parse se for um JSON Ollama
                    for line in chunk_str.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        if let Ok(ollama_resp) = serde_json::from_str::<Value>(line) {
                            if let Some(msg_obj) = ollama_resp.get("message") {
                                let mut has_content_or_tools = false;
                                let mut extracted_content = None;
                                let mut extracted_tool_calls: Option<Vec<crate::models::ChunkToolCall>> = None;

                                if let Some(content) = msg_obj.get("content").and_then(|c| c.as_str()) {
                                    if !content.is_empty() {
                                        session_tokens += 1;
                                        accumulator.push_str(content);
                                        extracted_content = Some(content.to_string());
                                        has_content_or_tools = true;
                                    }
                                }

                                if let Some(tool_calls_arr) = msg_obj.get("tool_calls").and_then(|tc| tc.as_array()) {
                                    let mut tcs = Vec::new();
                                    for (i, tc) in tool_calls_arr.iter().enumerate() {
                                        let mut new_tc = crate::models::ChunkToolCall {
                                            index: Some(i as i32),
                                            id: Some(format!("call_{}", uuid::Uuid::new_v4().to_string().replace("-", "").chars().take(8).collect::<String>())),
                                            r#type: Some("function".to_string()),
                                            function: None,
                                        };
                                        if let Some(func) = tc.get("function") {
                                            let name = func.get("name").and_then(|n| n.as_str()).map(|n| n.to_string());
                                            let args = func.get("arguments").map(|a| {
                                                if a.is_string() {
                                                    a.as_str().unwrap().to_string()
                                                } else {
                                                    serde_json::to_string(a).unwrap_or_default()
                                                }
                                            });
                                            new_tc.function = Some(crate::models::ChunkFunctionCall {
                                                name,
                                                arguments: args,
                                            });
                                        }
                                        tcs.push(new_tc);
                                    }
                                    if !tcs.is_empty() {
                                        extracted_tool_calls = Some(tcs);
                                        has_content_or_tools = true;
                                    }
                                }

                                if has_content_or_tools {
                                    let chunk_response = OpenAIChatChunkResponse {
                                        id: format!("chatcmpl-{}", uuid::Uuid::new_v4().to_string().replace("-", "").chars().take(12).collect::<String>()),
                                        object: "chat.completion.chunk".to_string(),
                                        created: 1234567890,
                                        model: requested_model.clone(),
                                        choices: vec![OpenAIChatChunkChoice {
                                            index: 0,
                                            delta: OpenAIChatChunkDelta {
                                                role: Some("assistant".to_string()),
                                                content: extracted_content,
                                                tool_calls: extracted_tool_calls,
                                            },
                                            finish_reason: None,
                                        }],
                                    };
                                    if let Ok(json_str) = serde_json::to_string(&chunk_response) {
                                        return Ok::<Event, Infallible>(Event::default().data(json_str));
                                    }
                                }
                            }
                            
                            // Tratar Evento de Fim de Transmissão do Ollama
                            // (Ollama envia "done": true no último pacote, com as estatísticas embutidas)
                            if let Some(done) = ollama_resp.get("done").and_then(|d| d.as_bool()) {
                                if done {
                                    // Bater na payload absoluta "eval_count" e "prompt_eval_count" do Ollama final JSON
                                    let llm_gen_tokens = ollama_resp.get("eval_count").and_then(|e| e.as_u64()).unwrap_or(session_tokens as u64) as usize;
                                    let llm_prompt_tokens = ollama_resp.get("prompt_eval_count").and_then(|e| e.as_u64()).unwrap_or(0) as usize;
                                    let total_real_tokens = llm_gen_tokens + llm_prompt_tokens;
                                    
                                    // 🚩 Observabilidade: Fim de Interação -> Gravando Métricas Cíbridas!
                                    let duration = start_time.elapsed().as_millis();
                                    if let Ok(mut t) = tracking_telemetry.write() {
                                        t.record_session(total_real_tokens, duration, &tracking_model);
                                    }

                                    // 🗄️ Imortalidade de Diálogo: Insere via Spawn para não bloquear o Axum Stream
                                    let final_text = accumulator.clone();
                                    let db_clone = tracking_db.clone();
                                    tokio::spawn(async move {
                                        crate::api_chat::save_message(&db_clone, tracking_session, "assistant", &final_text).await;
                                    });

                                   let finish_response = OpenAIChatChunkResponse {
                                        id: "chatcmpl-end".to_string(),
                                        object: "chat.completion.chunk".to_string(),
                                        created: 1234567890,
                                        model: requested_model.clone(),
                                        choices: vec![OpenAIChatChunkChoice {
                                            index: 0,
                                            delta: OpenAIChatChunkDelta {
                                                role: None,
                                                content: None,
                                                tool_calls: None,
                                            },
                                            finish_reason: Some("stop".to_string()),
                                        }],
                                    };
                                    if let Ok(json_str) = serde_json::to_string(&finish_response) {
                                        return Ok::<Event, Infallible>(Event::default().data(json_str));
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Keep-alive/vazios
                Ok::<Event, Infallible>(Event::default())
            }
            Err(e) => {
                error!("Erro mapeando os bytes de inferência da porta Ollama: {}", e);
                Ok::<Event, Infallible>(Event::default())
            }
        }
    });

    // Envolve a Stream num responder SSE do Axum e devolve o header Keep-Alive.
    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new())
        .into_response()
}

/// A Ponte UI-Core: Endpoint do Front-end Vue.js requisitando a Pressão da Rede
pub async fn telemetry_snapshot_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let snapshot = match state.telemetry.read() {
        Ok(t) => t.get_snapshot(),
        Err(_) => crate::telemetry::TelemetrySnapshot {
            total_tokens: 0,
            avg_tps: 0.0,
            estimated_cost: 0.0,
        },
    };
    
    // Devolve formatado igualzinho ao Node Python antigo pra Vue absorver sem refactor!
    Json(json!({
        "total_tokens": snapshot.total_tokens,
        "avg_tps": snapshot.avg_tps,
        "estimated_cost": snapshot.estimated_cost,
        "hardware": {
            "cpu": 0.0, // Preenchidos mockados ou simulados no JS (ou Rust Sysinfo futuro)
            "ram": 0.0,
            "io": 0.0
        }
    }))
}

/// O Canal Cíbrido Ao Vivo: Rota SSE despachando The Sentinel e Agent Logs
pub async fn realtime_logs_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.log_sender.subscribe();
    
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|res| async move {
        match res {
            Ok(log) => {
                if let Ok(json) = serde_json::to_string(&log) {
                    Some(Ok::<Event, Infallible>(Event::default().data(json)))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}

/// Stream Cíbrido SSE para o Sensus Sync Engine (RAG Pipeline Ocular)
pub async fn rag_sync_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.sync_sender.subscribe();
    
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|res| async move {
        match res {
            Ok(job) => {
                if let Ok(json) = serde_json::to_string(&job) {
                    Some(Ok::<Event, Infallible>(Event::default().data(json)))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}
