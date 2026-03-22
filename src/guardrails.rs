use regex::Regex;
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
pub fn evaluate_prompt(prompt: &str) -> Result<(), SecurityEvent> {
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
    if let Ok(cpf_regex) = regex::Regex::new(r"\b\d{3}\.\d{3}\.\d{3}-\d{2}\b") {
        if cpf_regex.is_match(prompt) {
            return Err(SecurityEvent {
                event_type: "PII Detected CPF".to_string(),
                severity: "High".to_string(),
                blocked: true,
                message: "Sensitive PII (CPF) transmission blocked by DLP.".to_string(),
                source: "Sovereign Guardrails".to_string(),
            });
        }
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
