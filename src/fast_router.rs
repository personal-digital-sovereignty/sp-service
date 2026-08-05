use crate::AppState;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

/// Obtém o modelo Roteador Base configurado (default: qwen3:0.6b)
pub async fn get_router_model(db: &sqlx::SqlitePool) -> String {
    let row_opt = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'sovereign_router'")
        .fetch_optional(db)
        .await
        .ok()
        .flatten();

    if let Some(row) = row_opt {
        let val: String = sqlx::Row::get(&row, "value_json");
        if let Ok(parsed) = serde_json::from_str::<Value>(&val) {
            if let Some(model) = parsed.get("model").and_then(|v| v.as_str()) {
                if !model.is_empty() {
                    return model.to_string();
                }
            }
        }
    }

    // Padrão de Inicialização se não existir
    let default_model = "qwen3:0.6b".to_string();
    let payload = json!({ "model": &default_model });
    let _ = sqlx::query("INSERT INTO global_settings (id, value_json) VALUES ('sovereign_router', ?) ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json")
        .bind(payload.to_string())
        .execute(db)
        .await;

    default_model
}

/// Valida se o roteador base está no Ollama e faz o pull bloqueante caso não esteja
pub async fn verify_and_pull_router(state: Arc<AppState>) {
    let router_model = get_router_model(&state.db).await;
    let ollama_url = crate::api_models::resolve_ollama_url(&state).await;

    info!("🚀 [Fast-Router] Validando presença do Sovereign Router: {}", router_model);

    let tags_url = format!("{}/api/tags", ollama_url);
    let mut exists = false;

    if let Ok(res) = state.http_client.get(&tags_url).send().await {
        if let Ok(json) = res.json::<Value>().await {
            if let Some(models) = json.get("models").and_then(|v| v.as_array()) {
                for m in models {
                    if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                        let base_name = if router_model.contains(':') {
                            router_model.clone()
                        } else {
                            format!("{}:latest", router_model)
                        };
                        if name == router_model || name == base_name {
                            exists = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    if !exists {
        info!("⏳ [Fast-Router] Modelo {} não encontrado localmente. Iniciando pull...", router_model);
        let _ = state.log_sender.send(crate::models::LogEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            level: "system".to_string(),
            message: format!("⚙️ Sovereign Fast-Router ({}): Baixando modelo. Pode demorar alguns minutos no primeiro boot...", router_model),
        });

        let pull_url = format!("{}/api/pull", ollama_url);
        let payload = json!({
            "name": router_model,
            "stream": false
        });

        // 30 min timeout to allow large SLM pulls on slow connections
        let pull_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1800))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
            
        if let Ok(res) = pull_client.post(&pull_url).json(&payload).send().await {
            if res.status().is_success() {
                let _ = state.log_sender.send(crate::models::LogEntry {
                    timestamp: chrono::Local::now().to_rfc3339(),
                    level: "system".to_string(),
                    message: format!("✅ Sovereign Fast-Router ({}) baixado e ativado com sucesso!", router_model),
                });
                info!("✅ [Fast-Router] Modelo roteador {} baixado com sucesso.", router_model);
            } else {
                error!("❌ [Fast-Router] Falha ao baixar o modelo roteador: {:?}", res.status());
            }
        }
    } else {
        info!("✅ [Fast-Router] Modelo roteador {} detectado e pronto para governança.", router_model);
    }
}

pub struct RoutedModels {
    pub p_model: String,
    pub m_model: String,
    pub g_model: String,
}

/// Escaneia o hardware local e os modelos do Ollama para formar a trindade P,M,G
pub async fn discover_local_routing_map(state: &AppState) -> RoutedModels {
    let router_model = get_router_model(&state.db).await;
    let mut routed = RoutedModels {
        p_model: router_model.clone(),
        m_model: router_model.clone(),
        g_model: router_model.clone(),
    };

    let hw = crate::hardware::capture_hardware_telemetry();
    let governing_ram = if hw.total_vram_gb > 0.0 { hw.total_vram_gb } else { hw.total_ram_gb };

    let ollama_url = crate::api_models::resolve_ollama_url(state).await;
    let mut local_models: Vec<(String, u64)> = Vec::new();

    if let Ok(res) = state.http_client.get(format!("{}/api/tags", ollama_url)).send().await {
        if let Ok(json) = res.json::<Value>().await {
            if let Some(models) = json.get("models").and_then(|v| v.as_array()) {
                for m in models {
                    if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                        let size = m.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                        local_models.push((name.to_string(), size));
                    }
                }
            }
        }
    }

    if local_models.is_empty() {
        return routed;
    }

    // Ordena do menor para o maior peso
    local_models.sort_by(|a, b| a.1.cmp(&b.1));

    let giga = 1024 * 1024 * 1024;
    let mut best_m = None;
    let mut best_g = None;

    for (name, size_bytes) in &local_models {
        if name == &router_model { continue; }

        let size_gb = *size_bytes as f64 / giga as f64;

        if size_gb > 1.5 && size_gb <= 5.5 {
            // Entre 3B e 8B
            best_m = Some(name.clone());
        } else if size_gb > 5.5 && size_gb <= 15.0 {
            // Modelos 9B até 14B
            if governing_ram >= 12.0 {
                best_g = Some(name.clone());
            } else {
                best_m = Some(name.clone()); // Rebaixa para M se não tiver RAM pra suportar G
            }
        } else if size_gb > 15.0 {
            // Modelos pesados 32B+
            if governing_ram >= 24.0 {
                best_g = Some(name.clone());
            }
        }
    }

    // Safefall para OOMs duros
    if governing_ram < 8.0 {
        best_g = best_m.clone();
    }
    if governing_ram < 4.0 {
        best_m = Some(router_model.clone());
        best_g = Some(router_model.clone());
    }

    routed.m_model = best_m.unwrap_or_else(|| router_model.clone());
    routed.g_model = best_g.unwrap_or_else(|| routed.m_model.clone());

    routed
}

/// Avalia a complexidade do prompt através do Router Model local (Sovereign Fast-Router)
pub async fn evaluate_complexity(prompt: &str, state: &AppState) -> String {
    let router_model = get_router_model(&state.db).await;
    let ollama_url = crate::api_models::resolve_ollama_url(state).await;
    let endpoint = format!("{}/api/chat", ollama_url);

    let sys_prompt = "Você é um classificador estrutural de tarefas de IA. 
Sua única função é analisar o texto do usuário e determinar a complexidade computacional para respondê-lo.
Regras:
1. Responda 'P' (Pequeno) para conversas simples, cumprimentos, perguntas triviais de conhecimento geral ou pedidos muito curtos.
2. Responda 'M' (Médio) para perguntas que exigem raciocínio, escrita de código básica, redação de e-mails ou explicações de nível médio.
3. Responda 'G' (Grande) para geração de código complexo, reescrita de longos documentos, análises profundas ou prompts gigantes (RAG).
RESPOSTA OBRIGATÓRIA: Responda APENAS com a letra P, M ou G. NENHUMA OUTRA PALAVRA.";

    let payload = json!({
        "model": router_model,
        "messages": [
            { "role": "system", "content": sys_prompt },
            { "role": "user", "content": prompt }
        ],
        "stream": false,
        "keep_alive": "-1", // Garante que o router sempre viva na memória
        "options": {
            "temperature": 0.1,
            "max_tokens": 10
        }
    });

    let mut result_category = "M".to_string(); // Fallback neutro

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    if let Ok(res) = client.post(&endpoint).json(&payload).send().await {
        if let Ok(json) = res.json::<Value>().await {
            if let Some(msg) = json.get("message").and_then(|m| m.get("content").and_then(|c| c.as_str())) {
                let clean = msg.trim().to_uppercase();
                if clean.contains('P') { result_category = "P".to_string(); }
                else if clean.contains('G') { result_category = "G".to_string(); }
                else if clean.contains('M') { result_category = "M".to_string(); }
            }
        }
    }

    result_category
}
