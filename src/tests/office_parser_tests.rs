//! ============================================================
//! sp-service — Office Parser Tests
//! Tests for document parsing (DOCX, XLSX, PPTX, CSV, ODF)
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::office_parser::parse_file;
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
}
