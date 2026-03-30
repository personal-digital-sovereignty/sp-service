#![allow(clippy::collapsible_if)]
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
                "docs.microsoft.com".into(), "developer.mozilla.org".into(), "rust-lang.org".into(), "python.org".into(), "github.com".into(), "brasilindicadores.com.br".into()
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

/// Struct de Transferência de Conhecimento Zero-Click
pub struct SovereignSearchResult {
    pub links: Vec<String>,
    pub snippets: String, // Texto bruto dos fragmentos de resultado de busca
}

/// Arquitetura nativa do Sovereign Web-Augmented Generation (WAG)
/// Blindada via Rust para injetar contexto externo ao RAG Cíbrido.
pub struct DeepResearchEngine {
    client: Client,
    pub db_pool: Option<sqlx::SqlitePool>,
    adblock_engine: Option<crate::adblocker::AdblockHandle>,
    trust_matrix: TrustMatrix,
}

impl DeepResearchEngine {
    pub fn new(db_pool: Option<sqlx::SqlitePool>, adblock_engine: Option<crate::adblocker::AdblockHandle>, vault_path: Option<std::path::PathBuf>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
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

    /// Extrai o domínio principal da URL para servir de chave primaria no Ledger.
    fn extract_domain(url: &str) -> String {
        let no_protocol = url.trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("www.");
        no_protocol.split('/').next().unwrap_or(url).to_string()
    }

    /// Atualiza o Ledger de Domínios para aplicar as regras de Quarentena de 60 Dias
    async fn update_domain_ledger(&self, url: &str, html_success: bool, ghost_success: bool) {
        if let Some(pool) = &self.db_pool {
            let domain = Self::extract_domain(url);
            let uuid_str = uuid::Uuid::new_v4().to_string();
            
            let quarantine = if !html_success && !ghost_success {
                "datetime('now', '+60 days')"
            } else {
                "NULL"
            };

            let q = format!("
                INSERT INTO domain_extraction_ledger (id, domain, technique_html_success, technique_ghost_success, quarantine_until, last_attempted_at)
                VALUES (?, ?, ?, ?, {}, CURRENT_TIMESTAMP)
                ON CONFLICT(domain) DO UPDATE SET 
                    technique_html_success = ?, 
                    technique_ghost_success = ?, 
                    quarantine_until = {},
                    last_attempted_at = CURRENT_TIMESTAMP
            ", quarantine, quarantine);

            let _ = sqlx::query(&q)
                .bind(&uuid_str)
                .bind(&domain)
                .bind(html_success)
                .bind(ghost_success)
                .bind(html_success)
                .bind(ghost_success)
                .execute(pool)
                .await;
        }
    }

    /// Realiza a varredura e o scrape profundo da URL alvo.
    pub async fn scrape_url(&self, url: &str) -> Result<String, String> {
        // --- QUARANTINE FIREWALL ---
        if let Some(pool) = &self.db_pool {
            let domain = Self::extract_domain(url);
            if let Ok(Some(date)) = sqlx::query_scalar::<_, String>("SELECT quarantine_until FROM domain_extraction_ledger WHERE domain = ? AND quarantine_until > CURRENT_TIMESTAMP").bind(&domain).fetch_optional(pool).await {
                tracing::warn!("⛔ [Sovereign Firewall] Domínio '{}' está restrito na Quarentena de WAF até {}. Economizando ciclos de CPU e ignorando URL.", domain, date);
                return Err(format!("Domain '{}' is currently Quarantined due to WAF Blocks.", domain));
            }
        }

        let response = self.client.get(url).send().await.map_err(|e| format!("HTTP Request failed: {}", e))?;
        
        if !response.status().is_success() {
            // WAF Ghost Fallback! Se tomar block da nuvem, não desiste: bate no arquivo morto multi-plataforma.
            if response.status() == 403 || response.status() == 401 || response.status() == 429 || response.status() == 406 || response.status() == 503 {
                tracing::warn!("🛡️ [WAF Blocked] Defesa Interceptada (HTTP {}). Acionando The Ghost Fallback Protocol...", response.status());
                if let Ok(ghost_data) = self.scrape_ghost_fallbacks(url).await {
                    self.update_domain_ledger(url, false, true).await;
                    return Ok(ghost_data);
                }
            }
            self.update_domain_ledger(url, false, false).await;
            return Err(format!("Server returned HTTP {}", response.status()));
        }

        let html_content = response.text().await.map_err(|e| format!("Failed to read HTML body: {}", e))?;
        
        // --- HYDRATION HUNTER (Phase 4.2) ---
        // Se a página for um SPA brutalmente ofuscado (Ex: Next.js), aborta o parsing
        // de DOM (que estaria vazio) e saca diretamente do cofre JSON original!
        if let Some(json_payload) = self.extract_hydration_json(&html_content) {
            tracing::info!("🎯 [Ghost Scraper] Payload SSR Interceptado! Ignorando DOM Tree parser e entregando Ouro Puro.");
            self.update_domain_ledger(url, true, false).await;
            return Ok(json_payload);
        }

        let markdown = self.sanitize_to_markdown(&html_content);
        
        // --- EPISTEMIC FALLBACK (JS/SPA DEFEATER) ---
        let is_suspect_spa = markdown.len() < 400 
            || markdown.to_lowercase().contains("enable javascript") 
            || markdown.to_lowercase().contains("javascript is required")
            || markdown.to_lowercase().contains("please wait...");
            
        if is_suspect_spa {
            tracing::warn!("⚠️ [The Nurse / Scraper] Vazio Epistêmico Detectado! SPA interceptado ({} bytes extraídos). Acionando The Ghost Fallback Protocol...", markdown.len());
            if let Ok(ghost_markdown) = self.scrape_ghost_fallbacks(url).await
                && ghost_markdown.len() > markdown.len() {
                    tracing::info!("✅ [Ghost Protocol] Sucesso! Payload histórico resgatado do Vácuo Multi-Plataforma.");
                    self.update_domain_ledger(url, false, true).await;
                    return Ok(ghost_markdown);
                }
            self.update_domain_ledger(url, false, false).await;
        } else {
            self.update_domain_ledger(url, true, false).await;
        }
        
        Ok(markdown)
    }

    /// O Caçador de Hidratação (SSR Ghost)
    fn extract_hydration_json(&self, html: &str) -> Option<String> {
        // 1. Next.js __NEXT_DATA__
        let next_re = Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(\{.*?\})</script>"#).unwrap();
        if let Some(cap) = next_re.captures(html)
            && let Some(json_str) = cap.get(1) {
                // Return formatado para o LLM não precisar lutar com o AST
                return Some(format!("```json\n{}\n```", json_str.as_str()));
            }

        // 2. SEO Microdata (JSON-LD)
        let ld_re = Regex::new(r#"<script type="application/ld\+json">([\s\S]*?)</script>"#).unwrap();
        let mut ld_blocks = Vec::new();
        for cap in ld_re.captures_iter(html) {
            if let Some(json_str) = cap.get(1) {
                ld_blocks.push(json_str.as_str().trim().to_string());
            }
        }
        if !ld_blocks.is_empty() {
             let combined = ld_blocks.join("\n\n");
             // Se tiver mais de 200 caracteres de metadata rica, consideramos um sucesso sólido
             if combined.len() > 200 {
                 return Some(format!("```json\n{}\n```", combined));
             }
        }

        None
    }

    /// O Master Router do Ghost Protocol: Tenta extração simultânea de Caches Globais Institucionais (CDX) e Comerciais
    async fn scrape_ghost_fallbacks(&self, url: &str) -> Result<String, String> {
        let (wayback, arquivo_pt, ukwa, vefsafn, gcache, archive_ph) = tokio::join!(
            self.scrape_via_wayback_machine(url),
            self.scrape_via_arquivo_pt(url),
            self.scrape_via_ukwa(url),
            self.scrape_via_vefsafn(url),
            self.scrape_via_google_cache(url),
            self.scrape_via_archive_today(url)
        );

        // Retorna o primeiro que tiver conteúdo útil (corrida de latência paralela!)
        if let Ok(md) = arquivo_pt
            && md.len() > 200 { return Ok(md); }
        if let Ok(md) = ukwa
            && md.len() > 200 { return Ok(md); }
        if let Ok(md) = vefsafn
            && md.len() > 200 { return Ok(md); }
        if let Ok(md) = wayback
            && md.len() > 200 { return Ok(md); }
        if let Ok(md) = gcache
            && md.len() > 200 { return Ok(md); }
        if let Ok(md) = archive_ph
            && md.len() > 200 { return Ok(md); }

        Err("Todas as 6 matrizes do The Ghost Fallback Protocol (Institucionais e Caches) falharam ou retornaram o vácuo.".to_string())
    }

    async fn scrape_via_google_cache(&self, url: &str) -> Result<String, String> {
        let cache_url = format!("https://webcache.googleusercontent.com/search?q=cache:{}", urlencoding::encode(url));
        if let Ok(resp) = self.client.get(&cache_url).header(reqwest::header::USER_AGENT, Self::get_random_user_agent()).send().await
            && resp.status().is_success()
                && let Ok(html) = resp.text().await {
                    let markdown = self.sanitize_to_markdown(&html);
                    tracing::info!("👻 [Ghost Protocol] Google Cache Hit: {} bytes resgatados.", markdown.len());
                    return Ok(markdown);
                }
        Err("Google Cache Fallback Failed".to_string())
    }

    async fn scrape_via_archive_today(&self, url: &str) -> Result<String, String> {
        let archive_url = format!("https://archive.ph/latest/{}", url);
        if let Ok(resp) = self.client.get(&archive_url).header(reqwest::header::USER_AGENT, Self::get_random_user_agent()).send().await
            && resp.status().is_success()
                && let Ok(html) = resp.text().await {
                    let markdown = self.sanitize_to_markdown(&html);
                    tracing::info!("👻 [Ghost Protocol] Archive.today Hit: {} bytes resgatados.", markdown.len());
                    return Ok(markdown);
                }
        Err("Archive.today Fallback Failed".to_string())
    }

    /// Ghost Fallback (Lusófono/Europeu): Arquivo.pt (Solr/PyWB)
    async fn scrape_via_arquivo_pt(&self, url: &str) -> Result<String, String> {
        tracing::info!("👻 [Ghost Protocol] Invocando Arquivo.pt (Europa/GDPR): arquivo.pt/wayback/cdx");
        let cdx_url = format!("https://arquivo.pt/wayback/cdx?url={}&output=json&limit=1&filter=statuscode:200", urlencoding::encode(url));
        if let Ok(cdx_req) = self.client.get(&cdx_url).header(reqwest::header::USER_AGENT, Self::get_random_user_agent()).send().await {
            if let Ok(cdx_json) = cdx_req.json::<Vec<Vec<String>>>().await {
                if cdx_json.len() >= 2 {
                    let timestamp = &cdx_json[1][1];
                    let original_url = &cdx_json[1][2]; 
                    let ghost_url = format!("https://arquivo.pt/wayback/{}id_/{}", timestamp, original_url);
                    if let Ok(ghost_req) = self.client.get(&ghost_url).send().await {
                        if ghost_req.status().is_success() {
                            let ghost_html = ghost_req.text().await.unwrap_or_default();
                            tracing::info!("🔗 [Ghost Protocol][Arquivo.pt] Download passivo efetuado (Timestamp: {})", timestamp);
                            if let Some(json_payload) = self.extract_hydration_json(&ghost_html) { return Ok(json_payload); }
                            return Ok(self.sanitize_to_markdown(&ghost_html));
                        }
                    }
                }
            }
        }
        Err("Arquivo.pt Fallback Failed".into())
    }

    /// Ghost Fallback (Inglês Global): UK Web Archive (UKWA)
    async fn scrape_via_ukwa(&self, url: &str) -> Result<String, String> {
        tracing::info!("👻 [Ghost Protocol] Invocando UKWA (Global English/UK): webarchive.org.uk");
        let cdx_url = format!("https://www.webarchive.org.uk/wayback/archive/cdx?url={}&output=json&limit=1&filter=statuscode:200", urlencoding::encode(url));
        if let Ok(cdx_req) = self.client.get(&cdx_url).header(reqwest::header::USER_AGENT, Self::get_random_user_agent()).send().await {
            if let Ok(cdx_json) = cdx_req.json::<Vec<Vec<String>>>().await {
                if cdx_json.len() >= 2 {
                    let timestamp = &cdx_json[1][1];
                    let original_url = &cdx_json[1][2]; 
                    let ghost_url = format!("https://www.webarchive.org.uk/wayback/archive/{}id_/{}", timestamp, original_url);
                    if let Ok(ghost_req) = self.client.get(&ghost_url).send().await {
                        if ghost_req.status().is_success() {
                            let ghost_html = ghost_req.text().await.unwrap_or_default();
                            tracing::info!("🔗 [Ghost Protocol][UKWA] Download passivo efetuado (Timestamp: {})", timestamp);
                            if let Some(json_payload) = self.extract_hydration_json(&ghost_html) { return Ok(json_payload); }
                            return Ok(self.sanitize_to_markdown(&ghost_html));
                        }
                    }
                }
            }
        }
        Err("UKWA Fallback Failed".into())
    }

    /// Ghost Fallback (RocksDB High-Speed Europe): Vefsafn.is (Islândia)
    async fn scrape_via_vefsafn(&self, url: &str) -> Result<String, String> {
        tracing::info!("👻 [Ghost Protocol] Invocando Vefsafn.is (OutbackCDX/RocksDB): wayback.vefsafn.is");
        let cdx_url = format!("https://wayback.vefsafn.is/wayback/cdx?url={}&output=json&limit=1&filter=statuscode:200", urlencoding::encode(url));
        if let Ok(cdx_req) = self.client.get(&cdx_url).header(reqwest::header::USER_AGENT, Self::get_random_user_agent()).send().await {
            if let Ok(cdx_json) = cdx_req.json::<Vec<Vec<String>>>().await {
                if cdx_json.len() >= 2 {
                    let timestamp = &cdx_json[1][1];
                    let original_url = &cdx_json[1][2]; 
                    let ghost_url = format!("https://wayback.vefsafn.is/wayback/{}id_/{}", timestamp, original_url);
                    if let Ok(ghost_req) = self.client.get(&ghost_url).send().await {
                        if ghost_req.status().is_success() {
                            let ghost_html = ghost_req.text().await.unwrap_or_default();
                            tracing::info!("🔗 [Ghost Protocol][Vefsafn.is] Download passivo efetuado (Timestamp: {})", timestamp);
                            if let Some(json_payload) = self.extract_hydration_json(&ghost_html) { return Ok(json_payload); }
                            return Ok(self.sanitize_to_markdown(&ghost_html));
                        }
                    }
                }
            }
        }
        Err("Vefsafn.is Fallback Failed".into())
    }

    /// Ghost Fallback: Consome o cache passivo do Wayback Machine via CDX API para aniquilar WAFs
    async fn scrape_via_wayback_machine(&self, url: &str) -> Result<String, String> {
        tracing::info!("👻 [Ghost Protocol] Invocando arquivo passivo: web.archive.org/cdx");
        
        // 1. Busca a captura mais recente em JSON (Status=200 Orgânico apenas)
        let cdx_url = format!("https://web.archive.org/cdx/search/cdx?url={}&output=json&limit=1&filter=statuscode:200", urlencoding::encode(url));
        
        let cdx_req = self.client.get(&cdx_url)
            .header(reqwest::header::USER_AGENT, Self::get_random_user_agent())
            .send().await.map_err(|e| format!("Wayback CDX API falhou: {}", e))?;
            
        if !cdx_req.status().is_success() {
            return Err("Wayback Machine bloqueou ou não respondeu a query CDX.".to_string());
        }
        
        let cdx_json: Vec<Vec<String>> = cdx_req.json().await.map_err(|_| "Falha ao ler cache CDX".to_string())?;
        
        if cdx_json.len() < 2 {
            return Err("Wayback Machine reportou: Nenhum snapshot HTTP 200 encontrado para este domínio.".to_string());
        }
        
        // 2. Extrai o timestamp da resposta
        let timestamp = &cdx_json[1][1];
        let original_url = &cdx_json[1][2]; // O target original url pode diferir levemente
        
        // 3. Monta a URL de visualização bruta (raw document para evitar o frame HTML do próprio archive)
        let ghost_url = format!("https://web.archive.org/web/{}id_/{}", timestamp, original_url);
        tracing::info!("🔗 [Ghost Protocol] Download passivo do snapshot efetuado no Timestamp: {}", timestamp);
        
        let ghost_req = self.client.get(&ghost_url).send().await.map_err(|e| format!("Falha no download via proxy fantasma: {}", e))?;
        
        if !ghost_req.status().is_success() {
            return Err("Falha na descompressão do snapshot fantasma.".to_string());
        }
        
        let ghost_html = ghost_req.text().await.unwrap_or_default();
        
        // Tenta achar JSON-LD interno no Ghost State primeiro
        if let Some(json_payload) = self.extract_hydration_json(&ghost_html) {
             return Ok(json_payload);
        }
        
        Ok(self.sanitize_to_markdown(&ghost_html))
    }

    /// Limpa o HTML ruidoso (scripts, estilos, anúncios) e extrai o texto principal em formato Semântico (Markdown-like).
    fn sanitize_to_markdown(&self, html: &str) -> String {
        let document = Html::parse_document(html);
        
        // Remove tags ofensores (Anti-Junk)
        // Isso é uma filtragem em memória antes da decodificação.
        let mut text_blocks = Vec::new();
        let mut lexical_density_tracker = std::collections::HashMap::new();
        
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
            
            // Filtro 3: Lexical Density (SEO Content Farms Spam)
            // Lixo SEO tem o péssimo hábito de repetir parágrafos idênticos ("Veja os Melhores X de 2026")
            if clean_text.len() > 40 {
                let freq = lexical_density_tracker.entry(clean_text.to_string()).or_insert(0);
                *freq += 1;
                
                if *freq > 4 {
                    tracing::error!("☣️ [Anti-SEO Firewall] Boilerplate Tóxico Detectado! Parágrafo ({} bytes) repetido 5x. Abortando árvore DOM por completo...", clean_text.len());
                    return String::new(); // Retorna vazio. Isso fará o scraper classificar como Vazio Epistemico e pular!
                }
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

    /// Extrator Semântico Zero-Click: Varre DuckDuckGo Lite para extrair os "Snippets" (Descrição do Resultado) 
    async fn search_duckduckgo_lite(&self, query: &str) -> String {
        let url = "https://lite.duckduckgo.com/lite/";
        let params = [("q", query), ("kl", "br-pt")];
        
        let req = self.client.post(url)
            .header(reqwest::header::USER_AGENT, Self::get_random_user_agent())
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml,application/xml")
            .header("Sec-Ch-Ua", "\"Google Chrome\";v=\"131\", \"Not:A-Brand\";v=\"8\"")
            .header("Sec-Ch-Ua-Mobile", "?0")
            .header("Sec-Ch-Ua-Platform", "\"Windows\"")
            .form(&params)
            .send()
            .await;

        match req {
            Ok(response) => {
                if let Ok(html) = response.text().await {
                    let document = scraper::Html::parse_document(&html);
                    let selector_snippet = scraper::Selector::parse("td.result-snippet").unwrap();
                    let mut snippets_markdown = String::new();
                    
                    for element in document.select(&selector_snippet).take(8) {
                        let text: Vec<_> = element.text().collect();
                        let clean_text = text.join(" ").replace("\n", " ").trim().to_string();
                        if clean_text.len() > 10 {
                            snippets_markdown.push_str(&format!("- [DDG Organic Snippet]: {}\n", clean_text));
                        }
                    }
                    return snippets_markdown;
                }
            },
            Err(e) => {
                tracing::debug!("🚨 DDG Sniper Erro: {}", e);
            }
        }
        String::new()
    }

    /// Dispara a busca Multi-Hop com tolerância impecável a falhas WAF/Cloudflare.
    pub async fn search_web(&self, query: &str) -> Result<SovereignSearchResult, String> {
        tracing::info!("🔍 [WAG] Inicializando Busca Autônoma Sovereign Omni-Scraper por: '{}'", query);
        
        // 1. O SOVEREIGN PARSER PARALELO: Tenta raspar Proxy Native + DuckDuckGo Snippets
        let (yahoo_res, ddg_snippets) = tokio::join!(
            self.search_sovereign_meta(query),
            self.search_duckduckgo_lite(query)
        );

        let mut final_links = Vec::new();

        match yahoo_res {
            Ok(links) if !links.is_empty() => {
                final_links = self.apply_pi_hole_filter(links).await;
                tracing::info!("✅ [WAG] Sovereign Meta-Search Bem-Sucedido! ({}) links orgânicos purificados.", final_links.len());
            },
            Err(e) => {
                let msg = format!("⚠️ [WAG Fallback] O motor primário tomou WAF Block. Rotacionando para SearXNG P2P. Erro: {}", e);
                tracing::warn!("{}", msg);
            },
            _ => {
                tracing::warn!("⚠️ [WAG Fallback] O motor primário encontrou 0 resultados.");
            }
        }

        // 2. O FALLBACK SOBERANO: Motores P2P
        if final_links.is_empty() {
            match self.search_searxng_public(query).await {
                Ok(links) if !links.is_empty() => {
                    final_links = self.apply_pi_hole_filter(links).await;
                    tracing::info!("✅ [WAG] Busca Cíbrida SearxNG Bem-Sucedida! ({}) links ancorados.", final_links.len());
                },
                _ => {
                    tracing::error!("❌ [WAG Fim de Linha] WAF Absoluto. SearXNG caiu também.");
                }
            }
        }

        if final_links.is_empty() && ddg_snippets.is_empty() {
             return Err("Dual-Engine Crash. WAF Block total e Snippets falharam.".to_string());
        }

        Ok(SovereignSearchResult {
            links: final_links,
            snippets: ddg_snippets,
        })
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
    /// (DESATIVADO): O usuário determinou a abolição das amarras de Tiers.
    async fn assign_sovereign_trust_score(&self, links: Vec<String>) -> Vec<String> {
        // Retorna a lista orgânica de pesquisa intocada sem pontuar `.gov.br` ou aplicar penalidades.
        links
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

        let mut links = Vec::new();
        {
            let document = scraper::Html::parse_document(&html);
            let selector = scraper::Selector::parse("a").unwrap();
            
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
        }

        links.dedup();
        let mut prioritized_links = self.assign_sovereign_trust_score(links).await;
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
                                let mut prioritized_links = self.assign_sovereign_trust_score(links).await;
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
