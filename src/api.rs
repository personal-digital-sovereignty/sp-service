use axum::{ extract::{Json, State}, response::{ sse::{Event, Sse}, IntoResponse, Response, }, }; use futures_util::StreamExt;  use serde_json::{json, Value}; use std::convert::Infallible; use std::sync::Arc; use tracing::{error, info};

use crate::models::{ OpenAIChatChunkChoice, OpenAIChatChunkDelta, OpenAIChatChunkResponse, OpenAIChatRequest, }; use crate::AppState;
use std::path::Path;
use scraper::{Html, Selector};

// -------------------------------------------------------------
// Autonomous Multi-Scraper Helperc
// -------------------------------------------------------------
async fn scrape_engine(client: &reqwest::Client, name: &str, url: &str, jitter_base: u64) -> String {
    let jitter = jitter_base + (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_micros() % 2500) as u64;
    tokio::time::sleep(tokio::time::Duration::from_millis(jitter)).await;

    tracing::info!("🕵️ [The Nurse | Squad {}] Infiltrando (Jitter Biológico: {}ms)...", name, jitter);
    
    if let Ok(resp) = client.get(url).send().await {
        if resp.status().is_success() {
            if let Ok(html) = resp.text().await {
                let document = scraper::Html::parse_document(&html);
                if let Ok(text_sel) = scraper::Selector::parse("p, span, h1, h2, h3, h4, h5, h6, a, li, div.BNeawe, div.snippet") {
                    let mut texts = Vec::new();
                    for node in document.select(&text_sel) {
                        let t = node.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        // Ignore residual JS/CSS by rejecting heavily bracketed strings and short meaningless noise
                        if t.len() > 10 && !t.contains("function(") && !t.contains("window.") && t.chars().filter(|c| *c == '{').count() < 2 {
                            texts.push(t);
                        }
                    }
                    
                    let condensed = texts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
                    let truncated: String = condensed.chars().take(3500).collect();
                    
                    let lower = truncated.to_lowercase();
                    if truncated.len() > 150 && !lower.contains("recaptcha") && !lower.contains("are you a human") && !lower.contains("bot detection") && !lower.contains("verificação de segurança") {
                        tracing::info!("✅ [The Nurse | Squad {}] Extração Pura: {} bytes", name, truncated.len());
                        return format!("📌 FONTE DE DADOS [{}]:\n{}", name, truncated);
                    } else {
                        tracing::info!("⚠️ [The Nurse | Squad {}] Extrator detectou página de CAPTCHA/WAF! Abortando snippet.", name);
                    }
                }
            }
        } else {
            tracing::info!("🚫 [The Nurse | Squad {}] Bloqueado pelo WAF HTTP Status: {}", name, resp.status());
        }
    } else {
        tracing::info!("❌ [The Nurse | Squad {}] Timeout de Rede.", name);
    }
    String::new()
}

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

let mut sys_temperature: Option<f64> = None;
let mut sys_top_k: Option<i64> = None;
let mut global_system_prompt: Option<String> = None;
let mut resolved_model = requested_model.clone();

if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'system_settings'").fetch_optional(&state.db).await {
    let val: String = sqlx::Row::get(&row, "value_json");
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
        
        // --- Tri-Agent Sovereign Router ---
        let prompt_lower = human_prompt.to_lowercase();
        let target_key = if prompt_lower.starts_with("/web") || prompt_lower.starts_with("/sys") {
            "nurse_model"
        } else if prompt_lower.starts_with("/plan") || prompt_lower.starts_with("/code") || prompt_lower.starts_with("/test") {
            "coder_model"
        } else {
            "doctor_model" // Default Conversational Agent
        };

        if let Some(specific_model) = parsed.get(target_key).and_then(|v| v.as_str()) {
            if !specific_model.is_empty() { 
                resolved_model = specific_model.to_string(); 
                tracing::info!("🧠 [Sovereign Router] Roteando intenção '{}' para o Agente Dedicado: {}", target_key, resolved_model);
            }
        } else if requested_model.to_lowercase().contains("gpt") {
            // Fallback Legacy
            if let Some(model_str) = parsed.get("llm_model").and_then(|v| v.as_str()) {
                if !model_str.is_empty() { resolved_model = model_str.to_string(); }
            } else {
                resolved_model = "qwen2.5:3b".to_string(); // Fallback de segurança
            }
            tracing::info!("🔄 Proxy OpenCode/Desktop enviou {}. Remapeando via Router para: {}", requested_model, resolved_model);
        }
        
        if let Some(t) = parsed.get("temperature").and_then(|v| v.as_f64()) { sys_temperature = Some(t); }
        if let Some(k) = parsed.get("top_k").and_then(|v| v.as_i64()) { sys_top_k = Some(k); }
        if let Some(p) = parsed.get("system_prompt").and_then(|v| v.as_str()) {
            if !p.is_empty() { global_system_prompt = Some(p.to_string()); }
        }
    }
}
let ollama_model = resolved_model;

// ===== THE PLANNER (MACRO ORCHESTRATION BYPASS) =====
if human_prompt.to_lowercase().starts_with("/plan") {
    let query = human_prompt[5..].trim().to_string();
    info!("🧠 [Sovereign Core] Plan & Execute Task detectada: /plan -> Iniciando Macro Orquestração em Background...");
    
    let db_clone = state.db.clone();
    let log_tx_clone = state.log_sender.clone();
    let vault_clone = state.vault_path.clone();
    
    tokio::spawn(async move {
        crate::plan_execute::start_plan_and_execute(query, vault_clone, db_clone, log_tx_clone).await;
    });

    let msg = "🧭 **Plan & Execute (Macro-Orquestração) Iniciado!**\nSua tarefa foi inserida no Threadpool Assíncrono do Cíbrido. O Planner irá quebrar seu pedido em etapas menores, validará nativamente na formatação strict JSON, e o Executor cuidará de cada etapa seqüencialmente sem travar seu terminal. \n\n*Acompanhe o Plasma Widget ou Logs para ver a orquestração em andamento!*".to_string();

    let chunk = crate::models::OpenAIChatChunkResponse {
        id: format!("chatcmpl-plan-{}", uuid::Uuid::new_v4()),
        object: "chat.completion.chunk".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: requested_model.clone(),
        choices: vec![crate::models::OpenAIChatChunkChoice {
            index: 0,
            delta: crate::models::OpenAIChatChunkDelta {
                role: Some("assistant".to_string()),
                content: Some(msg),
                tool_calls: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: None,
    };
    let stream = futures_util::stream::iter(vec![
        Ok::<Event, Infallible>(Event::default().data(serde_json::to_string(&chunk).unwrap_or_default())),
        Ok::<Event, Infallible>(Event::default().data("[DONE]")),
    ]);
    return Sse::new(stream).into_response();
}

// ===== THE NURSE (WEB & SYS AGENTIC BYPASS) =====
let mut web_context = String::new();
let mut sys_context = String::new();

let is_web = human_prompt.to_lowercase().starts_with("/web");
let is_sys = human_prompt.to_lowercase().starts_with("/sys");

if is_web {
    let query = human_prompt[4..].trim();
    
    // Detecção Inteligente de URLs passadas diretamente no prompt
    let mut url_to_scrape = String::new();
    let mut user_question = query.to_string();
    for word in query.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            url_to_scrape = word.to_string();
            user_question = user_question.replace(word, "").trim().to_string();
            break;
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Sovereign/1.0")
        .build()
        .unwrap_or_default();

    if !url_to_scrape.is_empty() {
        let jitter = 1200 + (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_micros() % 3000) as u64;
        info!("🔗 [The Nurse] (Human Jitter: {}ms) Extração de URL Direta: Lendo DOM de {}", jitter, url_to_scrape);
        tokio::time::sleep(tokio::time::Duration::from_millis(jitter)).await;

        if let Ok(resp) = client.get(&url_to_scrape).send().await {
            if let Ok(html) = resp.text().await {
                let document = scraper::Html::parse_document(&html);
                let body_sel = scraper::Selector::parse("body").unwrap();
                let raw_text = document.select(&body_sel).next()
                    .map(|b| b.text().collect::<Vec<_>>().join(" "))
                    .unwrap_or_else(|| document.root_element().text().collect::<Vec<_>>().join(" "));
                
                let condensed = raw_text.split_whitespace().collect::<Vec<_>>().join(" ");
                let truncated: String = condensed.chars().take(15000).collect(); // 15K chars context
                
                if !truncated.is_empty() {
                    web_context = format!("INSTRUÇÃO SISTÊMICA (THE NURSE): O usuário anexou a URL ({}). A proxy extraiu o texto bruto do site:\n\n{}\n\nAGENTE, RESPONDA A ISSO: '{}' (Foque nos dados!).", url_to_scrape, truncated, user_question);
                    info!("✅ [The Nurse] Sucesso! {} bytes lidos da URL direta.", truncated.len());
                } else {
                    web_context = "A proxy alcançou o site, mas a extração DOM de texto falhou (talvez Client-Side Rendering pesado).".to_string();
                }
            }
        } else {
            web_context = "Gateway Timeout ou Firewall na URL especificada.".to_string();
        }
    } else {
        info!("🌐 [The Nurse] Agentic Task: /web -> Engatilhando Infiltração Simultânea nos 4 Motores de Busca...");
        
        let q_encoded = urlencoding::encode(query);
        let u_google = format!("https://www.google.com/search?q={}", q_encoded);
        let u_bing = format!("https://www.bing.com/search?q={}", q_encoded);
        let u_yahoo = format!("https://br.search.yahoo.com/search?p={}", q_encoded);
        let u_ddg = format!("https://html.duckduckgo.com/html/?q={}", q_encoded);

        // Dispara os 4 agentes simultaneamente na Memória
        let (res_google, res_bing, res_yahoo, res_ddg) = tokio::join!(
            scrape_engine(&client, "Google", &u_google, 500),
            scrape_engine(&client, "Bing", &u_bing, 1000),
            scrape_engine(&client, "Yahoo", &u_yahoo, 1500),
            scrape_engine(&client, "DuckDuckGo", &u_ddg, 2000)
        );

        let mut combined_data = String::new();
        if !res_google.is_empty() { combined_data.push_str(&res_google); combined_data.push_str("\n\n"); }
        if !res_bing.is_empty() { combined_data.push_str(&res_bing); combined_data.push_str("\n\n"); }
        if !res_yahoo.is_empty() { combined_data.push_str(&res_yahoo); combined_data.push_str("\n\n"); }
        if !res_ddg.is_empty() { combined_data.push_str(&res_ddg); combined_data.push_str("\n\n"); }

        if !combined_data.trim().is_empty() {
            let current_time = chrono::Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
            web_context = format!("INSTRUÇÃO SISTÊMICA (THE NURSE): Missão Quad-Scraper Finalizada.\n[RELOGIO BIOLOGICO DO SISTEMA CIBRIDO: {}]\n\nAbaixo estão os dados dos Motores de Busca (se o resultado citar 'Hoje', 'Amanhã' ou 'Next Days', você DEVE traduzir mentalmente essas palavras para a data exata usando nosso Relógio Biológico informado acima):\n\n{}\n\nAGENTE, RESPONDA À PERGUNTA SOLICITADA ABAIXO OBRIGATORIAMENTE EM PORTUGUÊS DO BRASIL. SINTETIZE A VERDADE COM BASE EM TODOS OS VETORES ACIMA:\n'{}'", current_time, combined_data, query);
            info!("🎯 [The Nurse] Missão Cumprida: Combinados {} bytes de puro conhecimento Cíbrido.", combined_data.len());
        } else {
            web_context = "A infiltração falhou em TODOS OS 4 SISTEMAS. Cloudflare/WAF bloqueou totalmente o cluster de Sovereign Node. Peça desculpas ao comandante e inferencie sobre os dados que tem.".to_string();
        }
    }
} else if is_sys {
    let query = human_prompt[4..].trim();
    info!("⚙️ [The Nurse] Agentic Task detectada: /sys -> Analisando '{}'", query);
    sys_context = format!("INSTRUÇÃO SISTÊMICA (THE NURSE): O usuário solicitou análise profunda sobre a arquitetura 'Sovereign Pair'. Somos um sistema Cíbrido puramente em Rust (The Nurse/Axum) e Svelte 5 + Tailwind na UI. Usamos LLMs Locais (Ollama) mapeados via SQL. Foque em responder a seguinte dúvida de Engenharia: '{}'", query);
}
// =========================================================

let active_session_id = crate::api_chat::get_or_create_session(&state.db, payload.session_id, &human_prompt).await;

// Grava no Banco a pergunta Humana
crate::api_chat::save_message(&state.db, active_session_id, "user", &human_prompt).await;

// 2. Transcrever Mensagens Complexas (Multimodal/Arrays) para Strict Strings + Injeção de RAG Nativo
let mut purified_messages: Vec<Value> = Vec::new();

// --- SOVEREIGN CONTEXT INJECTOR (RAG V2 - KANBAN) ---
let mut project_context = String::new();
if let Some(pid) = payload.project_id {
    if let Ok(Some(proj_info)) = sqlx::query("SELECT name, purpose FROM projects WHERE id = ?")
        .bind(&pid)
        .fetch_optional(&state.db)
        .await
    {
        let p_name: String = sqlx::Row::get(&proj_info, "name");
        let p_purpose: Option<String> = sqlx::Row::get(&proj_info, "purpose");
        
        project_context.push_str(&format!("INSTRUÇÃO SISTÊMICA MÁXIMA (SOVEREIGN PROJECT ASSISTANT): 🧠 O usuário está focando absolutamente no Projeto Kanban: '{}'.\n", p_name));
        if let Some(purp) = p_purpose {
            project_context.push_str(&format!("🎯 O PROPÓSITO raiz desse projeto é: '{}'. Seu comportamento deve orbitar este propósito.\n", purp));
        }

        if let Ok(tasks) = sqlx::query("SELECT title, status FROM tasks WHERE project_id = ? AND status != 'Done'")
            .bind(&pid)
            .fetch_all(&state.db)
            .await
        {
            if !tasks.is_empty() {
                project_context.push_str("\n📌 TAREFAS ATIVAS NO KANBAN:\n");
                for row in tasks {
                    let t_title: String = sqlx::Row::get(&row, "title");
                    let t_status: String = sqlx::Row::get(&row, "status");
                    project_context.push_str(&format!("- [{}] {}\n", t_status, t_title));
                }
            }
        }

        if let Ok(docs) = sqlx::query("SELECT file_path FROM project_documents WHERE project_id = ?")
            .bind(&pid)
            .fetch_all(&state.db)
            .await
        {
            if !docs.is_empty() {
                project_context.push_str("\n📚 DOCUMENTOS CÍBRIDOS VINCULADOS AO PROJETO (RAG NATIVO ABSOLUTO):\n");
                for row in docs {
                    let mut doc_path: String = sqlx::Row::get(&row, "file_path");
                    if !doc_path.starts_with('/') {
                        doc_path = format!("{}/{}", state.vault_path.display(), doc_path);
                    }
                    
                    if let Ok(content) = std::fs::read_to_string(&doc_path) {
                        let truncated: String = content.chars().take(6000).collect(); // 6K ch limit to spare VRAM
                        project_context.push_str(&format!("\n--- Arquivo: {} ---\n{}\n---\n", doc_path, truncated));
                    }
                }
            }
        }
    }
}

if !project_context.is_empty() {
    purified_messages.push(json!({
        "role": "system",
        "content": project_context
    }));
}
// --- FIM DO PROJECT CONTEXT ---


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
} else if let Some(global_prompt) = global_system_prompt {
    // Injeta a Persona Customizada definida na UI se não for uma Agentic Task Restrita
    purified_messages.push(json!({
        "role": "system",
        "content": global_prompt
    }));
}

// Injeta a Orquestração do ReWOO (Reasoning Without Observation)
let workspace_id = payload.workspace_id.clone().unwrap_or_else(|| "default".to_string());
let rewoo_observations = crate::rewoo::execute_rewoo_plan(&human_prompt, &workspace_id, &state.db).await;
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
let mut ollama_options = json!({
    "num_keep": 4, // Forçar Lock do System Prompt na VRAM
    "num_ctx": 8192 // Desidratação do Nurse (16GB RAM overhead fix resolvido pra RPI/OracleA1)
});

if let Some(t) = sys_temperature { ollama_options["temperature"] = json!(t); }
if let Some(k) = sys_top_k { ollama_options["top_k"] = json!(k); }

let mut ollama_payload = json!({
    "model": ollama_model,
    "messages": purified_messages,
    "stream": true,
    "options": ollama_options
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
        if is_custom_cluster {
            tracing::warn!("🔄 OCI/Mesh Node Offline ({}). Iniciando Autonomia de Fallback para Localhost (127.0.0.1:11434)...", e);
            let local_endpoint = "http://127.0.0.1:11434/api/chat";
            match state.http_client.post(local_endpoint).json(&ollama_payload).send().await {
                Ok(fallback_r) if fallback_r.status().is_success() => {
                    tracing::info!("✅ [Sovereign Core] Autonomia de Fallback ativada com sucesso. Servindo LLM Localmente!");
                    fallback_r
                },
                _ => {
                    error!("🚨 Falha FATAL no nó mestre e no nó escravo.");
                    let err_msg = format!("*(Sovereign Core)* 🚨 **Severidade Máxima: Abandono de Frota**\nO Cluster OCI Oracle não respondeu (`{}`) E o Fallback Autônomo Local (127.0.0.1:11434) também está offline! Impossível iniciar inferência.", e);
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
            }
        } else {
            error!("🚨 Falha FATAL ao encontrar o motor LLM Local: {}", e);
            let err_msg = format!("*(The Nurse Local)* ⚠️ Serviço de IA Local (Ollama) inacessível na porta 11434. O daemon está rodando?\n\nErro: `{}`", e);

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
let tracking_log_sender = state.log_sender.clone();

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
                                    id: format!("session_{}", tracking_session),
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
                                
                                let tps = if duration > 0 { (total_real_tokens as f64 / (duration as f64 / 1000.0)).round() } else { 0.0 };
                                let _ = tracking_log_sender.send(crate::models::LogEntry {
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    level: "system".to_string(),
                                    message: format!("⚡ Geração de Conhecimento: {} tokens a {} T/s [{}]", total_real_tokens, tps, tracking_model),
                                });

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

pub async fn telemetry_snapshot_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let snapshot = match state.telemetry.read() {
        Ok(t) => t.get_snapshot(),
        Err(_) => crate::telemetry::TelemetrySnapshot {
            total_tokens: 0,
            avg_tps: 0.0,
            estimated_cost: 0.0,
        },
    };

    // Quarantine is removed from backend; Mocking 0 for legacy UI
    let quarantine_count = 0;
    let quarantined_files: Vec<serde_json::Value> = vec![];

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
    let mut total_ram_gb = 24.0;
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        total_ram_gb = (kb / 1024.0 / 1024.0).round();
                    }
                }
                break;
            }
        }
    }

    // Devolve formatado para o Dashboard Svelte
    Json(serde_json::json!({
        "total_tokens": snapshot.total_tokens,
        "avg_tps": snapshot.avg_tps,
        "estimated_cost": snapshot.estimated_cost,
        "active_models": 1, 
        "hardware": {
            "cpu": 0.0, // Preenchidos mockados ou simulados no JS (ou Rust Sysinfo futuro)
            "ram": 0.0,
            "ram_total_gb": total_ram_gb,
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

#[derive(serde::Deserialize)]
pub struct FeedbackRequest {
    pub text: String,
    pub agent: String,
    pub thumbs_up: bool,
}

/// Registra Telemetria RLHF (Feedback Positivo/Negativo) dos Modelos Locais
pub async fn feedback_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedbackRequest>,
) -> impl IntoResponse {
    let result = sqlx::query("INSERT INTO rlhf_feedback (agent_role, content, thumbs_up) VALUES (?, ?, ?)")
        .bind(payload.agent)
        .bind(payload.text)
        .bind(payload.thumbs_up)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => (axum::http::StatusCode::OK, axum::Json(serde_json::json!({"status": "success"}))).into_response(),
        Err(e) => {
            tracing::error!("Failed to save RLHF feedback: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, axum::Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[derive(serde::Deserialize)]
pub struct VaultGraphQuery {
    pub workspace_id: Option<String>,
}

/// Sensus Document Topology Builder (Vault Hub & Dashboard Graph Engine)
pub async fn vault_graph_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<VaultGraphQuery>,
) -> impl IntoResponse {
    let ws_identifier = query.workspace_id.unwrap_or_else(|| "default".to_string());
    
    // Attempt logical relational mapping (from workspaces.name to sensus_documents)
    use sqlx::Row;
    let docs = sqlx::query("SELECT id, relative_path FROM sensus_documents WHERE workspace_id = (SELECT id FROM workspaces WHERE name = ? LIMIT 1)")
        .bind(ws_identifier.clone())
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut nodes = Vec::new();
    let mut links = Vec::new();

    // Central Cybrid Node
    nodes.push(serde_json::json!({
        "id": "root",
        "name": format!("Local Vault ({})", ws_identifier),
        "type": "folder",
        "val": 15
    }));

    for doc in docs {
        let doc_id: i32 = doc.get("id");
        let doc_path: String = doc.get("relative_path");
        let node_id = format!("doc_{}", doc_id);
        
        nodes.push(serde_json::json!({
            "id": node_id.clone(),
            "name": doc_path,
            "type": "file",
            "val": 3
        }));

        links.push(serde_json::json!({
            "source": "root",
            "target": node_id,
            "type": "hierarchy"
        }));
    }

    (axum::http::StatusCode::OK, axum::Json(serde_json::json!({
        "nodes": nodes,
        "links": links
    }))).into_response()
}