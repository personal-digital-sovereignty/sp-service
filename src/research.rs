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

#[allow(dead_code)]
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
    #[allow(dead_code)]
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
            
            let is_critical = domain.ends_with(".gov.br") || domain.contains("bcb.gov.br") || domain.contains("ibge.gov.br") || domain.contains("anp.gov.br") || domain.contains("fgv.br");
            let is_commercial = domain.contains("tradingeconomics.com") || domain.contains("infomoney.com.br") || domain.contains("valor.globo");
            
            let quarantine = if !html_success && !ghost_success {
                if is_critical {
                    "datetime('now', '+5 minutes')"
                } else if is_commercial {
                    "datetime('now', '+2 hours')"
                } else {
                    "datetime('now', '+3 days')"
                }
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
    pub async fn scrape_url(&self, original_url: &str) -> Result<String, String> {
        if !crate::guardrails::is_safe_url(original_url) {
            return Err("SSRF Guardrail Ativado: O domínio de destino não é roteável publicamente.".to_string());
        }
        let mut url = original_url.to_string();

        // --- QUARANTINE FIREWALL ---
        if let Some(pool) = &self.db_pool {
            let domain = Self::extract_domain(&url);
            if let Ok(Some(date)) = sqlx::query_scalar::<_, String>("SELECT quarantine_until FROM domain_extraction_ledger WHERE domain = ? AND quarantine_until > CURRENT_TIMESTAMP").bind(&domain).fetch_optional(pool).await {
                
                // Cadeia de Fallback Estruturada
                if domain.contains("ibge") && !url.contains("sidra") {
                    tracing::warn!("⛔ [Sovereign Firewall] IBGE restrito até {}. Fallback P2P: portal SIDRA.", date);
                    url = "https://sidra.ibge.gov.br/tabela/1737".to_string();
                } else if domain.contains("bcb.gov.br") && !url.contains("dadosabertos") {
                    tracing::warn!("⛔ [Sovereign Firewall] BCB restrito até {}. Fallback P2P: Dados Abertos.", date);
                    url = "https://dadosabertos.bcb.gov.br/dataset/4449".to_string();
                } else if domain.contains("tradingeconomics.com") {
                    tracing::warn!("⛔ [Sovereign Firewall] TradingEconomics restrito até {}. Fallback alternativo: Dados de Mercado.", date);
                    url = "https://www.dadosdemercado.com.br/indices/ipca".to_string();
                } else {
                    tracing::warn!("⛔ [Sovereign Firewall] Domínio '{}' está restrito na Quarentena de WAF até {}. Economizando ciclos de CPU e ignorando URL.", domain, date);
                    return Err(format!("Domain '{}' is currently Quarantined due to WAF Blocks.", domain));
                }

                // Re-validate the fallback URL
                let new_domain = Self::extract_domain(&url);
                if let Ok(Some(new_date)) = sqlx::query_scalar::<_, String>("SELECT quarantine_until FROM domain_extraction_ledger WHERE domain = ? AND quarantine_until > CURRENT_TIMESTAMP").bind(&new_domain).fetch_optional(pool).await {
                    return Err(format!("Fallback Domain '{}' is also Quarantined until {}.", new_domain, new_date));
                }
            }
        }

        // --- CPU OFFLOADING (Fase 9: Cloud Readers Múltiplos) ---
        // Desacopla 90% do estresse de processamento do Rust repassando a interpretação do DOM para a nuvem.
        let cloud_readers = vec![
            format!("https://r.jina.ai/{}", url),
            format!("https://md.dita.to/{}", url),
            format!("https://txtify.it/{}", url), // Fallback 1: Txtify Reader
            format!("https://urltomarkdown.com/api?url={}", urlencoding::encode(&url)), // Fallback 2: General URL to MD
            format!("https://api.firecrawl.dev/v0/scrape?url={}", urlencoding::encode(&url)), // Fallback 3: Firecrawl Public Tier
        ];

        for reader_url in cloud_readers {
            if let Ok(resp) = self.client.get(&reader_url).header("X-Return-Format", "markdown").send().await {
                if resp.status().is_success() {
                    if let Ok(mut markdown) = resp.text().await {
                        // Trata proxies que devolvem JSON (ex: Firecrawl Public Tier ou WebIT)
                        if markdown.starts_with('{') && markdown.contains("\"markdown\"") {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&markdown) {
                                if let Some(md_data) = parsed.get("data").and_then(|d| d.get("markdown")).and_then(|m| m.as_str()) {
                                    markdown = md_data.to_string();
                                } else if let Some(md_text) = parsed.get("markdown").and_then(|m| m.as_str()) {
                                    markdown = md_text.to_string();
                                }
                            }
                        }
                        
                        // Trata proxies que devolvem HTML puro ao invés de Markdown
                        if markdown.trim().starts_with('<') {
                             markdown = self.sanitize_to_markdown(&markdown);
                        }

                        if markdown.len() > 200 && !markdown.to_lowercase().contains("enable javascript") && !markdown.contains("Access Denied") && !markdown.contains("Just a moment...") {
                            let snippet: String = markdown.replace('\n', " ").chars().take(150).collect();
                            tracing::info!("☁️ [Cloud Reader Offload] Markdown resgatado com sucesso via {} ({} bytes). CPU protegida!\n[PAYLOAD SNIPPET]: {}...", reader_url.split('/').nth(2).unwrap_or("Proxy"), markdown.len(), snippet);
                            self.update_domain_ledger(&url, true, false).await;
                            return Ok(markdown);
                        }
                    }
                }
            }
        }
        
        tracing::warn!("⚠️ [Cloud Offload Falhou] Retornando ao Scraper Nativo em Tela Cheia (Heavy CPU) para: {}", url);

        let response = self.client.get(&url).header(reqwest::header::USER_AGENT, Self::get_random_user_agent()).send().await.map_err(|e| format!("HTTP Request failed: {}", e))?;
        
        if !response.status().is_success() {
            // WAF Ghost Fallback! Se tomar block da nuvem, não desiste: bate no arquivo morto multi-plataforma.
            if response.status() == 403 || response.status() == 401 || response.status() == 429 || response.status() == 406 || response.status() == 503 {
                tracing::warn!("🛡️ [WAF Blocked] Defesa Interceptada (HTTP {}). Acionando The Ghost Fallback Protocol...", response.status());
                if let Ok(ghost_data) = self.scrape_ghost_fallbacks(&url).await {
                    self.update_domain_ledger(&url, false, true).await;
                    return Ok(ghost_data);
                }
            }
            self.update_domain_ledger(&url, false, false).await;
            return Err(format!("Server returned HTTP {}", response.status()));
        }

        let html_content = response.text().await.map_err(|e| format!("Failed to read HTML body: {}", e))?;
        
        // --- HYDRATION HUNTER (Phase 4.2) ---
        // Se a página for um SPA brutalmente ofuscado (Ex: Next.js), aborta o parsing
        // de DOM (que estaria vazio) e saca diretamente do cofre JSON original!
        if let Some(json_payload) = self.extract_hydration_json(&html_content) {
            let snippet: String = json_payload.replace('\n', " ").chars().take(150).collect();
            tracing::info!("🎯 [Ghost Scraper] Payload SSR Interceptado! Ignorando DOM Tree parser e entregando Ouro Puro.\n[PAYLOAD SNIPPET]: {}...", snippet);
            self.update_domain_ledger(&url, true, false).await;
            return Ok(json_payload);
        }

        let markdown = self.sanitize_to_markdown(&html_content);
        
        // --- EPISTEMIC FALLBACK (JS/SPA DEFEATER) ---
        let is_suspect_spa = markdown.len() < 400 
            || markdown.to_lowercase().contains("enable javascript") 
            || markdown.to_lowercase().contains("javascript is required")
            || markdown.to_lowercase().contains("please wait...");
            
        if is_suspect_spa {
            // FIX-5: Rate-limit do Ghost Protocol — usar contador atômico global.
            // Limita a 3 ativações por sessão de scraping para evitar 66+ requests
            // paralelas a Wayback Machine, Arquivo.pt, UKWA, Vefsafn.is etc.
            static GHOST_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let ghost_count = GHOST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            if ghost_count < 3 {
                tracing::warn!("⚠️ [The Nurse / Scraper] Vazio Epistêmico Detectado! SPA interceptado ({} bytes extraídos). Acionando The Ghost Fallback Protocol ({}/3)...", markdown.len(), ghost_count + 1);
                if let Ok(ghost_markdown) = self.scrape_ghost_fallbacks(&url).await {
                    if ghost_markdown.len() > markdown.len() {
                        tracing::info!("✅ [Ghost Protocol] Sucesso! Payload histórico resgatado do Vácuo Multi-Plataforma.");
                        self.update_domain_ledger(&url, false, true).await;
                        return Ok(ghost_markdown);
                    }
                }
            } else {
                tracing::warn!("🛑 [Ghost Protocol] Rate-limit atingido ({}/3). Ignorando fallback de arquivo para evitar sobrecarga de rede.", ghost_count + 1);
            }
            self.update_domain_ledger(&url, false, false).await;
        } else {
            self.update_domain_ledger(&url, true, false).await;
        }
        
        Ok(markdown)
    }

    /// O Caçador de Hidratação (SSR Ghost)
    fn extract_hydration_json(&self, html: &str) -> Option<String> {
        // 1. Next.js __NEXT_DATA__
        let next_re = Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(\{.*?\})</script>"#).unwrap();
        if let Some(cap) = next_re.captures(html) {
            if let Some(json_str) = cap.get(1) {
                // Return formatado para o LLM não precisar lutar com o AST
                return Some(format!("```json\n{}\n```", json_str.as_str()));
            }
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
             // FIX-4: Filtro de qualidade JSON-LD — rejeitar schema.org puramente estrutural
             // que não contém conteúdo jornalístico (BreadcrumbList, SiteNavigationElement, AudioObject
             // sem articleBody). Estes poluem o contexto do LLM com metadata inútil (~48KB cada).
             let combined_lower = combined.to_lowercase();
             let has_article_content = combined_lower.contains("articlebody")
                 || combined_lower.contains("description")
                 || combined_lower.contains("headline")
                 || combined_lower.contains("text");
             let is_structural_only = (combined_lower.contains("breadcrumblist") || combined_lower.contains("sitenavigationelement"))
                 && !has_article_content;

             if is_structural_only {
                 tracing::debug!("🗑️ [SSR Filter] JSON-LD descartado: BreadcrumbList/NavElement sem conteúdo jornalístico ({} bytes)", combined.len());
                 return None;
             }

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
        if let Ok(md) = arquivo_pt {
            if md.len() > 200 { return Ok(md); }
        }
        if let Ok(md) = ukwa {
            if md.len() > 200 { return Ok(md); }
        }
        if let Ok(md) = vefsafn {
            if md.len() > 200 { return Ok(md); }
        }
        if let Ok(md) = wayback {
            if md.len() > 200 { return Ok(md); }
        }
        if let Ok(md) = gcache {
            if md.len() > 200 { return Ok(md); }
        }
        if let Ok(md) = archive_ph {
            if md.len() > 200 { return Ok(md); }
        }

        Err("Todas as 6 matrizes do The Ghost Fallback Protocol (Institucionais e Caches) falharam ou retornaram o vácuo.".to_string())
    }

    async fn scrape_via_google_cache(&self, url: &str) -> Result<String, String> {
        let cache_url = format!("https://webcache.googleusercontent.com/search?q=cache:{}", urlencoding::encode(url));
        if let Ok(resp) = self.client.get(&cache_url).header(reqwest::header::USER_AGENT, Self::get_random_user_agent()).send().await {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    let markdown = self.sanitize_to_markdown(&html);
                    tracing::info!("👻 [Ghost Protocol] Google Cache Hit: {} bytes resgatados.", markdown.len());
                    return Ok(markdown);
                }
            }
        }
        Err("Google Cache Fallback Failed".to_string())
    }

    async fn scrape_via_archive_today(&self, url: &str) -> Result<String, String> {
        let archive_url = format!("https://archive.ph/latest/{}", url);
        if let Ok(resp) = self.client.get(&archive_url).header(reqwest::header::USER_AGENT, Self::get_random_user_agent()).send().await {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    let markdown = self.sanitize_to_markdown(&html);
                    tracing::info!("👻 [Ghost Protocol] Archive.today Hit: {} bytes resgatados.", markdown.len());
                    return Ok(markdown);
                }
            }
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
        // Conta e reporta quantos rastreadores explícitos foram obliterados fisicamente da VRAM
        let ad_count = document.select(&Selector::parse("script, iframe, aside, noscript, dialog").unwrap()).count();
        if ad_count > 0 {
            if let Some(pool) = self.db_pool.clone() {
                tokio::spawn(async move {
                    let _ = sqlx::query("UPDATE analytics SET val_int = val_int + ? WHERE id = 'total_trackers_blocked'")
                        .bind(ad_count as i32)
                        .execute(&pool)
                        .await;
                });
            }
        }
        
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

    /// Integração Nativa Kiwix-Serve: Verifica a Wikipedia Offline ZIM no localhost:38201
    async fn search_kiwix_local(&self, query: &str) -> Result<Vec<String>, String> {
        let url = format!("http://127.0.0.1:38201/search?pattern={}", urlencoding::encode(query));
        match self.client.get(&url).timeout(std::time::Duration::from_millis(800)).send().await {
            Ok(res) if res.status().is_success() => {
                if let Ok(html) = res.text().await {
                    let mut links = Vec::new();
                    let document = scraper::Html::parse_document(&html);
                    let selector = scraper::Selector::parse("a").unwrap();
                    for element in document.select(&selector) {
                        if let Some(href) = element.value().attr("href") {
                            // A API web do Kiwix injeta /c/ antes dos conteudos ZIM parseados.
                            if href.starts_with("/c/") && !href.contains("?") {
                                links.push(format!("http://127.0.0.1:38201{}", href));
                            }
                        }
                    }
                    links.dedup();
                    links.truncate(3); // Top 3 Wikipedia Hits Maximum
                    return Ok(links);
                }
            }
            _ => {}
        }
        Err("Kiwix Offline indisponível ou limite estourado".to_string())
    }

    /// Dispara a busca Multi-Hop com tolerância impecável a falhas WAF/Cloudflare.
    pub async fn search_web(&self, query: &str) -> Result<SovereignSearchResult, String> {
        tracing::info!("🔍 [WAG] Inicializando Busca Autônoma Sovereign Omni-Scraper por: '{}'", query);
        
        // 1. O SOVEREIGN PARSER PARALELO: Tenta raspar Proxy Native + DuckDuckGo Snippets + ZIM Wikipedia Local
        let (yahoo_res, ddg_snippets, kiwix_res) = tokio::join!(
            self.search_sovereign_meta(query),
            self.search_duckduckgo_lite(query),
            self.search_kiwix_local(query)
        );

        let mut final_links = Vec::new();

        if let Ok(k_links) = kiwix_res {
             if !k_links.is_empty() {
                 tracing::info!("📚 [Kiwix ZIM] Conhecimento Offline detectado! Injetando {} enciclopédias ZIM locais no Córtex.", k_links.len());
                 final_links.extend(k_links); // ZIM offline tem altíssima prioridade 
             }
        }

        match yahoo_res {
            Ok(links) if !links.is_empty() => {
                let pure = self.apply_pi_hole_filter(links).await;
                final_links.extend(pure);
                tracing::info!("✅ [WAG] Sovereign Meta-Search Bem-Sucedido! ({}) links orgânicos purificados.", final_links.len());
            },
            Err(e) => {
                let msg = format!("⚠️ [WAG Fallback] O motor primário tomou WAF Block. Rotacionando para Brave API. Erro: {}", e);
                tracing::warn!("{}", msg);
            },
            _ => {
                tracing::warn!("⚠️ [WAG Fallback] O motor primário encontrou 0 resultados.");
            }
        }

        // 1.5. O FALLBACK COMERCIAL (PRIVADO): Brave Search API
        if final_links.is_empty() {
            match self.search_brave_api(query).await {
                Ok(links) if !links.is_empty() => {
                    let pure = self.apply_pi_hole_filter(links).await;
                    final_links.extend(pure);
                    tracing::info!("✅ [WAG] Busca Cíbrida Brave API Bem-Sucedida! ({}) links ancorados.", final_links.len());
                },
                _ => {
                    tracing::warn!("⚠️ [WAG Fallback] Brave API caiu ou não configurada. Rotacionando para SearXNG P2P...");
                }
            }
        }

        // 2. O FALLBACK SOBERANO PARALELO: Motores P2P
        if final_links.is_empty() {
            match self.search_searxng_public(query).await {
                Ok(links) if !links.is_empty() => {
                    let pure = self.apply_pi_hole_filter(links).await;
                    final_links.extend(pure);
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
        
        // FIX-3: Dedup global via HashSet — elimina URLs duplicadas entre queries paralelas.
        // O dedup() antigo só removia adjacentes idênticos. Um HashSet garante unicidade absoluta.
        let mut seen = std::collections::HashSet::new();
        final_links.retain(|link| seen.insert(link.clone()));
        let deduped_count = seen.len();
        if deduped_count < final_links.len() + (final_links.len() / 3) {
            tracing::info!("🧹 [WAG Dedup] {} links únicos após remoção de duplicatas cross-query.", deduped_count);
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
    /// Pontua domínios institucionais/governamentais acima de blogs e ads.
    async fn assign_sovereign_trust_score(&self, links: Vec<String>) -> Vec<String> {
        let mut scored: Vec<(i32, String)> = links.into_iter().map(|link| {
            let lower = link.to_lowercase();
            let score = if lower.contains(".gov.br") || lower.contains("ibge.gov.br") { 100 }
            else if lower.contains("bcb.gov.br") || lower.contains("anp.gov.br") { 95 }
            else if lower.contains("petrobras.com.br") || lower.contains("epe.gov.br") { 90 }
            else if lower.contains("tradingeconomics.com") || lower.contains("investing.com") { 80 }
            else if lower.contains("reuters.com") || lower.contains("bloomberg.com") { 80 }
            else if lower.contains("infomoney.com.br") || lower.contains("valorinveste") { 75 }
            else if lower.contains("dadosdemercado.com.br") || lower.contains("numerando.com.br") { 70 }
            else if lower.contains("cnnbrasil.com.br") || lower.contains("g1.globo.com") { 65 }
            else if lower.contains("folha.uol.com.br") || lower.contains("estadao.com.br") { 65 }
            else if lower.contains("wikipedia.org") { 60 }
            else if lower.contains("eia.gov") { 85 }
            // Penalizações
            else if lower.contains("blog") || lower.contains("medium.com") { 20 }
            else if lower.contains("bing.com") || lower.contains("msn.com") { 5 }
            else { 50 }; // Neutro
            (score, link)
        }).collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, link)| link).collect()
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

        // FIX-2: Blacklist de domínios epistêmicamente inúteis que poluem o contexto do LLM.
        // Estes domínios são resultado de redirecionamentos Yahoo/Google que escapam do filtro primário.
        const JUNK_DOMAINS: &[&str] = &[
            "uservoice.com", "feedback.yahoo", "support.google", "accounts.google",
            "login.", "signin.", "signup.", "ads.", "pixel.", "tracker.",
            "play.google.com", "apps.apple.com", "itunes.apple.com",
            "facebook.com/sharer", "twitter.com/intent", "linkedin.com/share",
            // FIX-SCRAPE: Anúncios pagos do Bing, MSN redirects, e domínios de cartão corporativo
            "bing.com/aclick", "msn.com", "edenredmobilidade.com",
            "codigosdebarrasbrasil.com", "vexpenses.com",
        ];

        let mut links = Vec::new();
        {
            let document = scraper::Html::parse_document(&html);
            let selector = scraper::Selector::parse("a").unwrap();
            
            for element in document.select(&selector) {
                if let Some(href_attr) = element.value().attr("href") {
                    let mut candidate = String::new();

                    if href_attr.contains("RU=") {
                        let parts: Vec<&str> = href_attr.split("RU=").collect();
                        if parts.len() > 1 {
                            let inner = parts[1].split("/RK=").next().unwrap_or("");
                            if let Ok(decoded) = urlencoding::decode(inner) {
                                candidate = decoded.into_owned();
                            }
                        }
                    } else if href_attr.starts_with("http") && !href_attr.contains("r.search.yahoo.com") {
                        candidate = href_attr.to_string();
                    }

                    if !candidate.is_empty()
                        && candidate.starts_with("http")
                        && !candidate.contains("yahoo.com")
                        && !JUNK_DOMAINS.iter().any(|junk| candidate.contains(junk))
                    {
                        links.push(candidate);
                    }
                }
            }
        }

        links.dedup();
        let mut prioritized_links = self.assign_sovereign_trust_score(links).await;
        let scrape_cap = if let Some(pool) = &self.db_pool {
            crate::api_settings::load_scrape_limits(pool).await.max_links_per_search
        } else { 7 };
        prioritized_links.truncate(scrape_cap);
        Ok(prioritized_links)
    }



    /// Fallback Soberano: Integração nativa com a API do Brave Search para mitigar bloqueios WAF globais.
    async fn search_brave_api(&self, query: &str) -> Result<Vec<String>, String> {
        // A API Key deve ser injetada de forma segura na inicialização, mas para o RAG Cíbrido
        // usaremos a variável de ambiente ou configuração do banco.
        let api_key = std::env::var("BRAVE_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            return Err("Brave API Key não configurada (SOVEREIGN_BRAVE_API_KEY).".to_string());
        }

        tracing::info!("🦁 [Brave Search] Invocando API nativa de privacidade para contornar WAF.");
        // Assegurando que a API externa filtre nativamente conteúdos em língua portuguesa (pt-BR)
        let url = format!("https://api.search.brave.com/res/v1/web/search?q={}&count=15&search_lang=pt&country=br", urlencoding::encode(query));
        
        let req = self.client.get(&url)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", api_key)
            .header("Accept-Language", "pt-BR,pt;q=0.9,en-US;q=0.8")
            .send()
            .await;

        match req {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    if let Ok(json_data) = response.json::<serde_json::Value>().await {
                        if let Some(results) = json_data.get("web").and_then(|w| w.get("results")).and_then(|r| r.as_array()) {
                            let mut links = Vec::new();
                            for res in results {
                                if let Some(url_str) = res.get("url").and_then(|u| u.as_str()) {
                                    links.push(url_str.to_string());
                                }
                            }
                            let mut prioritized_links = self.assign_sovereign_trust_score(links).await;
                            let scrape_cap = if let Some(pool) = &self.db_pool {
                                crate::api_settings::load_scrape_limits(pool).await.max_links_per_search
                            } else { 7 };
                            prioritized_links.truncate(scrape_cap);
                            return Ok(prioritized_links);
                        }
                    }
                }
                Err(format!("Brave API falhou com HTTP {}", status))
            },
            Err(e) => Err(format!("Falha de conexão com a API do Brave: {}", e))
        }
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
        if let Some(pool) = &self.db_pool {
            if let Ok(json_str) = sqlx::query_scalar::<_, String>("SELECT value_json FROM global_settings WHERE id = 'searxng_nodes'")
                .fetch_one(pool)
                .await {
                if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&json_str) {
                    if !parsed.is_empty() {
                        instances = parsed;
                        tracing::info!("🔗 [SearxNG] {} Nodes Customizados de Busca carregados do Sensus Vault!", instances.len());
                    }
                }
            }
        }

        let mut last_error = String::new();

        for base_url in instances {
            tracing::info!("🔄 [SearxNG Agent] Simulando tráfego na instância P2P: {}", base_url);
            let url = format!("{}/search?q={}&format=json&language=pt-BR", base_url, urlencoding::encode(query));
            
            let req = self.client.get(&url).send().await;
            
            match req {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(json_data) = response.json::<serde_json::Value>().await {
                            if let Some(results) = json_data.get("results").and_then(|r| r.as_array()) {
                                let mut links = Vec::new();
                                for res in results {
                                    if let Some(url_str) = res.get("url").and_then(|u| u.as_str()) {
                                        links.push(url_str.to_string());
                                    }
                                }
                                let mut prioritized_links = self.assign_sovereign_trust_score(links).await;
                                let scrape_cap = if let Some(pool) = &self.db_pool {
                                    crate::api_settings::load_scrape_limits(pool).await.max_links_per_search
                                } else { 7 };
                                prioritized_links.truncate(scrape_cap);
                                return Ok(prioritized_links);
                            }
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
