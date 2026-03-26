use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct TrustMatrix {
    pub tier1: Vec<String>,
    pub tier2: Vec<String>,
    pub encyclopedia: Vec<String>,
}

impl Default for TrustMatrix {
    fn default() -> Self {
        Self {
            tier1: vec![
                "gov.br".into(), "ibge.gov.br".into(), "bcb.gov.br".into(), "tesourodireto.com.br".into(), 
                "anp.gov.br".into(), "cade.gov.br".into(), "mil.br".into(), ".gov/".into(), ".mil/".into(),
                "gov.uk".into(), "gob.es".into(), "gob.ar".into(), "gouv.fr".into(), "bund.de".into(), "go.jp".into(),
                "edu".into(), "edu.br".into(), "usp.br".into(), "mit.edu".into(), "harvard.edu".into(),
                "docs.microsoft.com".into(), "developer.mozilla.org".into(), "rust-lang.org".into(), "python.org".into(), "github.com".into()
            ],
            tier2: vec![
                "agenciabrasil.ebc.com.br".into(), "estadao.com.br".into(), "folha.uol.com.br".into(), 
                "piaui.folha.uol.com.br".into(), "nexojornal.com.br".into(), "lupa.uol.com.br".into(), 
                "aosfatos.org".into(), "apublica.org".into(), "infomoney.com.br".into(), "valor.globo.com".into(),
                "bbc.com".into(), "dw.com".into(), "reuters.com".into(), "stackoverflow.com".into()
            ],
            encyclopedia: vec!["wikipedia.org".into()],
        }
    }
}

struct ScoredUrl {
    url: String,
    score: i32,
}

/// Arquitetura nativa do Sovereign Web-Augmented Generation (WAG)
/// Blindada via Rust para injetar contexto externo ao RAG Cíbrido.
pub struct DeepResearchEngine {
    client: Client,
    db_pool: Option<sqlx::SqlitePool>,
    adblock_engine: Option<crate::adblocker::AdblockHandle>,
    trust_matrix: TrustMatrix,
}

impl DeepResearchEngine {
    pub fn new(db_pool: Option<sqlx::SqlitePool>, adblock_engine: Option<crate::adblocker::AdblockHandle>, vault_path: Option<std::path::PathBuf>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| Client::new());

        let mut trust_matrix = TrustMatrix::default();
        if let Some(path) = vault_path {
            let matrix_path = path.join("_agents").join("trust_matrix.json");
            if let Ok(content) = std::fs::read_to_string(&matrix_path) {
                if let Ok(parsed) = serde_json::from_str::<TrustMatrix>(&content) {
                    trust_matrix = parsed;
                }
            } else {
                // Generate default file if it doesn't exist
                let _ = std::fs::create_dir_all(matrix_path.parent().unwrap());
                if let Ok(json_str) = serde_json::to_string_pretty(&trust_matrix) {
                    let _ = std::fs::write(&matrix_path, json_str);
                }
            }
        }

        Self { client, db_pool, adblock_engine, trust_matrix }
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
        
        // Vamos capturar apenas elementos atômicos contendo texto orgânico e dados tabulares
        let selector = Selector::parse("p, h1, h2, h3, h4, h5, h6, li, td, th").unwrap();
        
        for element in document.select(&selector) {
            // Filtro 1: Genética (Ancestors)
            // Se o nó estiver dentro de contêineres lixo, ele é sumariamente abatido.
            let is_junk = element.ancestors().any(|a| {
                if let scraper::node::Node::Element(el) = a.value() {
                    let name = el.name();
                    matches!(name, "nav" | "header" | "footer" | "aside" | "script" | "style" | "noscript" | "form" | "iframe" | "button" | "dialog")
                } else {
                    false
                }
            });

            if is_junk {
                continue;
            }

            let tag_name = element.value().name();
            let is_header = tag_name.starts_with('h');
            let is_table_data = tag_name == "td" || tag_name == "th";
            
            // Foca o inner text, ignorando scripts implícitos
            let inner_text = element.text().collect::<Vec<_>>().join(" ");
            let clean_text = inner_text.trim();
            
            if clean_text.is_empty() {
                continue;
            }

            // Filtro 2: Comprimento Semântico
            // Ignorar textos minúsculos como "Fazer Login", "Categorias", "iPhone 15", etc.
            // Exceção: Cabeçalhos e Data Cells (td/th) podem ser curtos (Ex: "Resumo", "Conclusão", "R$ 5,30").
            if !is_header && !is_table_data && clean_text.len() < 30 && clean_text.split_whitespace().count() < 5 {
                continue;
            }
            
            // Formata o header
            let formatted = match tag_name {
                "h1" => format!("# {}\n", clean_text),
                "h2" => format!("## {}\n", clean_text),
                "h3" => format!("### {}\n", clean_text),
                "h4" => format!("#### {}\n", clean_text),
                "h5" => format!("##### {}\n", clean_text),
                "h6" => format!("###### {}\n", clean_text),
                "li" => format!("- {}", clean_text),
                "td" | "th" => format!("| {} |", clean_text),
                _ => format!("{}\n", clean_text), // <p>
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
    /// Estratégia 1: Sovereign Meta-Search (Proxy Nativo)
    /// Estratégia 2: Rotação Cíbrida Global de SearxNGs
    pub async fn search_web(&self, query: &str) -> Result<Vec<String>, String> {
        tracing::info!("🔍 [WAG] Inicializando Busca Autônoma Sovereign Meta-Search por: '{}'", query);
        
        // 1. O SOVEREIGN PARSER: Tenta raspar via Proxy Native com User-Agent Rotativo
        match self.search_sovereign_meta(query).await {
            Ok(links) if !links.is_empty() => {
                let clean_links = self.apply_pi_hole_filter(links).await;
                tracing::info!("✅ [WAG] Sovereign Meta-Search Bem-Sucedido! ({}) links orgânicos purificados.", clean_links.len());
                return Ok(clean_links);
            },
            Err(e) => {
                let msg = format!("⚠️ [WAG Fallback] O motor primário tomou WAF Block. Rotacionando para SearXNG P2P. Erro: {}", e);
                tracing::warn!("{}", msg);
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("deep_research_waf_audit.log") {
                    use std::io::Write;
                    let _ = writeln!(&mut file, "[Deep Research Agent] {}", msg);
                }
            },
            _ => {
                let msg = "⚠️ [WAG Fallback] O motor primário encontrou 0 resultados ou tomou proxy reset.";
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
                let clean_links = self.apply_pi_hole_filter(links).await;
                tracing::info!("✅ [WAG] Busca Cíbrida SearxNG Bem-Sucedida! ({}) links limpos ancorados.", clean_links.len());
                Ok(clean_links)
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

    /// Implementa o Sovereign Pi-Hole: Trilha a lista inteira contra o motor Bravee C/Rust
    /// e descarta sumariamente propagandas, telemetria e trackers.
    async fn apply_pi_hole_filter(&self, links: Vec<String>) -> Vec<String> {
        let mut final_links = Vec::new();
        if let Some(adb) = &self.adblock_engine {
            for link in links {
                let is_blocked = adb.check_url(&link).await;
                
                if is_blocked {
                   tracing::info!("🛡️ [Sovereign Pi-Hole] Lixo Publicitário Sublimado: {}", link);
                   if let Some(pool) = &self.db_pool {
                       // Atualiza a totalidade de Analytics
                       let _ = sqlx::query("UPDATE analytics SET val_int = val_int + 1 WHERE id = 'total_trackers_blocked'").execute(pool).await;
                   }
                } else {
                   final_links.push(link);
                }
            }
        } else {
            return links;
        }
        final_links
    }

    /// Executa o algoritmo de Vetting Institucional.
    /// Avalia cada link orgânico extraído e joga fontes Tier-1 e Tier-2 para o Topo do Index, 
    /// destruindo algoritmos de SEO falsos.
    fn assign_sovereign_trust_score(&self, links: Vec<String>) -> Vec<String> {
        let mut scored_links: Vec<ScoredUrl> = links.into_iter().map(|url| {
            let mut score = 0;
            let url_lower = url.to_lowercase();
            
            // Check Trust Matrix (Epistemology Rule)
            if self.trust_matrix.tier1.iter().any(|d| url_lower.contains(d)) {
                score += 200; // Tier 1: Guardian of Raw Numbers (gov.br, ibge)
            } else if self.trust_matrix.tier2.iter().any(|d| url_lower.contains(d)) {
                score += 200; // Tier 2: Guardian of the Narrative (Equal Epistemic weight!)
            } else if self.trust_matrix.encyclopedia.iter().any(|d| url_lower.contains(d)) {
                score += 30;
            } else if url_lower.contains(".org") || url_lower.contains(".io") {
                score += 10;
            }

            // Penalize suspicious SEO trash
            if url_lower.contains("pinterest.") || url_lower.contains("quora.") || url_lower.contains("yahoo.answers") {
                score -= 100;
            }

            ScoredUrl { url, score }
        }).collect();

        // Sort descending
        scored_links.sort_by(|a, b| b.score.cmp(&a.score));
        
        // Strip out the metadata struct and return the raw string vectors
        scored_links.into_iter().map(|s| s.url).collect()
    }

    /// Rotacionador de Identidade (Sovereign Cloak)
    fn get_random_user_agent() -> &'static str {
        let uas = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.3.1 Safari/605.1.15",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:124.0) Gecko/20100101 Firefox/124.0",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 Edg/123.0.2420.65",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        ];
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        uas[(now as usize) % uas.len()]
    }

    /// Extrator Cíbrido Nativo no Rust que utiliza Motores abertos como Proxy para bypass de WAF.
    async fn search_sovereign_meta(&self, query: &str) -> Result<Vec<String>, String> {
        let url = format!("https://search.yahoo.com/search?p={}", urlencoding::encode(query));
        
        let req = self.client.get(&url)
            .header(reqwest::header::USER_AGENT, Self::get_random_user_agent())
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9,pt-BR;q=0.8,pt;q=0.7")
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
            .header(reqwest::header::DNT, "1")
            .header("Sec-Ch-Ua", "\"Google Chrome\";v=\"123\", \"Not:A-Brand\";v=\"8\", \"Chromium\";v=\"123\"")
            .header("Sec-Ch-Ua-Mobile", "?0")
            .header("Sec-Ch-Ua-Platform", "\"Windows\"")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "none")
            .header("Sec-Fetch-User", "?1")
            .header("Upgrade-Insecure-Requests", "1")
            .send()
            .await.map_err(|e| format!("Proxy Nativo Erro de Rede: {}", e))?;

        if !req.status().is_success() {
            return Err(format!("Interceptado por HTTP Status {}", req.status()));
        }

        let html = req.text().await.map_err(|e| format!("Falha de Decodificação DOM: {}", e))?;
        
        if html.contains("To proceed, please verify that you are not a robot.") {
            return Err("Honeypot/Captcha nativo engatilhado na camada HTML do Engine Primário".to_string());
        }

        let document = scraper::Html::parse_document(&html);
        let selector = scraper::Selector::parse("a").unwrap();
        
        let mut links = Vec::new();
        for element in document.select(&selector) {
            if let Some(href_attr) = element.value().attr("href") {
                if href_attr.contains("RU=") {
                    let parts: Vec<&str> = href_attr.split("RU=").collect();
                    if parts.len() > 1 {
                        let inner = parts[1].split("/RK=").next().unwrap_or("");
                        if let Ok(decoded) = urlencoding::decode(inner) {
                            let clean_link = decoded.into_owned();
                            if clean_link.starts_with("http") && !clean_link.contains("yahoo.com") {
                                links.push(clean_link);
                            }
                        }
                    }
                } else if href_attr.starts_with("http") && !href_attr.contains("yahoo.com") && !href_attr.contains("r.search.yahoo.com") {
                    links.push(href_attr.to_string());
                }
            }
        }

        links.dedup();
        let mut prioritized_links = self.assign_sovereign_trust_score(links);
        prioritized_links.truncate(20); // Retorna estritamente o Top 20 de Confiabilidade
        Ok(prioritized_links)
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
                                let mut prioritized_links = self.assign_sovereign_trust_score(links);
                                prioritized_links.truncate(20);
                                return Ok(prioritized_links);
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
        let engine = DeepResearchEngine::new(None, None, None);
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
                        <li>Primeira evidência da RAG Matrix Pipeline Ativa</li>
                        <li>Segunda evidência da RAG Matrix Pipeline Ativa</li>
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
        assert!(markdown.contains("- Primeira evidência da RAG"));
        
        // Asserting the EXCLUSION of malicious/junk elements
        assert!(!markdown.contains("HACKED"), "Scraper leaked raw Script text!");
        assert!(!markdown.contains("body { color: red; }"), "Scraper leaked raw CSS text!");
        assert!(!markdown.contains("Ignored Header Navigation"), "Scraper leaked <header> navigation!");
        assert!(!markdown.contains("Adverts here"), "Scraper leaked <aside> tags!");
    }
}
