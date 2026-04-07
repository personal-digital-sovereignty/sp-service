use axum::{ extract::{Json, State}, response::{ sse::{Event, Sse}, IntoResponse, Response, }, }; use futures_util::StreamExt;  use serde_json::{json, Value}; use std::convert::Infallible; use std::sync::Arc; use tracing::{error, info};

use crate::models::{ OpenAIChatChunkChoice, OpenAIChatChunkDelta, OpenAIChatChunkResponse, OpenAIChatRequest, }; use crate::AppState;
// removed unused explicit scraper import

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
// -------------------------------------------------------------
// Autonomous Fleet Orchestrator (Phase 39)
// -------------------------------------------------------------
pub async fn discover_best_model(hierarchy: Vec<&str>, fallback: &str) -> String {
    let client = reqwest::Client::new();
    if let Ok(res) = client.get("http://127.0.0.1:11434/api/tags").send().await
        && let Ok(json) = res.json::<serde_json::Value>().await
            && let Some(models) = json.get("models").and_then(|m| m.as_array()) {
                let available_names: Vec<&str> = models.iter()
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                    .collect();
                
                for ideal in hierarchy {
                    if let Some(found) = available_names.iter().find(|&&n| n.contains(ideal)) {
                        tracing::info!("✨ [Fleet Orchestrator] Modelo Dinâmico Elevado: '{}' (Alvo Encontrado: '{}')", found, ideal);
                        return found.to_string();
                    }
                }
            }
    tracing::warn!("⚠️ [Fleet Orchestrator] Nenhum modelo da hierarquia encontrado. Fallback de Sobrevivência: '{}'", fallback);
    fallback.to_string()
}

pub async fn discover_cognitive_model_by_tier(tier: &str) -> String {
    let client = reqwest::Client::new();
    
    if let Ok(res) = client.get("http://127.0.0.1:11434/api/tags").send().await
        && let Ok(json) = res.json::<serde_json::Value>().await
        && let Some(models) = json.get("models").and_then(|m| m.as_array()) {
            
            let mut all_models = Vec::new();
            for m in models {
                if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                    let n_lower = name.to_lowercase();
                    if n_lower.contains("embed") || n_lower.contains("bge-") || n_lower.contains("nomic") {
                        continue; // Proteção contra injetar embeddings como agentes
                    }
                    if let Some(size_str) = m.get("details").and_then(|d| d.get("parameter_size")).and_then(|s| s.as_str()) {
                        let s_upper = size_str.to_uppercase();
                        let num_val: f32 = if s_upper.ends_with('B') {
                            s_upper.trim_end_matches('B').parse().unwrap_or(0.0)
                        } else if s_upper.ends_with('M') {
                            s_upper.trim_end_matches('M').parse::<f32>().unwrap_or(0.0) / 1000.0
                        } else {
                            0.0
                        };
                        
                        if num_val > 0.0 && num_val < 300.0 { // Foco SLM / LLM
                            all_models.push((name.to_string(), num_val));
                        }
                    }
                }
            }
            
            if all_models.is_empty() { return "llama3.2:latest".to_string(); }

            // Mathematical Sizing Tiers
            let (min_b, max_b) = match tier {
                "intern" => (0.0, 2.9),
                "junior" => (3.0, 4.0),
                "senior" => (4.1, 9.5),
                "specialist" => (9.6, 999.0),
                _ => (3.0, 999.0)
            };

            // 1. Filter Strict Candidates within the Squad Tier
            let mut strict_matches: Vec<_> = all_models.iter().filter(|(_, s)| *s >= min_b && *s <= max_b).collect();
            if !strict_matches.is_empty() {
                // Ascend to the largest and most capable LLM boundary inside this exact tier
                strict_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let elected = strict_matches[0].0.clone();
                tracing::info!("✨ [Dynamic Squad Scanner] Engatilhando Patente [{}] -> Encontrou mente local matemática: '{}'", tier, elected);
                return elected;
            }

            // 2. Cascade Fallback (Nearest Median Distance)
            // Se o usuário não tiver um "Sênior", a matemática escala verticalmente/horizontalmente pro vizinho mais próximo sem 404s.
            let target_median = (min_b + max_b.min(30.0)) / 2.0;
            all_models.sort_by(|a, b| {
                let diff_a = (a.1 - target_median).abs();
                let diff_b = (b.1 - target_median).abs();
                diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
            });
            
            let fallback_elected = all_models[0].0.clone();
            tracing::warn!("⚠️ [Dynamic Squad Scanner] Fresta Cognitiva! Nenhum SLM estrito para a Patente [{}]. Substituição Euclidiana mais próxima: '{}'", tier, fallback_elected);
            return fallback_elected;
        }
        
    "llama3.2:latest".to_string()
}

use sqlx::Row;

pub async fn query_most_honest_model(db_pool: Option<&sqlx::SqlitePool>, fallback: &str) -> String {
    if let Some(pool) = db_pool {
        // Ignoramos Deepseek ativamente baseado no feedback do comandante de que ele não serve como inquisitor factual
        // Restringindo também modelos MENORES QUE 3B de atuarem como Honest Inquisitors (1b, 1.5b, 1.7b, 2b)
        let row = sqlx::query(
            "SELECT model_name FROM model_hallucinations WHERE model_name NOT LIKE '%deepseek%' AND model_name NOT LIKE '%1b%' AND model_name NOT LIKE '%1.5b%' AND model_name NOT LIKE '%1.7b%' AND model_name NOT LIKE '%2b%' ORDER BY lies_detected ASC, queries_processed DESC LIMIT 1"
        )
        .fetch_optional(pool)
        .await;

        if let Ok(Some(db_row)) = row
            && let Ok(name) = db_row.try_get::<String, _>("model_name") {
                tracing::info!("⚖️ [Inquisidor Solitário] Modelo eleito pelo SQLite (Menos Alucinações): {}", name);
                return name;
            }
    }
    
    tracing::warn!("⚠️ [Inquisidor Solitário] Sem dados históricos suficientes na Tabela de Alucinações. Usando fallback de confiança: {}", fallback);
    fallback.to_string()
}

/// O Primeiro Controlador Cíbrido: Recebendo os Pensamentos do VS Code.
pub async fn chat_completions_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<OpenAIChatRequest>,
) -> Response {
    // Fallback/Extrator: Se 'stream' não vier especificado, assumimos True em respeito aos IDs nativos
    let is_stream = payload.stream.unwrap_or(true);
let requested_model = payload.model.clone();

info!("🔥 [Sovereign Core] Interceptando requisição OpenCode/TUI para o modelo: [{}] | Streaming: {}", requested_model, is_stream);

// Broadcast Log (Cíbrido Live)
let _ = state.log_sender.send(crate::models::LogEntry {
    timestamp: "".to_string(), // O Frontend popula no JS puro
    level: "agent".to_string(),
    message: format!("The Nurse acordou (Requisição de Inferência OpenCode/TUI para {})", requested_model),
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

// ======= Visual Artist Hard-Bypass (G.1 Palette Tool) =======
if payload.visual_artist_mode.unwrap_or(false) && !human_prompt.trim().is_empty() {
    tracing::info!("🎨 [Sovereign Vision] Dedicated Palette Mode Triggered! Bypassing LLM completely for prompt: {}", human_prompt);
    
    let db_clone = state.db.clone();
    let session_guard = payload.session_id.unwrap_or(1); // Default to Main Session if omitted
    let cloned_prompt = human_prompt.clone();
    
    // Store User Prompt in DB just like the normal flow
    tokio::spawn(async move {
        let _ = sqlx::query("INSERT INTO chat_messages (session_id, role, content) VALUES (?, ?, ?)")
            .bind(session_guard)
            .bind("user")
            .bind(&cloned_prompt)
            .execute(&db_clone).await;
    });

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<axum::response::sse::Event, std::convert::Infallible>>();
    let session_guard_bot = payload.session_id.unwrap_or(1);
    let db_clone_bot = state.db.clone();
    let cloned_prompt2 = human_prompt.clone();

    tokio::spawn(async move {
        let loading_chunk = crate::models::OpenAIChatChunkResponse {
            id: format!("chatcmpl-art-{}", uuid::Uuid::new_v4()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Local::now().timestamp(),
            model: "Sovereign Visual Engine".to_string(),
            choices: vec![crate::models::OpenAIChatChunkChoice { index: 0, delta: crate::models::OpenAIChatChunkDelta { role: Some("assistant".to_string()), content: Some(format!("🎨 **Sovereign Vision Engine (Zero-Touch Bypass)**: Acionando SD.cpp no Bare-Metal para forjar imagem fotorealista Baseada em: *{}*.\n\n*(Aguarde, forjando tensores na CPU...)*\n\n", cloned_prompt2)), tool_calls: None }, finish_reason: None }],
            usage: None,
        };
        let _ = tx.send(Ok(axum::response::sse::Event::default().data(serde_json::to_string(&loading_chunk).unwrap_or_default())));
        
        tracing::info!("⚙️ Disparando POST interno para http://127.0.0.1:38001/v1/images/generations...");
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(1800)).build().unwrap_or_default();
        match client.post("http://127.0.0.1:38001/v1/images/generations").json(&serde_json::json!({ "prompt": cloned_prompt2, "n": 1 })).send().await {
            Ok(r) => {
                let status = r.status();
                if !status.is_success() {
                    tracing::error!("❌ [Sovereign Vision] Erro HTTP da Rota de Imagem: {}", status);
                    let _ = sqlx::query("INSERT INTO chat_messages (session_id, role, content) VALUES (?, ?, ?)").bind(session_guard_bot).bind("assistant").bind(format!("❌ [Sovereign Vision] Erro HTTP da Rota de Imagem: {}", status)).execute(&db_clone_bot).await;
                } else if let Ok(j) = r.json::<serde_json::Value>().await {
                    if let Some(url) = j.get("data").and_then(|arr| arr.as_array()).and_then(|a| a.first()).and_then(|f| f.get("url")).and_then(|u| u.as_str()) {
                        tracing::info!("✅ [Sovereign Vision] Imagem concluída! Renderizando URL: {}", url);
                        let markdown_img = format!("![Sovereign Vault Artefact]({})\n\n", url);
                        
                        let ok_chunk = crate::models::OpenAIChatChunkResponse {
                            id: format!("chatcmpl-art-{}", uuid::Uuid::new_v4()), object: "chat.completion.chunk".to_string(), created: chrono::Local::now().timestamp(), model: "Sovereign Visual Engine".to_string(),
                            choices: vec![crate::models::OpenAIChatChunkChoice { index: 0, delta: crate::models::OpenAIChatChunkDelta { role: Some("assistant".to_string()), content: Some(markdown_img.clone()), tool_calls: None }, finish_reason: Some("stop".to_string()) }], usage: None,
                        };
                        let _ = tx.send(Ok(axum::response::sse::Event::default().data(serde_json::to_string(&ok_chunk).unwrap_or_default())));
                        let _ = sqlx::query("INSERT INTO chat_messages (session_id, role, content) VALUES (?, ?, ?)").bind(session_guard_bot).bind("assistant").bind(&markdown_img).execute(&db_clone_bot).await;
                    } else {
                        tracing::error!("❌ [Sovereign Vision] Payload JSON Retornou 200 OK mas não continha data[0].url! Raw: {}", j);
                        let _ = sqlx::query("INSERT INTO chat_messages (session_id, role, content) VALUES (?, ?, ?)").bind(session_guard_bot).bind("assistant").bind("❌ Erro JSON da Image Engine.").execute(&db_clone_bot).await;
                    }
                } else {
                    tracing::error!("❌ [Sovereign Vision] Falha ao parsear reposta JSON do Engine de Imagens.");
                }
            },
            Err(e) => {
                tracing::error!("❌ [Sovereign Vision] Falha Crítica de Conexão com a própria Engine: {}", e);
                let _ = sqlx::query("INSERT INTO chat_messages (session_id, role, content) VALUES (?, ?, ?)").bind(session_guard_bot).bind("assistant").bind(format!("❌ Falha local de Conexão SD.cpp: {}", e)).execute(&db_clone_bot).await;
            }
        }
        let _ = tx.send(Ok(axum::response::sse::Event::default().data("[DONE]")));
    });
    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
    return axum::response::Sse::new(stream).into_response();
}


if let Err(security_alert) = crate::guardrails::evaluate_prompt(&human_prompt, &state.db).await {
    tracing::warn!("🛡️ [Sovereign Guardrails] Ameaça Bloqueada: {}", security_alert.message);
    
    let db_clone = state.db.clone();
    let alert_clone = security_alert.clone();
    tokio::spawn(async move {
        let _ = sqlx::query("INSERT INTO security_logs (event_type, severity, blocked, message, source) VALUES (?, ?, ?, ?, ?)")
            .bind(alert_clone.event_type)
            .bind(alert_clone.severity)
            .bind(alert_clone.blocked)
            .bind(alert_clone.message)
            .bind(alert_clone.source)
            .execute(&db_clone)
            .await;
    });

    let _ = state.log_sender.send(crate::models::LogEntry {
        timestamp: "".to_string(),
        level: "security".to_string(),
        message: format!("Threat Intel Blocked: {}", security_alert.message),
    });

    let chunk = crate::models::OpenAIChatChunkResponse {
        id: format!("chatcmpl-sec-{}", uuid::Uuid::new_v4()),
        object: "chat.completion.chunk".to_string(),
        created: chrono::Local::now().timestamp(),
        model: requested_model.clone(),
        choices: vec![crate::models::OpenAIChatChunkChoice {
            index: 0,
            delta: crate::models::OpenAIChatChunkDelta {
                role: Some("assistant".to_string()),
                content: Some(format!("🚨 **Sovereign Guardrails Interception**\n\nSua requisição feriu as políticas do Nexus Command Center.\n\n- **Tipo:** {}\n- **Severidade:** {}\n- **Motivo:** {}", security_alert.event_type, security_alert.severity, security_alert.message)),
                tool_calls: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: None,
    };
    
    let stream = futures_util::stream::iter(vec![
        Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data(serde_json::to_string(&chunk).unwrap_or_default())),
        Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data("[DONE]")),
    ]);
    return axum::response::Sse::new(stream).into_response();
}

let mut sys_temperature: Option<f64> = None;
let mut sys_top_k: Option<i64> = None;
let mut global_system_prompt: Option<String> = None;
let mut system_ai_name = "The Nurse".to_string();
let mut resolved_model = requested_model.clone();

// 🛑 THE SOVEREIGN FIREWALL (Zero-Day Model Fallback) 🛑
// If OpenCode (or the IDE) injects commercial models blindly, we forcefully hijack 
// them down to the Sovereign Private Mesh locally, ensuring NO 404 Ollama Panics on Factory Installs.
if requested_model.to_lowercase().contains("gpt") || requested_model.to_lowercase().contains("claude") {
    let hierarchy = vec!["qwen2.5:14b", "gemma2:9b", "gemma2", "llama3.1:8b", "llama3.1", "qwen2.5:7b", "qwen2.5", "llama3.2"];
    resolved_model = discover_best_model(hierarchy, "llama3.2:latest").await;
    tracing::info!("🔄 Proxy OpenCode/IDE enviou modelo comercial [{}]. Hijacking dinâmico para Endpoint Soberano: [{}]", requested_model, resolved_model);
}

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
                tracing::info!("🧠 [Sovereign Router] Roteando intenção '{}' para o Agente Dedicado Mestre: {}", target_key, resolved_model);
            }
        } else if let Some(model_str) = parsed.get("llm_model").and_then(|v| v.as_str()) {
            // Restore legacy fallback explicitly from DB if Sovereign Routing failed, BUT the DB has a default engine.
            if !model_str.is_empty() { 
                resolved_model = model_str.to_string(); 
                tracing::info!("🔄 [Sovereign Router] Usando Motor Estático da Base de Dados: {}", resolved_model);
            }
        }
        
        if let Some(t) = parsed.get("temperature").and_then(|v| v.as_f64()) { sys_temperature = Some(t); }
        if let Some(k) = parsed.get("top_k").and_then(|v| v.as_i64()) { sys_top_k = Some(k); }
        
        let mut base_prompt = String::new();
        // Support both ai_name and aiName json structures
        let name_val = parsed.get("ai_name").or_else(|| parsed.get("aiName")).and_then(|v| v.as_str());
        if let Some(name) = name_val
            && !name.is_empty() {
                system_ai_name = name.to_string();
                base_prompt = format!("Identidade Sistêmica: Assuma a persona local soberana definida pelo usuário. Seu nome é {}. Aja de forma coerente e amigável sem ser repetitivo.\n\n", name);
            }
        
        if let Some(p) = parsed.get("system_prompt").and_then(|v| v.as_str())
            && !p.is_empty() { 
                base_prompt.push_str(p); 
            }
        
        if !base_prompt.is_empty() {
            global_system_prompt = Some(base_prompt);
        }
    }
}
let ollama_model = resolved_model.clone();

// ===== THE PLANNER (MACRO ORCHESTRATION BYPASS) =====
if human_prompt.to_lowercase().starts_with("/plan") {
    let query = human_prompt[5..].trim().to_string();
    info!("🧠 [Sovereign Core] Plan & Execute Task detectada: /plan -> Iniciando Macro Orquestração em Background...");
    
    let state_clone = state.clone();
    let model_to_use = resolved_model.clone();
    
    tokio::spawn(async move {
        crate::plan_execute::start_plan_and_execute(query, state_clone, model_to_use).await;
    });

    let msg = "🧭 **Plan & Execute (Macro-Orquestração) Iniciado!**\nSua tarefa foi inserida no Threadpool Assíncrono do Cíbrido. O Planner irá quebrar seu pedido em etapas menores, validará nativamente na formatação strict JSON, e o Executor cuidará de cada etapa seqüencialmente sem travar seu terminal. \n\n*Acompanhe o Plasma Widget ou Logs para ver a orquestração em andamento!*".to_string();

    let chunk = crate::models::OpenAIChatChunkResponse {
        id: format!("chatcmpl-plan-{}", uuid::Uuid::new_v4()),
        object: "chat.completion.chunk".to_string(),
        created: chrono::Local::now().timestamp(),
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
let (tx_sse, mut rx_sse) = tokio::sync::mpsc::unbounded_channel::<axum::response::sse::Event>();
let tx_sse_clone = tx_sse.clone();

let payload = payload.clone();
let state = state.clone();
let human_prompt = human_prompt.clone();
let requested_model = requested_model.clone();
let ollama_model = ollama_model.clone();
let global_system_prompt = global_system_prompt.clone();
let sys_temperature = sys_temperature;
let sys_top_k = sys_top_k;

tokio::spawn(async move {
    let mut web_context = String::new();
    let mut sys_context = String::new();

    let send_thought = |text: &str| {
        let chunk = crate::models::OpenAIChatChunkResponse {
            id: format!("chatcmpl-thought-{}", uuid::Uuid::new_v4()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Local::now().timestamp(),
            model: requested_model.clone(),
            choices: vec![crate::models::OpenAIChatChunkChoice {
                index: 0,
                delta: crate::models::OpenAIChatChunkDelta {
                    role: Some("assistant".to_string()),
                    content: Some(text.to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            usage: None,
        };
        let _ = tx_sse_clone.send(axum::response::sse::Event::default().data(serde_json::to_string(&chunk).unwrap()));
    };

    let is_web = human_prompt.to_lowercase().starts_with("/web");
    let is_sys = human_prompt.to_lowercase().starts_with("/sys");

if payload.deep_research.unwrap_or(false) {
    let mut url_to_scrape = String::new();
    let mut user_question = human_prompt.clone();
    
    for word in human_prompt.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            url_to_scrape = word.to_string();
            user_question = user_question.replace(word, "").trim().to_string();
            break;
        }
    }

    if !url_to_scrape.is_empty() {
        send_thought(&format!("<thought>🔎 Lendo URL solicitada Diretamente: {}...</thought>\n\n", url_to_scrape));
        tracing::info!("🕸️ [WAG Native] O botão 'Deep Research' estava ATIVO na UI. Acionando raspagem perene p/ {}", url_to_scrape);
        let wag_args = serde_json::json!({ "url": url_to_scrape });
        let wag_result = crate::mcp::execute_mcp_tool(&state, "mcp_deep_research", &wag_args).await;
        
        web_context = format!("INSTRUÇÃO SISTÊMICA (DEEP RESEARCH/WAG): O motor de Agentic Web-Scraping leu a URL solicitada ({}) e a salvou fisicamente na Sensus Database Vault local do usuário.\n\nEis o PREVIEW direto (Truncado) dos dados limpos recém-extraídos da internet:\n\n{}\n\nAGENTE: Baseado estritamente nestes dados in-locus, responda/analise de forma soberba a: '{}'", url_to_scrape, wag_result, user_question);
    } else {
        tracing::info!("🧠 [WAG Multi-Hop] Nenhuma URL explícita no prompt. Iniciando Deep Research Agentico (Sub-Processo LLM) para Múltiplas Visões: '{}'", user_question);
        send_thought("<thought>Iniciando Sovereign Search Engine (WAG)...</thought>\n<thought>Acionando Sub-LLM O Doutrinador...</thought>\n");
        // 1. Notifica o Frontend que o Loop Começou
        let _ = state.log_sender.send(crate::models::LogEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            level: "agent".to_string(),
            message: "🧠 [Deep Research] Acionando O Doutrinador (Sub-LLM) para quebrar sua pergunta em Múltiplas Queries Analíticas...".to_string(),
        });

        let query_system_prompt = "Você é o Agente Doutrinador de Buscas Profundas. Leia a CONVERSA HISTÓRICA atual do usuário com o assistente. Com base no contexto e no ÚLTIMO pedido do usuário, crie exatamente 3 strings de pesquisa distintas para vasculhar a internet profundamente a respeito do tema solicitado. Retorne EXATAMENTE UM ARRAY JSON de strings, sem NENHUM texto extra. Exemplo: [\"query1\", \"query2\", \"query3\"]";
        
        let mut synth_messages = vec![
            serde_json::json!({ "role": "system", "content": query_system_prompt })
        ];

        let msg_count = payload.messages.len();
        let skip_count = msg_count.saturating_sub(6);

        for msg in payload.messages.iter().skip(skip_count) {
            let content_str = match &msg.content {
                Some(crate::models::MessageContent::Text(t)) => t.clone(),
                Some(crate::models::MessageContent::Multimodal(parts)) => {
                    parts.iter().filter_map(|p| p.get("text").and_then(|t| t.as_str())).collect::<String>()
                },
                None => "".to_string(),
            };
            synth_messages.push(serde_json::json!({ "role": msg.role, "content": content_str }));
        }

        let llm_payload = serde_json::json!({
            "model": requested_model.clone(),
            "messages": synth_messages,
            "format": "json",
            "stream": false,
            "options": { "temperature": 0.2 }
        });

        // Resolve a Conexão Dinâmica do Ollama via SQLite O.S
        let mut sub_ollama_url = "http://127.0.0.1:11434".to_string();
        if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'ollama_clusters'").fetch_optional(&state.db).await {
            let val: String = sqlx::Row::get(&row, "value_json");
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
                let active_id = parsed.get("active_cluster_id").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(clusters) = parsed.get("clusters").and_then(|v| v.as_array()) {
                    for c in clusters {
                        if c.get("id").and_then(|v| v.as_str()).unwrap_or("") == active_id
                            && let Some(url) = c.get("url").and_then(|v| v.as_str()) {
                                sub_ollama_url = url.trim_end_matches('/').to_string();
                            }
                    }
                }
            }
        }
        if sub_ollama_url == "http://host.docker.internal:11434" && std::env::var("SOVEREIGN_RUN_ENV").unwrap_or_default() == "native" {
            sub_ollama_url = "http://127.0.0.1:11434".to_string();
        }

        let query_endpoint = format!("{}/api/chat", sub_ollama_url);
        let mut extracted_queries = Vec::new();

        if let Ok(res) = state.http_client.post(&query_endpoint).json(&llm_payload).timeout(std::time::Duration::from_secs(30)).send().await
            && let Ok(json_res) = res.json::<serde_json::Value>().await
                && let Some(content) = json_res.get("message").and_then(|m| m.get("content").and_then(|c| c.as_str()))
                    && let Ok(queries) = serde_json::from_str::<Vec<String>>(content) {
                        extracted_queries = queries;
                    }

        if extracted_queries.is_empty() {
            tracing::warn!("⚠️ [WAG Multi-Hop] Sub-LLM falhou no Strict JSON. Fallback para Query Direta.");
            extracted_queries = vec![user_question.clone()];
        }

        let _ = state.log_sender.send(crate::models::LogEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            level: "agent".to_string(),
            message: format!("📡 [Deep Research] 3 Queries Forjadas: {:?}. Lançando {} Spiders Concurrentes à Malha SearxNG...", extracted_queries, extracted_queries.len()),
        });

        send_thought(&format!("<thought>Desdobramento em {} visões paralelas concluído. Lançando Meta-Spiders Cíbridas...</thought>\n", extracted_queries.len()));
        for q in &extracted_queries {
            send_thought(&format!("<thought>🔍 Querying web: \"{}\"</thought>\n", q));
        }

        let engine = std::sync::Arc::new(crate::research::DeepResearchEngine::new(Some(state.db.clone()), Some(state.adblock_engine.clone()), Some(state.vault_path.clone())));
        
        let mut search_handles = Vec::new();
        // Dispara Paralelizações puras no CPU
        for q in extracted_queries {
            let engine_clone = engine.clone();
            let q_clone = q.clone();
            search_handles.push(tokio::spawn(async move {
                engine_clone.search_web(&q_clone).await
            }));
        }

        let mut all_links = Vec::new();
        let mut all_snippets = String::new();
        for res in futures_util::future::join_all(search_handles).await {
            if let Ok(Ok(search_res)) = res {
                all_links.extend(search_res.links);
                all_snippets.push_str(&search_res.snippets);
                all_snippets.push('\n');
            }
        }
        all_links.sort();
        all_links.dedup();
        all_links.truncate(6); // Poda agressiva p/ não atolar a KV Cache GPU!

        let _ = state.log_sender.send(crate::models::LogEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            level: "agent".to_string(),
            message: format!("🕸️ [Deep Research] Capturado top {} Master-URLs. Lendo simultaneamente de {} IPs Cíbridos...", all_links.len(), all_links.len()),
        });

        let mut scrape_handles = Vec::new();
        for link in all_links {
            let engine_clone = engine.clone();
            scrape_handles.push(tokio::spawn(async move {
                let markdown = engine_clone.scrape_url(&link).await.unwrap_or_else(|_| String::new());
                (link, markdown)
            }));
        }

        let mut master_dossier = String::new();
        if !all_snippets.trim().is_empty() {
             master_dossier.push_str(&format!("## ZERO-CLICK SEARCH SNIPPETS (Multi-Query)\n{}\n\n", all_snippets));
        }

        for res in futures_util::future::join_all(scrape_handles).await {
            if let Ok((link, mut markdown)) = res
                && markdown.len() > 100 {
                    markdown.truncate(4000); // 4KB por página x 6 = 24KB seguro dentro da GPU local (8B Contexto Mínimo Llama3).
                    // Cortar quebra uma formatação. Tratamento leviano p/ velocidade.
                    master_dossier.push_str(&format!("## Origem Escaneada Profundamente: {}\n{}\n\n", link, markdown));
                }
        }

        let _ = state.log_sender.send(crate::models::LogEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            level: "agent".to_string(),
            message: "✅ [Deep Research] Dossiê Multi-Site Concluído Massivamente! Despejando Relatório Estratégico no Córtex Principal.".to_string(),
        });

        web_context = format!("INSTRUÇÃO SISTÊMICA (MULTI-HOP DEEP RESEARCH): O motor Sovereign disparou Sub-Agentes que leram simultaneamente as visões e dados de dezenas páginas globais sobre a missão do usuário.\n\nEis o Dossiê Massivo (truncado em blocos para caber na memória) gerado em tempo-real para você:\n\n{}\n\nAGENTE CHEFE: Baseado ESTRITAMENTE E APENAS na visão transversal deste dossiê recém extraído, estruture uma resposta madura, profunda e com autoridade para: '{}'. Sempre referencie a Origem Escaneada (URL) no seu texto livremente.", master_dossier, user_question);
    }
} else if is_web {
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
    sys_context = format!("INSTRUÇÃO SISTÊMICA ({}): O usuário solicitou análise profunda sobre a arquitetura 'Sovereign Pair'. Somos um sistema Cíbrido puramente em Rust ({}/Axum) e Svelte 5 + Tailwind na UI. Usamos LLMs Locais (Ollama) mapeados via SQL. Foque em responder a seguinte dúvida de Engenharia: '{}'", system_ai_name.to_uppercase(), system_ai_name, query);
}
// =========================================================

let active_session_id = crate::api_chat::get_or_create_session(&state.db, payload.session_id, &human_prompt).await;

// Grava no Banco a pergunta Humana
crate::api_chat::save_message(&state.db, active_session_id, "user", &human_prompt).await;

// 2. Transcrever Mensagens Complexas (Multimodal/Arrays) para Strict Strings + Injeção de RAG Nativo
let mut purified_messages: Vec<Value> = Vec::new();

// --- SOVEREIGN CONTEXT INJECTOR (RAG V2 - KANBAN) ---
let mut project_context = String::new();
if let Some(pid) = payload.project_id
    && let Ok(Some(proj_info)) = sqlx::query("SELECT name, purpose FROM projects WHERE id = ?")
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

        if let Ok(tasks) = sqlx::query("SELECT title, status, created_at, deadline FROM tasks WHERE project_id = ? AND status != 'Done'")
            .bind(&pid)
            .fetch_all(&state.db)
            .await
            && !tasks.is_empty() {
                project_context.push_str("\n📌 TAREFAS ATIVAS NO KANBAN (Com Cronologia):\n");
                for row in tasks {
                    let t_title: String = sqlx::Row::get(&row, "title");
                    let t_status: String = sqlx::Row::get(&row, "status");
                    let t_created: Option<String> = sqlx::Row::get(&row, "created_at");
                    let t_deadline: Option<String> = sqlx::Row::get(&row, "deadline");
                    
                    let c = t_created.unwrap_or_else(|| "Desconhecida".to_string());
                    let d = t_deadline.unwrap_or_else(|| "Sem prazo".to_string());
                    
                    project_context.push_str(&format!("- [{}] {} (Criada: {} | Prazo: {})\n", t_status, t_title, c, d));
                }
            }

        if let Ok(docs) = sqlx::query("SELECT file_path FROM project_documents WHERE project_id = ?")
            .bind(&pid)
            .fetch_all(&state.db)
            .await
            && !docs.is_empty() {
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
}

if let Some(global_prompt) = global_system_prompt {
    // Injeta a Persona Customizada definida na UI com Alta Prioridade Cognitiva (Enforcement Tático)
    purified_messages.push(json!({
        "role": "system",
        "content": format!(">>> DIRETRIZ ABSOLUTA DE PERSONALIDADE, COMPORTAMENTO E NÍVEL DE CONHECIMENTO TÉCNICO <<<\n{}\n>>> VOCÊ DEVE AGIR ESTRITAMENTE CONFORME ESTA DIRETRIZ EM TODAS AS SUAS RESPOSTAS, MANTENDO A IMERSÃO E A PROFUNDIDADE INTELECTUAL EXIGIDA. <<<", global_prompt)
    }));
}

// Injeta a Orquestração do ReWOO (Reasoning Without Observation) Apenas Se Houver Plano
let workspace_id = payload.workspace_id.clone().unwrap_or_else(|| "default".to_string());
send_thought("<thought>Consultando Plano de Tarefas Sovereign Hub...</thought>\n");
let rewoo_observations = crate::rewoo::execute_rewoo_plan(&human_prompt, &workspace_id, &state.db).await;
if !rewoo_observations.trim().is_empty() && rewoo_observations != "ReWOO Accumulated Observations:\n" {
    send_thought("<thought>Sovereign ReWOO: Executando nós paralelos mapeados na memória local...</thought>\n<thought>Grafo ReWOO consolidado. Injetando descobertas.</thought>\n\n");
    tracing::debug!("🧠 ReWOO Workflow Executed. Injecting compiled DAG observations.");
    purified_messages.push(json!({
        "role": "system",
        "content": rewoo_observations
    }));
}

// Injeta o Override de Desenho (RLHF Bypass) para combater a recusa algorítmica de IAs de texto (Qwen/Gemma)
let hp_lower = human_prompt.to_lowercase();
if hp_lower.contains("desenhe ") || hp_lower.contains("faça uma imagem") || hp_lower.contains("crie uma imagem") || hp_lower.contains("imagine um") {
    purified_messages.push(json!({
        "role": "system",
        "content": ">> SYSTEM OVERRIDE (TOOL CALL ENFORCEMENT) <<\nVocê possui TOTAL capacidade de gerar imagens fotorealísticas, pois está conectado ao motor Bare-Metal SD.cpp! Você NÃO DEVE responder em plain-text dizendo que 'não pode criar imagens' ou pedindo desculpas. VOCÊ DEVE OBRIGATORIAMENTE emitir uma Tool Call JSON para a ferramenta 'dispatch_visual_artist' traduzindo o desejo do usuário para Inglês."
    }));
}

// Injeta a Memória de Estado Dinâmica (Working Memory) para mitigar a amnésia de SLMs em longos diálogos
let turn_count = payload.messages.len();
if turn_count > 3 {
    let working_memory_prompt = format!(
        "<state_memory>\n\
        {{\n\
            \"context_retention\": \"Você está no turno {} da interação. Ative sua Working Memory estrutural. Não levante informações redundantes já explicadas.\",\n\
            \"task_goal\": \"Sintetizar respostas de fluidez temporal, focando ESTRITAMENTE na fronteira do prompt atual. Considere o histórico como 'established_facts'.\",\n\
            \"pending_verification\": false\n\
        }}\n\
        </state_memory>\n\
        [SISTEMA INTERNO]: Leia a <state_memory> JSON acima antes de formatar a saída. Não responda baseado unicamente na reatividade do LLM, seja inteligente em não repetir o passado.",
        turn_count
    );

    purified_messages.push(json!({
        "role": "system",
        "content": working_memory_prompt
    }));
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
let mut ollama_options = json!({
    "num_keep": 4, // Forçar Lock do System Prompt na VRAM
    "num_ctx": 8192, // Desidratação do Nurse (16GB RAM overhead fix resolvido pra RPI/OracleA1)
    "repeat_penalty": 1.15
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
let mut injected_tools = Vec::new();
if let Some(tools) = payload.tools {
    injected_tools.extend(tools);
}

// ================= THE SOVEREIGN VISUAL ARTIST (NATIVE TOOL) =================
let visual_artist_tool = serde_json::json!({
    "type": "function",
    "function": {
        "name": "dispatch_visual_artist",
        "description": "Ferramenta para desenhar ilustrações, criar quadros ou arte visual fotorealista (Text-to-Image) quando o usuário pedir para 'desenhar', 'criar imagem' ou tirar uma foto. SEMPRE use esta ferramenta em vez de dizer que não pode gerar imagens.",
        "parameters": {
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "O prompt fotorealista, cinematográfico e extremamente detalhado em INGLÊS da cena visual desejada."
                }
            },
            "required": ["prompt"]
        }
    }
});
injected_tools.push(serde_json::from_value(visual_artist_tool).unwrap());

ollama_payload["tools"] = json!(injected_tools);

if let Some(tool_choice) = payload.tool_choice {
    ollama_payload["tool_choice"] = tool_choice;
}

// Resgate Masterplan da Tabela de Configurações
let env_ollama_url = std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
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
    // Simplificado: sempre usar localhost se for host native O.S para compatibilidade IPv6 (MacOS)
    if std::env::var("SOVEREIGN_RUN_ENV").unwrap_or_default() == "native" {
         ollama_base_url = "http://localhost:11434".to_string();
    }
}

let endpoint = format!("{}/api/chat", ollama_base_url);

let mut retry_count = 0;
let res = loop {
    let response_result = state
        .http_client
        .post(&endpoint)
        .json(&ollama_payload)
        .send()
        .await;

    match response_result {
        Ok(r) if r.status().is_success() => break r,
        Ok(r) => {
            let status = r.status();
            let err_body = r.text().await.unwrap_or_default();

            // Interceptador de Auto-Cura para modelos alheios a Tool-Calling via API
            if retry_count == 0 && status == reqwest::StatusCode::BAD_REQUEST && (err_body.contains("does not support") || err_body.contains("tools")) {
                tracing::warn!("⚠️ Modelo Ollama [{}] rejeitou a payload via API ({}). Removendo Tools e re-entrando em ciclo autônomo...", ollama_model, err_body);
                if let Some(obj) = ollama_payload.as_object_mut() {
                    obj.remove("tools");
                    obj.remove("tool_choice");
                }
                retry_count += 1;
                continue;
            }

            error!("❌ Ollama recusou a requisição HTTP. Status: {} - Body: {}", status, err_body);
        
        let err_msg = if status == reqwest::StatusCode::NOT_FOUND && err_body.contains("not found") {
            format!("*(Conexão Remota)* 🚨 **Falha: Modelo Ausente no Nó Remoto**\nO modelo `{}` não está instalado no seu nó remoto (`{}`).\n\n**Solução:** Vá em 'Gerenciar Nós' nas configurações e execute o Download/Pull deste modelo para que nossos agentes possam usá-lo.", ollama_model, endpoint)
        } else {
            format!("*(Protocolo de Fallback)* 🚨 Falha no nó LLM configurado ({}). Status HTTP: {} - Body: {}", endpoint, status, err_body)
        };
        
        let err_chunk = crate::models::OpenAIChatChunkResponse {
            id: format!("chatcmpl-err-{}", uuid::Uuid::new_v4()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Local::now().timestamp(),
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
        let mut error_stream = futures_util::stream::iter(vec![
            Ok::<Event, Infallible>(Event::default().data(serde_json::to_string(&err_chunk).unwrap_or_default())),
            Ok::<Event, Infallible>(Event::default().data("[DONE]")),
        ]);
        while let Some(Ok(event)) = futures_util::StreamExt::next(&mut error_stream).await {
            let _ = tx_sse_clone.send(event);
        }
        return;
    },
    Err(e) => {
            if is_custom_cluster {
                tracing::warn!("🔄 OCI/Mesh Node Offline ({}). Iniciando Autonomia de Fallback para Localhost (127.0.0.1:11434)...", e);
                let local_endpoint = "http://127.0.0.1:11434/api/chat";
                match state.http_client.post(local_endpoint).json(&ollama_payload).send().await {
                    Ok(fallback_r) if fallback_r.status().is_success() => {
                        tracing::info!("✅ [Sovereign Core] Autonomia de Fallback ativada com sucesso. Servindo LLM Localmente!");
                        break fallback_r;
                    },
                _ => {
                    error!("🚨 Falha FATAL no nó mestre e no nó escravo.");
                    let err_msg = format!("*(Sovereign Core)* 🚨 **Severidade Máxima: Abandono de Frota**\nO Cluster OCI Oracle não respondeu (`{}`) E o Fallback Autônomo Local (127.0.0.1:11434) também está offline! Impossível iniciar inferência.", e);
                    let err_chunk = crate::models::OpenAIChatChunkResponse {
                        id: format!("chatcmpl-err-{}", uuid::Uuid::new_v4()),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Local::now().timestamp(),
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
                    let mut error_stream = futures_util::stream::iter(vec![
                        Ok::<Event, Infallible>(Event::default().data(serde_json::to_string(&err_chunk).unwrap_or_default())),
                        Ok::<Event, Infallible>(Event::default().data("[DONE]")),
                    ]);
                    while let Some(Ok(event)) = futures_util::StreamExt::next(&mut error_stream).await {
                        let _ = tx_sse_clone.send(event);
                    }
                    return;
                }
            }
        } else {
            error!("🚨 Falha FATAL ao encontrar o motor LLM Local: {}", e);
            let err_msg = format!("*(The Nurse Local)* ⚠️ Serviço de IA Local (Ollama) inacessível na porta 11434. O daemon está rodando?\n\nErro: `{}`", e);

            let err_chunk = crate::models::OpenAIChatChunkResponse {
                id: format!("chatcmpl-err-{}", uuid::Uuid::new_v4()),
                object: "chat.completion.chunk".to_string(),
                created: chrono::Local::now().timestamp(),
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
            let mut error_stream = futures_util::stream::iter(vec![
                Ok::<Event, Infallible>(Event::default().data(serde_json::to_string(&err_chunk).unwrap_or_default())),
                Ok::<Event, Infallible>(Event::default().data("[DONE]")),
            ]);
            while let Some(Ok(event)) = futures_util::StreamExt::next(&mut error_stream).await {
                let _ = tx_sse_clone.send(event);
            }
            return;
        }
    }
    } // closes match
}; // closes loop

// Criamos o Túnel de Transmissão contínua em Rust
// Variáveis locais puras para contabilização na Closure do Stream
let tracking_telemetry = state.telemetry.clone();
let tracking_db = state.db.clone();
let tracking_session = active_session_id;
let tracking_model = ollama_model.clone();

let tracking_human_query = human_prompt.clone();
let mut tracking_rag_context = project_context.clone();
if !web_context.is_empty() { tracking_rag_context.push('\n'); tracking_rag_context.push_str(&web_context); }
if !sys_context.is_empty() { tracking_rag_context.push('\n'); tracking_rag_context.push_str(&sys_context); }
if tracking_rag_context.trim().is_empty() { tracking_rag_context = "Interação Direta (Zero-Shot / Sem Contexto RAG)".to_string(); }

let start_time = std::time::Instant::now();
let mut session_tokens = 0;
let mut accumulator = String::new(); // Memory Builder da Resposta do Agente
let tracking_log_sender = state.log_sender.clone();
let tx_map_capture = tx_sse_clone.clone();

// Extraímos os Bytes Chunk a Chunk e mapeamos pro formato OpenAI SSE:
let mut map_stream = res.bytes_stream().map(move |result| {
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
                                let mut is_visual_artist = false;
                                let mut visual_prompt = String::new();

                                for (i, tc) in tool_calls_arr.iter().enumerate() {
                                    let mut new_tc = crate::models::ChunkToolCall {
                                        index: Some(i as i32),
                                        id: Some(format!("call_{}", uuid::Uuid::new_v4().to_string().replace("-", "").chars().take(8).collect::<String>())),
                                        r#type: Some("function".to_string()),
                                        function: None,
                                    };
                                    if let Some(func) = tc.get("function") {
                                        let name = func.get("name").and_then(|n| n.as_str()).map(|n| n.to_string());
                                        
                                        // Intercept Sovereign Native Visual Artist
                                        if name.as_deref() == Some("dispatch_visual_artist") {
                                            is_visual_artist = true;
                                            if let Some(args_val) = func.get("arguments") {
                                                if args_val.is_object() {
                                                    visual_prompt = args_val.get("prompt").and_then(|p| p.as_str()).unwrap_or("").to_string();
                                                } else if let Some(parsed) = args_val.as_str().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) {
                                                    visual_prompt = parsed.get("prompt").and_then(|p| p.as_str()).unwrap_or("").to_string();
                                                }
                                            }
                                        }
                                        
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
                                
                                if is_visual_artist && !visual_prompt.is_empty() {
                                    let tx_visual = tx_map_capture.clone();
                                    let p_clone = visual_prompt.clone();
                                    let req_model = requested_model.clone();
                                    
                                    tokio::spawn(async move {
                                        let loading_msg = format!("\n\n🎨 **Sovereign Vision Engine**: Acionando SD.cpp no Bare-Metal para forjar imagem fotorealista Baseada em: *{}*. Aguarde...\n\n", p_clone);
                                        let chunk = crate::models::OpenAIChatChunkResponse {
                                            id: format!("chatcmpl-art-{}", uuid::Uuid::new_v4()),
                                            object: "chat.completion.chunk".to_string(),
                                            created: chrono::Local::now().timestamp(),
                                            model: req_model.clone(),
                                            choices: vec![crate::models::OpenAIChatChunkChoice { index: 0, delta: crate::models::OpenAIChatChunkDelta { role: Some("assistant".to_string()), content: Some(loading_msg), tool_calls: None }, finish_reason: None }],
                                            usage: None,
                                        };
                                        let _ = tx_visual.send(Event::default().data(serde_json::to_string(&chunk).unwrap()));

                                        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(120)).build().unwrap_or_default();
                                        if let Ok(r) = client.post("http://127.0.0.1:38001/v1/images/generations").json(&serde_json::json!({ "prompt": p_clone })).send().await {
                                            if let Ok(j) = r.json::<serde_json::Value>().await {
                                                if let Some(url) = j.get("data").and_then(|arr| arr.as_array()).and_then(|a| a.first()).and_then(|f| f.get("url")).and_then(|u| u.as_str()) {
                                                    let ok_msg = format!("![Sovereign Vault Artefact]({})\n\n", url);
                                                    let ok_chunk = crate::models::OpenAIChatChunkResponse {
                                                        id: format!("chatcmpl-art-{}", uuid::Uuid::new_v4()),
                                                        object: "chat.completion.chunk".to_string(),
                                                        created: chrono::Local::now().timestamp(),
                                                        model: req_model.clone(),
                                                        choices: vec![crate::models::OpenAIChatChunkChoice { index: 0, delta: crate::models::OpenAIChatChunkDelta { role: Some("assistant".to_string()), content: Some(ok_msg), tool_calls: None }, finish_reason: None }],
                                                        usage: None,
                                                    };
                                                    let _ = tx_visual.send(Event::default().data(serde_json::to_string(&ok_chunk).unwrap()));
                                                } else {
                                                    let err_chunk = crate::models::OpenAIChatChunkResponse {
                                                        id: format!("chatcmpl-art-{}", uuid::Uuid::new_v4()),
                                                        object: "chat.completion.chunk".to_string(),
                                                        created: chrono::Local::now().timestamp(),
                                                        model: req_model.clone(),
                                                        choices: vec![crate::models::OpenAIChatChunkChoice { index: 0, delta: crate::models::OpenAIChatChunkDelta { role: Some("assistant".to_string()), content: Some("\n\n❌ Falha estrutural no motor de Visão. A imagem não foi renderizada.\n".to_string()), tool_calls: None }, finish_reason: None }],
                                                        usage: None,
                                                    };
                                                    let _ = tx_visual.send(Event::default().data(serde_json::to_string(&err_chunk).unwrap()));
                                                }
                                            }
                                        } else {
                                            let offline_chunk = crate::models::OpenAIChatChunkResponse {
                                                id: format!("chatcmpl-art-{}", uuid::Uuid::new_v4()),
                                                object: "chat.completion.chunk".to_string(),
                                                created: chrono::Local::now().timestamp(),
                                                model: req_model.clone(),
                                                choices: vec![crate::models::OpenAIChatChunkChoice { index: 0, delta: crate::models::OpenAIChatChunkDelta { role: Some("assistant".to_string()), content: Some("\n\n❌ Motor Visual OFFLINE: O processo SD.cpp na Porta 7860 não pôde ser alcançado.\n".to_string()), tool_calls: None }, finish_reason: None }],
                                                usage: None,
                                            };
                                            let _ = tx_visual.send(Event::default().data(serde_json::to_string(&offline_chunk).unwrap()));
                                        }
                                    });
                                    
                                    // Bypass Tool array returning to UI. Svelte is naive to this tool natively.
                                    return Ok::<Event, Infallible>(Event::default());
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
                                
                                // 🗄️ Histórico Absoluto: Persistindo Tokens e Uptime no Ledger SQLite
                                let sql_db = tracking_db.clone();
                                let sql_model = tracking_model.clone();
                                let sql_tokens = total_real_tokens as i64;
                                let sql_dur = duration as i64;
                                tokio::spawn(async move {
                                    let _ = sqlx::query(
                                        "INSERT INTO model_metrics (model_name, total_tokens, total_duration_ms, first_used_at, last_used_at) 
                                         VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                                         ON CONFLICT(model_name) DO UPDATE SET 
                                             total_tokens = total_tokens + excluded.total_tokens,
                                             total_duration_ms = total_duration_ms + excluded.total_duration_ms,
                                             last_used_at = CURRENT_TIMESTAMP"
                                    )
                                    .bind(&sql_model)
                                    .bind(sql_tokens)
                                    .bind(sql_dur)
                                    .execute(&sql_db)
                                    .await;
                                });
                                
                                let tps = if duration > 0 { (total_real_tokens as f64 / (duration as f64 / 1000.0)).round() } else { 0.0 };
                                let _ = tracking_log_sender.send(crate::models::LogEntry {
                                    timestamp: chrono::Local::now().to_rfc3339(),
                                    level: "system".to_string(),
                                    message: format!("⚡ Geração de Conhecimento: {} tokens a {} T/s [{}]", total_real_tokens, tps, tracking_model),
                                });

                                // 🗄️ Imortalidade de Diálogo: Insere via Spawn para não bloquear o Axum Stream
                                let final_text = accumulator.clone();
                                let db_clone = tracking_db.clone();
                                let tr_q = tracking_human_query.clone();
                                let tr_ctx = tracking_rag_context.clone();
                                
                                tokio::spawn(async move {
                                    crate::api_chat::save_message(&db_clone, tracking_session, "assistant", &final_text).await;

                                    // Engatilha a Avaliação (Auto Evaluator / The Nurse) no Rastro
                                    let eval_id = uuid::Uuid::new_v4().to_string();
                                    let _ = sqlx::query("INSERT INTO evaluations (id, conversation_id, user_query, rag_context, ai_response, status) VALUES (?, ?, ?, ?, ?, 'pending')")
                                        .bind(&eval_id)
                                        .bind(tracking_session.to_string())
                                        .bind(&tr_q)
                                        .bind(&tr_ctx)
                                        .bind(&final_text)
                                        .execute(&db_clone)
                                        .await;
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

while let Some(Ok(event)) = futures_util::StreamExt::next(&mut map_stream).await {
    let _ = tx_sse_clone.send(event);
}
}); // Fim do tokio::spawn

let final_stream = async_stream::stream! {
    while let Some(event) = rx_sse.recv().await {
        yield Ok::<_, std::convert::Infallible>(event);
    }
};

// Envolve a Stream num responder SSE do Axum e devolve o header Keep-Alive.
Sse::new(final_stream)
    .keep_alive(axum::response::sse::KeepAlive::new())
    .into_response()
}

/// Spawns the Sovereign Pair Desktop GUI (Tauri App) process natively from the OS.
pub async fn launch_gui_handler() -> impl IntoResponse {
    tracing::info!("🚀 [Sovereign Core] Invocação de GUI Recebida! Spawning Sovereign Tauri Desktop...");
    // Tenta spawnar o binário instalado globalmente no path do Linux (.deb / pacman)
    if std::process::Command::new("sovereign-pair").spawn().is_err() {
        // Fallback robusto para o ambiente de compilação de desenvolvimento local
        let dev_bin_path = "/home/jefersonlopes/Developer/local-repositories/sovereign-pair/svelte-ui/src-tauri/target/release/sovereign-pair";
        let _ = std::process::Command::new(dev_bin_path).spawn();
    }
    axum::response::Json(serde_json::json!({ "status": "gui_dispatched" }))
}

pub async fn telemetry_snapshot_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let snapshot = match state.telemetry.write() {
        Ok(mut t) => {
            t.refresh_hardware();
            t.get_snapshot()
        },
        Err(_) => crate::telemetry::TelemetrySnapshot {
            total_tokens: 0,
            avg_tps: 0.0,
            avg_latency_ms: 0,
            estimated_cost: 0.0,
            models_usage: std::collections::HashMap::new(),
            hardware: crate::telemetry::HardwareSnapshot {
                cpu_cores: vec![],
                ram_usage_mb: 0.0,
                ram_total_gb: 24.0,
                io_rx_bytes: 0,
                io_tx_bytes: 0,
                gpu_name: "GPU Compute".to_string(),
                gpu_vram_total_mb: 0,
            }
        },
    };

    let security_logs = sqlx::query("SELECT event_type, severity, blocked, message, source, strftime('%Y-%m-%d %H:%M:%S', created_at) as created_at FROM security_logs ORDER BY id DESC LIMIT 5")
        .fetch_all(&state.db)
        .await
        .map(|rows| {
            rows.into_iter().map(|row| {
                serde_json::json!({
                    "event_type": sqlx::Row::get::<String, _>(&row, "event_type"),
                    "severity": sqlx::Row::get::<String, _>(&row, "severity"),
                    "blocked": sqlx::Row::get::<bool, _>(&row, "blocked"),
                    "message": sqlx::Row::get::<String, _>(&row, "message"),
                    "source": sqlx::Row::get::<String, _>(&row, "source"),
                    "created_at": sqlx::Row::get::<String, _>(&row, "created_at"),
                })
            }).collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let security_blocks_count = sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM security_logs")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let trackers_blocked_count = sqlx::query_scalar::<_, i32>("SELECT val_int FROM analytics WHERE id = 'total_trackers_blocked'")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

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

    let vaults_count = sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM workspaces").fetch_one(&state.db).await.unwrap_or(1);
    let synced_files = sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM sensus_documents").fetch_one(&state.db).await.unwrap_or(0);
    
    let mut vault_categories = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&state.vault_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type()
                && file_type.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.starts_with('.') {
                        vault_categories.push(name);
                    }
                }
        }
    }

    let historical_models = sqlx::query("SELECT model_name, total_tokens, total_duration_ms, strftime('%Y-%m-%d %H:%M:%S', first_used_at) as first_used_at, strftime('%Y-%m-%d %H:%M:%S', last_used_at) as last_used_at FROM model_metrics ORDER BY total_tokens DESC")
        .fetch_all(&state.db)
        .await
        .map(|rows| {
            rows.into_iter().map(|row| {
                serde_json::json!({
                    "model_name": sqlx::Row::get::<String, _>(&row, "model_name"),
                    "total_tokens": sqlx::Row::get::<i64, _>(&row, "total_tokens"),
                    "total_duration_ms": sqlx::Row::get::<i64, _>(&row, "total_duration_ms"),
                    "first_used_at": sqlx::Row::get::<String, _>(&row, "first_used_at"),
                    "last_used_at": sqlx::Row::get::<String, _>(&row, "last_used_at"),
                })
            }).collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let content_gaps = sqlx::query("SELECT query, context, frequency, status FROM knowledge_gaps ORDER BY frequency DESC LIMIT 5")
        .fetch_all(&state.db)
        .await
        .map(|rows| {
            rows.into_iter().map(|row| {
                serde_json::json!({
                    "query": sqlx::Row::get::<String, _>(&row, "query"),
                    "context": sqlx::Row::get::<String, _>(&row, "context"),
                    "frequency": sqlx::Row::get::<i32, _>(&row, "frequency"),
                    "status": sqlx::Row::get::<String, _>(&row, "status"),
                })
            }).collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut topic_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    if let Ok(sessions) = sqlx::query("SELECT tags_json FROM chat_sessions WHERE tags_json IS NOT NULL").fetch_all(&state.db).await {
        for row in sessions {
            if let Ok(tags_str) = sqlx::Row::try_get::<String, _>(&row, "tags_json")
                && let Ok(tags) = serde_json::from_str::<Vec<String>>(&tags_str) {
                    for tag in tags {
                        *topic_counts.entry(tag).or_insert(0) += 1;
                    }
                }
        }
    }
    let mut top_topics: Vec<_> = topic_counts.into_iter().map(|(topic, count)| serde_json::json!({ "topic": topic, "count": count })).collect();
    top_topics.sort_by(|a, b| b["count"].as_i64().unwrap_or(0).cmp(&a["count"].as_i64().unwrap_or(0)));
    top_topics.truncate(5);

    // Devolve formatado para o Dashboard Svelte
    Json(serde_json::json!({
        "total_tokens": snapshot.total_tokens,
        "avg_tps": snapshot.avg_tps,
        "avg_latency_ms": snapshot.avg_latency_ms,
        "estimated_cost": snapshot.estimated_cost,
        "models_usage": snapshot.models_usage,
        "historical_models": historical_models,
        "content_gaps": content_gaps,
        "top_topics": top_topics,
        "active_models": snapshot.models_usage.keys().len(), 
        "security_blocks": security_blocks_count,
        "trackers_blocked": trackers_blocked_count,
        "security_logs": security_logs,
        "hardware": {
            "cpu_cores": snapshot.hardware.cpu_cores,
            "ram": snapshot.hardware.ram_usage_mb,
            "ram_total_gb": snapshot.hardware.ram_total_gb,
            "io_rx": snapshot.hardware.io_rx_bytes,
            "io_tx": snapshot.hardware.io_tx_bytes,
            "gpu_name": snapshot.hardware.gpu_name,
            "gpu_vram_total_mb": snapshot.hardware.gpu_vram_total_mb
        },
        "cronos": {
            "tasks_today": pending_tasks,
            "progress": progress,
            "vaults_count": vaults_count,
            "synced_files": synced_files,
            "vault_categories": vault_categories
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
#[allow(dead_code)]
pub struct VaultGraphQuery {
    pub workspace_id: Option<String>,
}

/// Sensus Document Topology Builder (Vault Hub & Dashboard Graph Engine)
#[allow(dead_code)]
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