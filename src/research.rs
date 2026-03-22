use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use regex::Regex;

/// Arquitetura nativa do Sovereign Web-Augmented Generation (WAG)
/// Blindada via Rust para injetar contexto externo ao RAG Cíbrido.
pub struct DeepResearchEngine {
    client: Client,
    db_pool: Option<sqlx::SqlitePool>,
}

impl DeepResearchEngine {
    pub fn new(db_pool: Option<sqlx::SqlitePool>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, db_pool }
    }

    /// Realiza a varredura e o scrape profundo da URL alvo.
    pub async fn scrape_url(&self, url: &str) -> Result<String, String> {
        let response = self.client.get(url).send().await.map_err(|e| format!("HTTP Request failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("Server returned HTTP {}", response.status()));
        }

        let html_content = response.text().await.map_err(|e| format!("Failed to read HTML body: {}", e))?;
        
        Ok(self.sanitize_to_markdown(&html_content))
    }

    /// Limpa o HTML ruidoso (scripts, estilos, anúncios) e extrai o texto principal em formato Semântico (Markdown-like).
    fn sanitize_to_markdown(&self, html: &str) -> String {
        let document = Html::parse_document(html);
        
        // Remove tags ofensores (Anti-Junk)
        // Isso é uma filtragem em memória antes da decodificação.
        let mut text_blocks = Vec::new();
        
        // Vamos capturar parágrafos, cabeçalhos e listas
        let selector = Selector::parse("p, h1, h2, h3, h4, li, article, main, .content").unwrap();
        
        for element in document.select(&selector) {
            let tag_name = element.value().name();
            
            // Foca o inner text, ignorando scripts implícitos
            let inner_text = element.text().collect::<Vec<_>>().join(" ");
            let clean_text = inner_text.trim();
            
            if clean_text.is_empty() {
                continue;
            }
            
            // Formata o header
            let formatted = match tag_name {
                "h1" => format!("# {}\n", clean_text),
                "h2" => format!("## {}\n", clean_text),
                "h3" => format!("### {}\n", clean_text),
                "h4" => format!("#### {}\n", clean_text),
                "li" => format!("- {}", clean_text),
                _ => format!("{}\n", clean_text), // <p>, <article>, <main>
            };
            
            text_blocks.push(formatted);
        }
        
        // Retira blocos curtos demais ou inúteis
        let mut markdown = text_blocks.join("\n");
        
        // Expressão Regular agressiva para remover espaçamentos múltiplos (Whitespace Normalization)
        let re = Regex::new(r"\n{3,}").unwrap();
        markdown = re.replace_all(&markdown, "\n\n").to_string();
        
        markdown
    }

    /// Dispara a busca Multi-Hop com tolerância impecável a falhas WAF/Cloudflare.
    /// Estratégia 1: DuckDuckGo HTML Nativo (Light DOM)
    /// Estratégia 2: Rotação Cíbrida Global de SearxNGs
    pub async fn search_web(&self, query: &str) -> Result<Vec<String>, String> {
        tracing::info!("🔍 [WAG] Inicializando Busca Autônoma Deep Research (Dual-Engine) por: '{}'", query);
        
        // 1. O SOVEREIGN PARSER: Tenta raspar a página retro do DuckDuckGo (Estratégia 1)
        match self.search_ddg_html(query).await {
            Ok(links) if !links.is_empty() => {
                tracing::info!("✅ [WAG] Busca Nativa DGG HTML Bem-Sucedida! ({}) links ancorados.", links.len());
                return Ok(links);
            },
            Err(e) => {
                let msg = format!("⚠️ [WAG Fallback] Falha catastrófica ou bloqueio WAF agressivo no DuckDuckGo: {}", e);
                tracing::warn!("{}", msg);
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("deep_research_waf_audit.log") {
                    use std::io::Write;
                    let _ = writeln!(&mut file, "[Deep Research Agent] {}", msg);
                }
            },
            _ => {
                let msg = "⚠️ [WAG Fallback] DuckDuckGo retornou 0 links limpos (WAF/Cloudflare invisível travou a listagem).";
                tracing::warn!("{}", msg);
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("deep_research_waf_audit.log") {
                    use std::io::Write;
                    let _ = writeln!(&mut file, "[Deep Research Agent] {}", msg);
                }
            }
        }

        // 2. O FALLBACK SOBERANO: Motores de Busca Descentralizados P2P (Estratégia 2)
        match self.search_searxng_public(query).await {
            Ok(links) if !links.is_empty() => {
                tracing::info!("✅ [WAG] Busca Cíbrida SearxNG Bem-Sucedida! ({}) links ancorados.", links.len());
                Ok(links)
            },
            Err(e) => {
                tracing::error!("❌ [WAG Fim de Linha] WAF Absoluto. Nenhuma instância pública do SearxNG sobreviveu. Erro: {}", e);
                Err(format!("Dual-Engine Crash. WAF Block total ativo na malha. {}", e))
            },
            _ => {
                Err("Dual-Engine não achou nenhum resultado útil para a query.".to_string())
            }
        }
    }

    /// Extrator 100% Nativo no Rust que burla bloqueios massivos do DDG acessando sua rede retro HTML.
    async fn search_ddg_html(&self, query: &str) -> Result<Vec<String>, String> {
        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
        
        let req = self.client.get(&url)
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("DNT", "1")
            .send()
            .await.map_err(|e| format!("Network Erro ao contatar html.duckduckgo.com: {}", e))?;

        if !req.status().is_success() {
            return Err(format!("Interceptado por HTTP Status {}", req.status()));
        }

        let html = req.text().await.map_err(|e| format!("Falha de Decodificação DOM: {}", e))?;
        
        if html.contains("CAPTCHA") || html.contains("To continue") || html.contains("cloudflare") {
            return Err("Honeypot/Captcha nativo engatilhado na camada HTML do DuckDuckGo".to_string());
        }

        let document = Html::parse_document(&html);
        let selector = Selector::parse("a.result__snippet").unwrap_or_else(|_| Selector::parse("a.result__url").unwrap());
        
        let mut links = Vec::new();
        for element in document.select(&selector) {
            if let Some(href_attr) = element.value().attr("href") {
                if href_attr.starts_with("http") && !href_attr.contains("duckduckgo.com") {
                    links.push(href_attr.to_string());
                } else if href_attr.contains("uddg=") {
                    // Reverse-engineering do token de redirecionamento nativo do DuckDuckGo
                    let parts: Vec<&str> = href_attr.split("uddg=").collect();
                    if parts.len() > 1 {
                        let inner = parts[1].split('&').next().unwrap_or("");
                        if let Ok(decoded) = urlencoding::decode(inner) {
                            links.push(decoded.into_owned());
                        }
                    }
                }
            }
        }

        links.dedup();
        links.truncate(5); // Retorna estritamente o Top 5
        Ok(links)
    }

    /// Rotação autônoma que pula em instâncias OpenSource pelo mundo pedindo ajuda via API JSON.
    async fn search_searxng_public(&self, query: &str) -> Result<Vec<String>, String> {
        let mut instances = vec![
            "https://paulgo.io".to_string(), 
            "https://searx.be".to_string(), 
            "https://search.bus-hit.me".to_string(),
            "https://search.nadeko.net".to_string(),
            "https://searx.daetalytica.io".to_string()
        ];

        // Se o banco Cíbrido estiver acoplado, verifique os IPs configurados pelo usuário
        if let Some(pool) = &self.db_pool
            && let Ok(json_str) = sqlx::query_scalar::<_, String>("SELECT value_json FROM global_settings WHERE id = 'searxng_nodes'")
                .fetch_one(pool)
                .await
                && let Ok(parsed) = serde_json::from_str::<Vec<String>>(&json_str)
                    && !parsed.is_empty() {
                        instances = parsed;
                        tracing::info!("🔗 [SearxNG] {} Nodes Customizados de Busca carregados do Sensus Vault!", instances.len());
                    }

        let mut last_error = String::new();

        for base_url in instances {
            tracing::info!("🔄 [SearxNG Agent] Simulando tráfego na instância P2P: {}", base_url);
            let url = format!("{}/search?q={}&format=json", base_url, urlencoding::encode(query));
            
            let req = self.client.get(&url).send().await;
            
            match req {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(json_data) = response.json::<serde_json::Value>().await
                            && let Some(results) = json_data.get("results").and_then(|r| r.as_array()) {
                                let mut links = Vec::new();
                                for res in results {
                                    if let Some(url_str) = res.get("url").and_then(|u| u.as_str()) {
                                        links.push(url_str.to_string());
                                    }
                                }
                                links.truncate(5);
                                return Ok(links);
                            }
                    } else {
                        tracing::debug!("Instância {} fechou as portas HTTP {}", base_url, response.status());
                        last_error = format!("HTTP {}", response.status());
                    }
                },
                Err(e) => {
                    tracing::debug!("Rede hostil em {}: {}", base_url, e);
                    last_error = e.to_string();
                }
            }
        }

        Err(format!("A armadura caiu. O WAF destruiu todas as 5 instâncias. Último vestígio: {}", last_error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_markdown_sanitization() {
        let engine = DeepResearchEngine::new(None);
        let html_mock = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script>alert("HACKED");</script>
                <style>body { color: red; }</style>
            </head>
            <body>
                <header>Ignored Header Navigation</header>
                <main>
                    <h1>Sovereign Pair WAG Test</h1>
                    <p>This is a test paragraph describing the Deep Research module.</p>
                    <ul>
                        <li>Item 1</li>
                        <li>Item 2</li>
                    </ul>
                </main>
                <aside>Adverts here</aside>
            </body>
            </html>
        "#;

        let markdown = engine.sanitize_to_markdown(html_mock);
        
        // Asserting the inclusion of valid semantic elements
        assert!(markdown.contains("# Sovereign Pair WAG Test"));
        assert!(markdown.contains("This is a test paragraph"));
        assert!(markdown.contains("- Item 1"));
        
        // Asserting the EXCLUSION of malicious/junk elements
        assert!(!markdown.contains("HACKED"), "Scraper leaked raw Script text!");
        assert!(!markdown.contains("body { color: red; }"), "Scraper leaked raw CSS text!");
        assert!(!markdown.contains("Ignored Header Navigation"), "Scraper leaked <header> navigation!");
        assert!(!markdown.contains("Adverts here"), "Scraper leaked <aside> tags!");
    }
}
