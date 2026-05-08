//! ============================================================
//! sp-service — Office Parser Tests
//! Tests for document parsing (DOCX, XLSX, PPTX, CSV, ODF)
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::office_parser::{
        parse_file, extract_text_from_xml, find_chartable_subgrid,
        generate_svg_bar_chart, escape_xml,
    };
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ─────────────────────────────────────────────────────────
    // CSV Parser Tests (via parse_file)
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_parse_file_csv_basic() {
        let mut temp_file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(temp_file, "nome,idade,cidade").unwrap();
        writeln!(temp_file, "João,30,SP").unwrap();
        writeln!(temp_file, "Maria,25,RJ").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("João") || text.contains("30") || text.contains("Maria"));
    }

    #[test]
    fn test_parse_file_csv_empty() {
        let mut temp_file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(temp_file, "").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        // Empty CSV should return empty or error gracefully
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_file_csv_with_quotes() {
        let mut temp_file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(temp_file, "nome,descricao").unwrap();
        writeln!(temp_file, r#""João","Descrição com ""aspas"""#).unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_file_csv_missing_file() {
        let result = parse_file("/caminho/invalido/arquivo.csv");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_file_csv_single_column() {
        let mut temp_file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(temp_file, "nome").unwrap();
        writeln!(temp_file, "João").unwrap();
        writeln!(temp_file, "Maria").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_file_csv_single_row() {
        let mut temp_file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(temp_file, "João,30,SP").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_file_csv_special_characters() {
        let mut temp_file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(temp_file, "símbolo,emoji").unwrap();
        writeln!(temp_file, "©,®").unwrap();
        writeln!(temp_file, "😀,🎉").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
    }

    // ─────────────────────────────────────────────────────────
    // parse_file Integration Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_parse_file_unknown_extension() {
        let mut temp_file = NamedTempFile::with_suffix(".xyz").unwrap();
        writeln!(temp_file, "Conteúdo de teste").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        // Unknown extension should fallback to text read
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Conteúdo"));
    }

    #[test]
    fn test_parse_file_txt_extension() {
        let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        writeln!(temp_file, "Arquivo texto simples").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("texto"));
    }

    #[test]
    fn test_parse_file_md_extension() {
        let mut temp_file = NamedTempFile::with_suffix(".md").unwrap();
        writeln!(temp_file, "# Markdown").unwrap();
        writeln!(temp_file, "Conteúdo **markdown**").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Markdown"));
    }

    #[test]
    fn test_parse_file_json_extension() {
        let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
        writeln!(temp_file, r#"{{"key": "value"}}"#).unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_file_xml_extension() {
        let mut temp_file = NamedTempFile::with_suffix(".xml").unwrap();
        writeln!(temp_file, r#"<?xml version="1.0"?>"#).unwrap();
        writeln!(temp_file, r#"<root><item>Test</item></root>"#).unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
    }

    // ─────────────────────────────────────────────────────────
    // Edge Cases and Error Handling
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_parse_file_invalid_docx() {
        // Criar arquivo .docx inválido (na verdade é texto)
        let mut temp_file = NamedTempFile::with_suffix(".docx").unwrap();
        writeln!(temp_file, "Isto não é um DOCX válido").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        // Deve falhar pois não é um ZIP/DOCX válido
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_file_path_with_spaces() {
        // Testar paths com espaços (suportado pelo tempfile)
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("arquivo com espaços.csv");
        
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "nome,valor").unwrap();
        writeln!(file, "teste,123").unwrap();
        file.flush().unwrap();

        let result = parse_file(file_path.to_str().unwrap());
        
        // Cleanup
        let _ = std::fs::remove_file(&file_path);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_file_unicode_content() {
        let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        writeln!(temp_file, "Português: João, São Paulo").unwrap();
        writeln!(temp_file, "Inglês: naïve, résumé").unwrap();
        writeln!(temp_file, "Espanhol: niño, año").unwrap();
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());
        
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("João") || text.contains("São Paulo"));
    }

    #[test]
    fn test_parse_file_large_content() {
        let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();

        // Escrever conteúdo grande (~10KB)
        for i in 0..1000 {
            writeln!(temp_file, "Linha {}: Conteúdo de teste para performance", i).unwrap();
        }
        temp_file.flush().unwrap();

        let result = parse_file(temp_file.path().to_str().unwrap());

        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Linha 500"));
        assert!(text.contains("Linha 999"));
    }

    // ─────────────────────────────────────────────────────────
    // extract_text_from_xml Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_extract_text_from_xml_docx_style() {
        let xml = r#"<?xml version="1.0"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body><w:p><w:r><w:t>Hello World</w:t></w:r></w:p>
            <w:p><w:r><w:t>Second paragraph</w:t></w:r></w:p>
            </w:body></w:document>"#;

        let result = extract_text_from_xml(xml, b"w:t");
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Hello World"));
        assert!(text.contains("Second paragraph"));
    }

    #[test]
    fn test_extract_text_from_xml_empty() {
        let xml = r#"<root><w:t></w:t></root>"#;
        let result = extract_text_from_xml(xml, b"w:t");
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_text_from_xml_multiple_tags() {
        let xml = r#"<root><w:t>One</w:t><w:t>Two</w:t><w:t>Three</w:t></root>"#;
        let result = extract_text_from_xml(xml, b"w:t");
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("One"));
        assert!(text.contains("Two"));
        assert!(text.contains("Three"));
    }

    #[test]
    fn test_extract_text_from_xml_with_formatting() {
        // Simula bold via w:b
        let xml = r#"<?xml version="1.0"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body><w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Bold text</w:t></w:r></w:p>
            </w:body></w:document>"#;

        let result = extract_text_from_xml(xml, b"w:t");
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Bold text"));
    }

    // ─────────────────────────────────────────────────────────
    // find_chartable_subgrid Tests
    // ────────────────────────────────────────────────────────

    #[test]
    fn test_find_chartable_subgrid_basic() {
        // Típica planilha com headers na linha 0 e dados a partir da linha 1
        let matrix = vec![
            vec!["".into(), "Q1".into(), "Q2".into(), "Q3".into()],
            vec!["Revenue".into(), "100".into(), "150".into(), "200".into()],
            vec!["Expenses".into(), "50".into(), "75".into(), "100".into()],
        ];

        let result = find_chartable_subgrid(&matrix);
        assert!(result.is_some());
        let sub = result.unwrap();
        assert_eq!(sub.col_start, 0);
        assert_eq!(sub.row_start, 0);
    }

    #[test]
    fn test_find_chartable_subgrid_no_numbers() {
        let matrix = vec![
            vec!["Nome".into(), "Cidade".into()],
            vec!["João".into(), "São Paulo".into()],
            vec!["Maria".into(), "Rio".into()],
        ];

        let result = find_chartable_subgrid(&matrix);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_chartable_subgrid_empty() {
        let matrix: Vec<Vec<String>> = vec![];
        let result = find_chartable_subgrid(&matrix);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_chartable_subgrid_single_row() {
        let matrix = vec![vec!["A".into(), "B".into(), "C".into()]];
        let result = find_chartable_subgrid(&matrix);
        // Single row sem dados numéricos abaixo não gera subgrid válido
        assert!(result.is_none() || result.unwrap().row_start == 0);
    }

    // ─────────────────────────────────────────────────────────
    // generate_svg_bar_chart Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_generate_svg_bar_chart_basic() {
        let matrix = vec![
            vec!["".into(), "Jan".into(), "Fev".into()],
            vec!["Produto A".into(), "100".into(), "150".into()],
            vec!["Produto B".into(), "80".into(), "120".into()],
        ];

        let svg = generate_svg_bar_chart(&matrix, "Vendas Trimestrais");

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Vendas Trimestrais"));
        assert!(svg.contains("Produto A"));
        assert!(svg.contains("Produto B"));
        assert!(svg.contains("Jan"));
        assert!(svg.contains("Fev"));
    }

    #[test]
    fn test_generate_svg_bar_chart_escaping() {
        let matrix = vec![
            vec!["".into(), "Col A".into()],
            vec!["R&D <Expenses>".into(), "100".into()],
        ];

        let svg = generate_svg_bar_chart(&matrix, "Test & <Chart>");

        assert!(svg.contains("Test &amp; &lt;Chart&gt;"));
        assert!(svg.contains("R&amp;D &lt;Expenses&gt;"));
    }

    // ─────────────────────────────────────────────────────────
    // escape_xml Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_escape_xml_basic() {
        assert_eq!(escape_xml("hello"), "hello");
    }

    #[test]
    fn test_escape_xml_ampersand() {
        assert_eq!(escape_xml("a & b"), "a &amp; b");
    }

    #[test]
    fn test_escape_xml_lt_gt() {
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn test_escape_xml_quotes() {
        assert_eq!(escape_xml("say \"hi\""), "say &quot;hi&quot;");
    }

    #[test]
    fn test_escape_xml_combined() {
        assert_eq!(escape_xml("a < b & c > d"), "a &lt; b &amp; c &gt; d");
    }

    #[test]
    fn test_escape_xml_empty() {
        assert_eq!(escape_xml(""), "");
    }
}
