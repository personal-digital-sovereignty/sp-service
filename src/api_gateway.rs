use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct PublicApiEntry {
    pub name: String,
    pub url: String,
    pub description: Option<String>,
    pub auth: Option<String>,
    pub https: Option<String>,
    pub cors: Option<String>,
    pub category: Option<String>,
}

pub async fn search_api_directory(query: &str, pool: &sqlx::SqlitePool) -> String {
    let q = format!("%{}%", query.to_lowercase());
    
    // Search SQLite for matching APIs
    let matches: Result<Vec<PublicApiEntry>, sqlx::Error> = sqlx::query_as::<_, PublicApiEntry>(
        r#"
        SELECT name, url, description, auth, https, cors, category 
        FROM public_api_directory 
        WHERE LOWER(name) LIKE ? OR LOWER(description) LIKE ? OR LOWER(category) LIKE ? 
        LIMIT 20
        "#,
    )
    .bind(&q).bind(&q).bind(&q)
    .fetch_all(pool)
    .await;

    match matches {
        Ok(apis) if apis.is_empty() => {
            format!("Nenhuma API pública aberta sob o termo '{}' foi encontrada na lista autorizada do Sovereign.", query)
        },
        Ok(apis) => {
            let mut result = format!("### Catálogo de APIs Free Resgatado (Termo: '{}')\n\nVocê tem autorização Tática para acessar estas URLs diretamente ao invés de buscar no Google!\n\n", query);
            for api in apis {
                let desc = api.description.clone().unwrap_or_else(|| "Sem descrição".to_string());
                let auth = api.auth.clone().unwrap_or_else(|| "Unknown".to_string());
                let cors = api.cors.clone().unwrap_or_else(|| "Unknown".to_string());
                let cat = api.category.clone().unwrap_or_else(|| "Unknown".to_string());
                
                result.push_str(&format!("- **{}** (Categoria: {})\n  Descrição: {}\n  Auth Necessária: {} | CORS: {}\n  Endpoint Básico Autorizado para uso: `{}`\n", 
                    api.name, cat, desc, auth, cors, api.url));
            }
            result
        },
        Err(e) => {
            format!("[ERRO] Falha ao consultar o banco de dados public_api_directory: {}", e)
        }
    }
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
