use axum::{ extract::{Json, State}, response::{ sse::{Event, Sse}, IntoResponse, Response, }, }; use futures_util::StreamExt;  use serde_json::{json, Value}; use std::convert::Infallible; use std::sync::Arc; use tracing::{error, info};

use crate::models::{ OpenAIChatChunkChoice, OpenAIChatChunkDelta, OpenAIChatChunkResponse, OpenAIChatRequest, }; use crate::AppState;

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
    let mut resolved_model = "qwen2.5:3b".to_string(); // Fallback de segurança
    if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'system_settings'").fetch_optional(&state.db).await {
        let val: String = sqlx::Row::get(&row, "value_json");
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
            if let Some(model_str) = parsed.get("llm_model").and_then(|v| v.as_str()) {
                if !model_str.is_empty() {
                    resolved_model = model_str.to_string();
                }
            }
        }
    }
    tracing::info!("🔄 Proxy OpenCode/Desktop enviou {}. Remapeando para o modelo local SQLite: {}", requested_model, resolved_model);
    resolved_model
} else {
    requested_model.clone()
};

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

// ===== THE NURSE (WEB & SYS AGENTIC BYPASS) =====
let mut web_context = String::new();
let mut sys_context = String::new();

let is_web = human_prompt.to_lowercase().starts_with("/web");
let is_sys = human_prompt.to_lowercase().starts_with("/sys");

if is_web {
    let query = human_prompt[4..].trim();
    info!("🌐 [The Nurse - Sovereign Core] Agentic Task detectada: /web -> Buscando '{}' na World Wide Web nativamente...", query);
    
    let ddg_url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Sovereign/1.0")
        .build()
        .unwrap_or_default();
        
    if let Ok(resp) = client.get(&ddg_url).send().await {
        if let Ok(html) = resp.text().await {
            let document = scraper::Html::parse_document(&html);
            let result_sel = scraper::Selector::parse(".result").unwrap();
            let title_sel = scraper::Selector::parse(".result__title").unwrap();
            let link_sel = scraper::Selector::parse(".result__url").unwrap();
            let snippet_sel = scraper::Selector::parse(".result__snippet").unwrap();
            
            let mut results = Vec::new();
            for node in document.select(&result_sel).take(5) {
                let title = node.select(&title_sel).next().map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
                let link = node.select(&link_sel).next().map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
                let snippet = node.select(&snippet_sel).next().map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
                
                if !title.is_empty() && !snippet.is_empty() {
                    results.push(format!("- **{}** ({}): {}", title, link, snippet));
                }
            }
            
            if !results.is_empty() {
                web_context = format!("INSTRUÇÃO SISTÊMICA (THE NURSE): O usuário solicitou uma pesquisa Web em tempo real. Você está operando como proxy. Seguem os últimos resultados mais quentes do motor de busca sobre o assunto:\n\n{}\n\nRESPONDA À PERGUNTA INICIAL DO USUÁRIO BASEANDO-SE EXCLUSIVAMENTE NOS FATOS ACIMA. FOQUE NOS FATOS.", results.join("\n"));
                info!("✅ [The Nurse] Sucesso! {} resultados extraídos da Web e injetados no Pipeline RAG.", results.len());
            } else {
                web_context = "A busca web falhou em extrair dados dos seletores DOM ou foi bloqueada pelo firewall do motor de busca.".to_string();
            }
        }
    } else {
        web_context = "Timeout de Rede: Não foi possível alcançar os nós de busca da Web externa.".to_string();
    }
} else if is_sys {
    let query = human_prompt[4..].trim();
    info!("⚙️ [The Nurse] Agentic Task detectada: /sys -> Analisando '{}'", query);
    sys_context = format!("INSTRUÇÃO SISTÊMICA (THE NURSE): O usuário solicitou análise profunda sobre a arquitetura 'Sovereign Pair'. Somos um sistema Cíbrido. Usamos Rust (The Nurse/Axum), Vue 3 + Tailwind (UI), LLMs Locais (Ollama), Python API. Foque em responder a seguinte dúvida de Engenharia: '{}'", query);
}
// =========================================================

let active_session_id = crate::api_chat::get_or_create_session(&state.db, payload.session_id, &human_prompt).await;

// Grava no Banco a pergunta Humana
crate::api_chat::save_message(&state.db, active_session_id, "user", &human_prompt).await;

// 2. Transcrever Mensagens Complexas (Multimodal/Arrays) para Strict Strings + Injeção de RAG Nativo
let mut purified_messages: Vec<Value> = Vec::new();

// Injeta o Contexto Web ou Sys gerados nativamente pelo Rust The Nurse (Agent Tasks)
if !web_context.is_empty() {
    purified_messages.push(json!({
        "role": "system",
        "content": web_context
    }));
}

if !sys_context.is_empty() {
    purified_messages.push(json!({
        "role": "system",
        "content": sys_context
    }));
}

// Injeta a Orquestração do ReWOO (Reasoning Without Observation)
let rewoo_observations = crate::rewoo::execute_rewoo_plan(&human_prompt, &state.vault_path).await;
tracing::debug!("🧠 ReWOO Workflow Executed. Injecting compiled DAG observations.");
purified_messages.push(json!({
    "role": "system",
    "content": rewoo_observations
}));

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
    "options": {
        "num_keep": 4, // Forçar Lock do System Prompt na VRAM
        "num_ctx": 8192 // Desidratação do Nurse (16GB RAM overhead fix resolvido pra RPI/OracleA1)
    }
});

// Injeção de Tools Requisitadas pelo Frontend (Vercel AI SDK JSON Schema)
if let Some(tools) = payload.tools {
    ollama_payload["tools"] = json!(tools);
}
if let Some(tool_choice) = payload.tool_choice {
    ollama_payload["tool_choice"] = tool_choice;
}

// Resgate Masterplan da Tabela de Configurações
let env_ollama_url = std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
let mut ollama_base_url = env_ollama_url.trim_end_matches('/').to_string();
let mut is_custom_cluster = false;

if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'ollama_clusters'").fetch_optional(&state.db).await {
    let val: String = sqlx::Row::get(&row, "value_json");
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
        let active_id = parsed.get("active_cluster_id").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(clusters) = parsed.get("clusters").and_then(|v| v.as_array()) {
            for c in clusters {
                if c.get("id").and_then(|v| v.as_str()).unwrap_or("") == active_id
                    && let Some(url) = c.get("url").and_then(|v| v.as_str()) {
                        let cluster_url = url.trim_end_matches('/').to_string();
                        // Se a URL na UI não for nula nem vazia, adotamos:
                        if !cluster_url.is_empty() {
                            ollama_base_url = cluster_url;
                            is_custom_cluster = ollama_base_url != "http://localhost:11434" && ollama_base_url != "http://127.0.0.1:11434" && ollama_base_url != "http://host.docker.internal:11434";
                        }
                    }
            }
        }
    }
}

// Se estiver rodando nativo na máquina local (não via docker) e tentar invocar o host.docker.internal do .env legados, corrige pra localhost 
// Só ignora se for um custom cluster selecionado de fato na UI (tipo um IP da Oracle)
if !is_custom_cluster && ollama_base_url == "http://host.docker.internal:11434" {
    // Tenta checar se o docker internal está resolvendo, se a request HTTP estiver quebrando por DNS, caimos de volta
    // Simplificado: sempre usar 127.0.0.1 se for host native O.S
    if std::env::var("SOVEREIGN_RUN_ENV").unwrap_or_default() == "native" {
         ollama_base_url = "http://127.0.0.1:11434".to_string();
    }
}

let endpoint = format!("{}/api/chat", ollama_base_url);

let res = match state
    .http_client
    .post(&endpoint)
    .json(&ollama_payload)
    .send()
    .await
{
    Ok(r) if r.status().is_success() => r,
    Ok(r) => {
        let status = r.status();
        let err_body = r.text().await.unwrap_or_default();
        error!("❌ Ollama recusou a requisição HTTP. Status: {} - Body: {}", status, err_body);
        
        let err_msg = if status == reqwest::StatusCode::NOT_FOUND && err_body.contains("not found") {
            format!("*(Conexão Remota)* 🚨 **Falha: Modelo Ausente no Nó Remoto**\nO modelo `{}` não está instalado no seu nó remoto (`{}`).\n\n**Solução:** Vá em 'Gerenciar Nós' nas configurações e execute o Download/Pull deste modelo para que nossos agentes possam usá-lo.", ollama_model, endpoint)
        } else {
            format!("*(Protocolo de Fallback)* 🚨 Falha no nó LLM configurado ({}). Status HTTP: {} - Body: {}", endpoint, status, err_body)
        };
        
        let err_chunk = crate::models::OpenAIChatChunkResponse {
            id: format!("chatcmpl-err-{}", uuid::Uuid::new_v4()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: ollama_model.clone(),
            choices: vec![crate::models::OpenAIChatChunkChoice {
                index: 0,
                delta: crate::models::OpenAIChatChunkDelta {
                    role: Some("assistant".to_string()),
                    content: Some(err_msg),
                    tool_calls: None,
                },
                finish_reason: Some("error".to_string()),
            }],
            usage: None,
        };
        let stream = futures_util::stream::iter(vec![
            Ok::<Event, Infallible>(Event::default().data(serde_json::to_string(&err_chunk).unwrap_or_default())),
            Ok::<Event, Infallible>(Event::default().data("[DONE]")),
        ]);
        return Sse::new(stream).into_response();
    },
    Err(e) => {
        error!("🚨 Falha FATAL ao encontrar o motor LLM: {}", e);
        let err_msg = if is_custom_cluster {
            format!("*(Sovereign Core)* 🚨 **Severidade Máxima: A1 Oracle Offline**\nO cluster remoto mapeado em `{}` não respondeu. Certifique-se de que a VM está ligada e acessível na rede.\n\nDetalhe do Gateway: `{}`", endpoint, e)
        } else {
            format!("*(The Nurse Local)* ⚠️ Serviço de IA Local (Ollama) inacessível na porta 11434. O daemon está rodando?\n\nErro: `{}`", e)
        };

        let err_chunk = crate::models::OpenAIChatChunkResponse {
            id: format!("chatcmpl-err-{}", uuid::Uuid::new_v4()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: ollama_model.clone(),
            choices: vec![crate::models::OpenAIChatChunkChoice {
                index: 0,
                delta: crate::models::OpenAIChatChunkDelta {
                    role: Some("assistant".to_string()),
                    content: Some(err_msg),
                    tool_calls: None,
                },
                finish_reason: Some("error".to_string()),
            }],
            usage: None,
        };
        let stream = futures_util::stream::iter(vec![
            Ok::<Event, Infallible>(Event::default().data(serde_json::to_string(&err_chunk).unwrap_or_default())),
            Ok::<Event, Infallible>(Event::default().data("[DONE]")),
        ]);
        return Sse::new(stream).into_response();
    }
};

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

                            if let Some(content) = msg_obj.get("content").and_then(|c| c.as_str())
                                && !content.is_empty() {
                                    session_tokens += 1;
                                    accumulator.push_str(content);
                                    extracted_content = Some(content.to_string());
                                    has_content_or_tools = true;
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
                                    usage: None,
                                };
                                if let Ok(json_str) = serde_json::to_string(&chunk_response) {
                                    return Ok::<Event, Infallible>(Event::default().data(json_str));
                                }
                            }
                        }
                        
                        // Tratar Evento de Fim de Transmissão do Ollama
                        // (Ollama envia "done": true no último pacote, com as estatísticas embutidas)
                        if let Some(done) = ollama_resp.get("done").and_then(|d| d.as_bool())
                            && done {
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
                                    usage: Some(crate::models::OpenAITokenUsage {
                                        prompt_tokens: llm_prompt_tokens as i32,
                                        completion_tokens: llm_gen_tokens as i32,
                                        total_tokens: total_real_tokens as i32,
                                    }),
                                };
                                if let Ok(json_str) = serde_json::to_string(&finish_response) {
                                    return Ok::<Event, Infallible>(Event::default().data(json_str));
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

#[derive(serde::Serialize, sqlx::FromRow)]
struct QuarantineItem {
    id: i64,
    file_path: String,
    reason: String,
}

pub async fn telemetry_snapshot_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let snapshot = match state.telemetry.read() {
        Ok(t) => t.get_snapshot(),
        Err(_) => crate::telemetry::TelemetrySnapshot {
            total_tokens: 0,
            avg_tps: 0.0,
            estimated_cost: 0.0,
        },
    };

    let quarantine_count = sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM quarantine_logs WHERE status = 'QUARANTINED'")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let quarantined_files = sqlx::query_as::<_, QuarantineItem>(
        "SELECT id, file_path, reason FROM quarantine_logs WHERE status = 'QUARANTINED' ORDER BY created_at DESC LIMIT 5"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let pending_tasks = sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM tasks WHERE status != 'completed'")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let total_tasks = sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM tasks")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let progress = if total_tasks > 0 {
        let completed = total_tasks - pending_tasks;
        ((completed as f64 / total_tasks as f64) * 100.0) as i32
    } else {
        0
    };

    // Devolve formatado igualzinho ao Node Python antigo pra Vue absorver sem refactor!
    Json(serde_json::json!({
        "total_tokens": snapshot.total_tokens,
        "avg_tps": snapshot.avg_tps,
        "estimated_cost": snapshot.estimated_cost,
        "active_models": 1, 
        "hardware": {
            "cpu": 0.0, // Preenchidos mockados ou simulados no JS (ou Rust Sysinfo futuro)
            "ram": 0.0,
            "io": 0.0
        },
        "cronos": {
            "gaps": quarantine_count,
            "gaps_list": quarantined_files,
            "tasks_today": pending_tasks,
            "progress": progress
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
#[allow(dead_code)]
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