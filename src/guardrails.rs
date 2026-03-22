use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityEvent {
    pub event_type: String,
    pub severity: String,
    pub blocked: bool,
    pub message: String,
    pub source: String,
}

/// Verifica um prompt do usuário contra políticas de segurança.
/// Retorna `Err(SecurityEvent)` se uma violação agressiva for identificada.
use sqlx::Row;

pub async fn evaluate_prompt(prompt: &str, db: &sqlx::SqlitePool) -> Result<(), SecurityEvent> {
    // DB-Driven Custom Guardrails
    if let Ok(Some(row)) = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'system_settings'").fetch_optional(db).await {
        let val: String = row.get("value_json");
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val)
            && let Some(guardrails) = parsed.get("guardrails").and_then(|v| v.as_array()) {
                for rule in guardrails {
                    let rule_type = rule.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let rule_value = rule.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = rule.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    
                    if rule_value.is_empty() { continue; }

                    if rule_type == "keyword" {
                        if prompt.to_lowercase().contains(&rule_value.to_lowercase()) {
                            return Err(SecurityEvent {
                                event_type: "Keyword Blocked".to_string(),
                                severity: "Critical".to_string(),
                                blocked: true,
                                message: format!("Custom rule '{}' blocked this interaction.", desc),
                                source: "DevSecOps".to_string()
                            });
                        }
                    } else if rule_type == "regex"
                        && let Ok(re) = regex::Regex::new(rule_value)
                            && re.is_match(prompt) {
                                return Err(SecurityEvent {
                                    event_type: "Regex Blocked".to_string(),
                                    severity: "Critical".to_string(),
                                    blocked: true,
                                    message: format!("Custom regex '{}' triggered.", desc),
                                    source: "DevSecOps".to_string()
                                });
                            }
                }
            }
    }

    let prompt_lower = prompt.to_lowercase();

    // 1. Jailbreak / Prompt Injection Patterns
    let injection_patterns = [
        "ignore all previous instructions",
        "ignore previous rules",
        "forget everything",
        "you are now a",
        "bypass rules",
        "system prompt",
        "do anything now",
        "disregard above instructions"
    ];

    for pattern in injection_patterns.iter() {
        if prompt_lower.contains(pattern) {
            return Err(SecurityEvent {
                event_type: "SEC Prompt Injection".to_string(),
                severity: "Critical".to_string(),
                blocked: true,
                message: format!("Jailbreak pattern '{}' detected.", pattern),
                source: "Sovereign Guardrails".to_string(),
            });
        }
    }

    // 2. PII Detection (CPF / SSN / Credenciais)
    // Regex simples para capturar formato de CPF em texto pt-br ou chaves genéricas AWS/Tokens.
    if let Ok(cpf_regex) = regex::Regex::new(r"\b\d{3}\.\d{3}\.\d{3}-\d{2}\b")
        && cpf_regex.is_match(prompt) {
            return Err(SecurityEvent {
                event_type: "PII Detected CPF".to_string(),
                severity: "High".to_string(),
                blocked: true,
                message: "Sensitive PII (CPF) transmission blocked by DLP.".to_string(),
                source: "Sovereign Guardrails".to_string(),
            });
        }
    
    // 3. Toxicity (Heurísticas Simples de Violação de Compliance Exemplo)
    let toxic_words = [
        "hack into",
        "how to build a bomb",
        "bypass password",
        "sql injection for",
    ];

    for toxic in toxic_words.iter() {
        if prompt_lower.contains(toxic) {
            return Err(SecurityEvent {
                event_type: "Toxicity Check".to_string(),
                severity: "High".to_string(),
                blocked: true,
                message: format!("Malicious intent pattern '{}' flagged.", toxic),
                source: "Sovereign Guardrails".to_string(),
            });
        }
    }

    Ok(())
}
