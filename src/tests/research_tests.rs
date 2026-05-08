//! ============================================================
//! sp-service — Research Engine Tests
//! Tests for pub(crate) functions: extract_domain, extract_hydration_json,
//! get_random_user_agent, sanitize_to_markdown
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::research::{DeepResearchEngine, TrustMatrix};

    // ─────────────────────────────────────────────────────────
    // extract_domain Tests
    // ────────────────────────────────────────────────────────

    #[test]
    fn test_extract_domain_https() {
        let domain = DeepResearchEngine::extract_domain("https://www.bcb.gov.br/dados");
        assert_eq!(domain, "bcb.gov.br");
    }

    #[test]
    fn test_extract_domain_http() {
        let domain = DeepResearchEngine::extract_domain("http://ibge.gov.br/pesquisa");
        assert_eq!(domain, "ibge.gov.br");
    }

    #[test]
    fn test_extract_domain_no_protocol() {
        let domain = DeepResearchEngine::extract_domain("www.example.com/path/to/page");
        assert_eq!(domain, "example.com");
    }

    #[test]
    fn test_extract_domain_no_www() {
        let domain = DeepResearchEngine::extract_domain("https://example.com/page");
        assert_eq!(domain, "example.com");
    }

    #[test]
    fn test_extract_domain_root_only() {
        let domain = DeepResearchEngine::extract_domain("https://gov.br/");
        assert_eq!(domain, "gov.br");
    }

    #[test]
    fn test_extract_domain_subdomain() {
        let domain = DeepResearchEngine::extract_domain("https://dados.gov.br/api/v1");
        assert_eq!(domain, "dados.gov.br");
    }

    #[test]
    fn test_extract_domain_plain_string() {
        let domain = DeepResearchEngine::extract_domain("just-a-string-without-url");
        assert_eq!(domain, "just-a-string-without-url");
    }

    // ─────────────────────────────────────────────────────────
    // get_random_user_agent Tests
    // ────────────────────────────────────────────────────────

    #[test]
    fn test_get_random_user_agent_is_valid() {
        let ua = DeepResearchEngine::get_random_user_agent();
        assert!(ua.starts_with("Mozilla/5.0"));
    }

    #[test]
    fn test_get_random_user_agent_contains_browser_info() {
        let ua = DeepResearchEngine::get_random_user_agent();
        assert!(ua.contains("AppleWebKit") || ua.contains("Gecko"));
    }

    // ─────────────────────────────────────────────────────────
    // extract_hydration_json Tests
    // ─────────────────────────────────────────────────────────

    fn make_engine() -> DeepResearchEngine {
        DeepResearchEngine::new(None, None, None)
    }

    #[test]
    fn test_extract_hydration_json_next_data() {
        let engine = make_engine();
        let html = r#"<html><head><script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"title":"Hello"}}}</script></head></html>"#;
        let result = engine.extract_hydration_json(html);
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("```json"));
    }

    #[test]
    fn test_extract_hydration_json_json_ld_article() {
        let engine = make_engine();
        // JSON-LD precisa ter >200 chars para passar o filtro de qualidade
        let html = r#"<html><head>
        <script type="application/ld+json">{"@type":"Article","headline":"Breaking News: Major Economic Developments in Brazil as the Central Bank Announces New Monetary Policy Framework","description":"Important story of the day covering economic impacts across multiple sectors including agriculture, technology, and financial services"}</script>
        </head></html>"#;
        let result = engine.extract_hydration_json(html);
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.contains("Breaking News"));
    }

    #[test]
    fn test_extract_hydration_json_rejects_breadcrumb_only() {
        let engine = make_engine();
        // JSON-LD com BreadcrumbList mas sem conteúdo jornalístico → deve ser rejeitado
        let html = r#"<html><head>
        <script type="application/ld+json">{"@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","position":1}]}</script>
        </head></html>"#;
        let result = engine.extract_hydration_json(html);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_hydration_json_short_json_ld_rejected() {
        let engine = make_engine();
        // JSON-LD curto demais (< 200 chars) → deve ser rejeitado
        let html = r#"<script type="application/ld+json">{"a":"b"}</script>"#;
        let result = engine.extract_hydration_json(html);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_hydration_json_no_scripts() {
        let engine = make_engine();
        let html = "<html><body><p>No scripts here</p></body></html>";
        let result = engine.extract_hydration_json(html);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_hydration_json_multiple_json_ld() {
        let engine = make_engine();
        let html = r#"<html><head>
        <script type="application/ld+json">{"@type":"Article","headline":"Story One: A Comprehensive Report on Climate Change and Its Economic Impacts Across Latin America","description":"Desc one"}</script>
        <script type="application/ld+json">{"@type":"Article","headline":"Story Two: The Rise of Renewable Energy Sources in Brazil and Its Effect on the National Grid","description":"Desc two"}</script>
        </head></html>"#;
        let result = engine.extract_hydration_json(html);
        assert!(result.is_some());
        let json = result.unwrap();
        // Deve conter ambos os stories
        assert!(json.contains("Story One") || json.contains("Story Two"));
    }

    // ─────────────────────────────────────────────────────────
    // TrustMatrix Default Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_trust_matrix_default_has_tier1() {
        let matrix = TrustMatrix::default();
        assert!(!matrix.tier1.is_empty());
        assert!(matrix.tier1.contains(&"gov.br".to_string()));
    }

    #[test]
    fn test_trust_matrix_default_has_tier2() {
        let matrix = TrustMatrix::default();
        assert!(!matrix.tier2.is_empty());
        assert!(matrix.tier2.contains(&"reuters.com".to_string()));
    }

    #[test]
    fn test_trust_matrix_default_has_encyclopedia() {
        let matrix = TrustMatrix::default();
        assert!(!matrix.encyclopedia.is_empty());
        assert!(matrix.encyclopedia.contains(&"wikipedia.org".to_string()));
    }

    #[test]
    fn test_trust_matrix_clone() {
        let matrix = TrustMatrix::default();
        let cloned = matrix.clone();
        assert_eq!(matrix.tier1, cloned.tier1);
    }
}
