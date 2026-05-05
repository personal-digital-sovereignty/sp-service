use crate::models::{LogEntry, PlanExecuteBlueprint};
use serde_json::json;
use tracing::error;

pub async fn start_plan_and_execute(
    query: String,
    state: std::sync::Arc<crate::AppState>,
    model: String,
) {
    let log_sender = state.log_sender.clone();
    let db = state.db.clone();
    let _ = log_sender.send(LogEntry {
        timestamp: chrono::Local::now().to_rfc3339(),
        level: "agent".to_string(),
        message: format!("🧠 [Plan & Execute] Inicializando Macro-Orquestração para: '{}'", query),
    });

    let client = reqwest::Client::new();
    let ollama_url = format!("{}{}", std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()), "/api/chat");

    // Fase 1: O Planner (Gerar o JSON Determinístico)
    let planner_system = r#"Você é o Agente Planejador Híbrido. 
Seu dever é quebrar solicitações gigantes do usuário em pequenas sub-tarefas autônomas.
RESPOSTA MANDATÓRIA: DEVE ser EXATAMENTE um JSON na estrutura:
{
  "plan": [
    { "task": "Ler arquivo X", "action": "extract" },
    { "task": "Resumir arquivo X", "action": "summarize" }
  ]
}
VOCÊ NÃO PODE RESPONDER NADA ALÉM DO JSON.
"#;

    let plan_payload = json!({
        "model": model.clone(),
        "messages": [
            { "role": "system", "content": planner_system },
            { "role": "user", "content": format!("Desmanche a seguinte meta em no máximo 3 etapas: {}", query) }
        ],
        "format": "json", // Strict JSON Chaining
        "stream": false,
        "options": {
            "temperature": 0.1 // Determinismo
        }
    });

    let _ = log_sender.send(LogEntry {
        timestamp: chrono::Local::now().to_rfc3339(),
        level: "system".to_string(),
        message: "⏳ Solicitando ao LLM a quebra da tarefa raiz em Grafo JSON (Strict Pattern)...".to_string(),
    });

    let plan_response = match client.post(&ollama_url).json(&plan_payload).send().await {
        Ok(res) if res.status().is_success() => res,
        _ => {
            error!("🚨 Falha ao contactar Ollama na fase de Planejamento");
            return;
        }
    };

    let plan_json_res = plan_response.json::<serde_json::Value>().await;
    if let Ok(json_body) = plan_json_res {
        if let Some(msg) = json_body.get("message").and_then(|m| m.get("content").and_then(|c| c.as_str())) {
            
            // Tenta deserializar o JSON purificado na strict Struct do Rust (Prompt Chaining)
            match serde_json::from_str::<PlanExecuteBlueprint>(msg) {
                Ok(blueprint) => {
                    let _ = log_sender.send(LogEntry {
                        timestamp: chrono::Local::now().to_rfc3339(),
                        level: "agent".to_string(),
                        message: format!("✅ Plano Tático Gerado com Sucesso: {} steps detectados.", blueprint.plan.len()),
                    });

                    // Loop de Execução
                    let mut aggregated_results = String::new();

                    for (i, step) in blueprint.plan.iter().enumerate() {
                        let _ = log_sender.send(LogEntry {
                            timestamp: chrono::Local::now().to_rfc3339(),
                            level: "agent".to_string(),
                            message: format!("⚙️ Executando Step {}/{} [{}] -> {}", i + 1, blueprint.plan.len(), step.action, step.task),
                        });

                        // Fase 2: O Executor (Roda cada step e junta no contexto)
                        let executor_payload = json!({
                            "model": model.clone(),
                            "messages": [
                                { "role": "system", "content": "Você é o Agente Executor Cíbrido. Realize a ação imperativamente. Responda APENAS com o resultado puro da ação. Se precisar ler ou escrever arquivos no projeto local, use as TOOLS disponibilizadas pelo protocolo MCP." },
                                { "role": "user", "content": format!("Contexto Anterior Acumulado:\n{}\n\nAção a tomar AGORA: ({}) -> {}", aggregated_results, step.action, step.task) }
                            ],
                            "tools": crate::mcp::get_mcp_tools(),
                            "stream": false
                        });

                        if let Ok(exec_res) = client.post(&ollama_url).json(&executor_payload).send().await {
                            if let Ok(exec_json) = exec_res.json::<serde_json::Value>().await {
                                if let Some(message) = exec_json.get("message") {
                                    // Se o LLM Cuspiu uma ToolCall MCP Autônoma:
                                    if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
                                        for tc in tool_calls {
                                            if let Some(func) = tc.get("function") {
                                                let name = func.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                                let arguments = func.get("arguments").cloned().unwrap_or(json!({}));
                                                
                                                let _ = log_sender.send(LogEntry {
                                                    timestamp: chrono::Local::now().to_rfc3339(),
                                                    level: "mcp".to_string(), // Especial p/ Frontend
                                                    message: format!("🔓 Autorização MCP Acionada: Agente invocou ferramenta [{}]", name),
                                                });

                                                let mcp_result = crate::mcp::execute_mcp_tool(&state, name, &arguments).await;
                                                aggregated_results.push_str(&format!("\n[Ação de FileSystem MCP do Step {}]: {}\n", i + 1, mcp_result));
                                            }
                                        }
                                    } 
                                    // Se cuspiu apenas Text Content
                                    else if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                                        aggregated_results.push_str(&format!("\n[Conclusão Semântica do Step {}]:\n{}\n", i + 1, content));
                                    }
                                }
                    }

                    let _ = log_sender.send(LogEntry {
                        timestamp: chrono::Local::now().to_rfc3339(),
                        level: "agent".to_string(),
                        message: "🏁 Todo o pipeline Plan & Execute foi resolvido. A Orquestração Macro finalizou o Job!".to_string(),
                    });

                    // Fase 3: Gravação final no DB
                    let final_blob = format!("Plan & Execute Resolvido Automaticamente:\n{}", aggregated_results);
                    crate::api_chat::save_message(&db, 1, "assistant", &final_blob).await;

                },
                Err(e) => {
                    let _ = log_sender.send(LogEntry {
                        timestamp: chrono::Local::now().to_rfc3339(),
                        level: "error".to_string(),
                        message: format!("❌ O LLM quebrou o Chain JSON! Deserialização falhou: {}", e),
                    });
                }
            }
        }
}
