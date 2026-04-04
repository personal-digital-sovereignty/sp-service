use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicApiEntry {
    pub name: String,
    pub url: String,
    pub description: String,
    pub auth: String,
    pub https: String,
    pub cors: String,
    pub category: String,
}

// Embedded Base64 Payload compiled into the executable directly via build.rs
const API_B64: &str = include_str!("public_apis.b64");

lazy_static::lazy_static! {
    pub static ref PUBLIC_APIS: Vec<PublicApiEntry> = {
        use base64::{Engine as _, engine::general_purpose};
        let decoded = general_purpose::STANDARD.decode(API_B64.trim()).unwrap_or_default();
        serde_json::from_slice(&decoded).unwrap_or_else(|_| Vec::new())
    };
}

pub fn search_api_directory(query: &str) -> String {
    let q = query.to_lowercase();
    let matches: Vec<&PublicApiEntry> = PUBLIC_APIS.iter()
        .filter(|api| api.name.to_lowercase().contains(&q) || 
                      api.description.to_lowercase().contains(&q) ||
                      api.category.to_lowercase().contains(&q))
        .take(20) // Limit to 20 results to avoid Context Window Overflow
        .collect();

    if matches.is_empty() {
        return format!("Nenhuma API pública aberta sob o termo '{}' foi encontrada na lista autorizada do Sovereign.", query);
    }

    let mut result = format!("### Catálogo de APIs Free Resgatado (Termo: '{}')\n\nVocê tem autorização Tática para acessar estas URLs diretamente ao invés de buscar no Google!\n\n", query);
    for api in matches {
        result.push_str(&format!("- **{}** (Categoria: {})\n  Descrição: {}\n  Auth Necessária: {} | CORS: {}\n  Endpoint Básico Autorizado para uso: `{}`\n", 
            api.name, api.category, api.description, api.auth, api.cors, api.url));
    }
    result
}

pub async fn fetch_json_endpoint(url: &str) -> String {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Sovereign-Pair-Cognitive-Router/1.0")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
        
    match client.get(url).send().await {
        Ok(res) => {
            let status = res.status();
            if let Ok(text) = res.text().await {
                // Abort synthesis if we hit an HTML page masquerading as an API
                if text.trim_start().starts_with("<!DOCTYPE") || text.trim_start().starts_with("<html") {
                    return format!("[ERRO TÁTICO] A URL '{}' não devolveu um JSON consumível, e sim uma página web (HTML Blockado). Use o RAG Web Scraper (`dispatch_sub_researcher`) ou refine a URL da API.", url);
                }
                
                // Truncate string to avoid filling the whole Master LLM Context Window
                let max_len = 10000;
                let truncated = if text.len() > max_len {
                    format!("{}... [TRUNCADO POR OVERHEAD]", &text[..max_len])
                } else {
                    text
                };
                
                format!("[Status Code: {}] Retorno Cru do Endpoint ({}) para Mapeamento Cognitivo:\n```json\n{}\n```", status, url, truncated)
            } else {
                "[ERRO] A API estourou falha silenciosa na leitura do Body.".to_string()
            }
        },
        Err(e) => format!("[ERRO] A conexão tática Sovereign Gateway com a API falhou: {}", e)
    }
}
