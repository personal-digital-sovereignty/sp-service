#[cfg(test)]
mod tests {
    use crate::office_parser::parse_file;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_csv_basic() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let csv_data = "nome,idade,cidade\nJoão,30,SP\nMaria,25,RJ";
        temp_file.write_all(csv_data.as_bytes()).unwrap();
        
        let path = temp_file.path().with_extension("csv");
        std::fs::rename(temp_file.path(), &path).unwrap();

        let result = parse_file(path.to_str().unwrap()).unwrap();
        
        assert!(result.contains("| nome | idade | cidade |"));
        assert!(result.contains("| João | 30 | SP |"));
        assert!(result.contains("| Maria | 25 | RJ |"));
    }

    #[test]
    fn test_parse_csv_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().with_extension("csv");
        std::fs::rename(temp_file.path(), &path).unwrap();

        let result = parse_file(path.to_str().unwrap()).unwrap();
        assert_eq!(result.trim(), "|  |\n|  |");
    }

    #[test]
    fn test_parse_csv_with_quotes() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let csv_data = "nome,descricao\n\"João\",\"Descrição com aspas\"";
        temp_file.write_all(csv_data.as_bytes()).unwrap();
        
        let path = temp_file.path().with_extension("csv");
        std::fs::rename(temp_file.path(), &path).unwrap();

        let result = parse_file(path.to_str().unwrap()).unwrap();
        assert!(result.contains("| nome | descricao |"));
        assert!(result.contains("| João | Descrição com aspas |"));
    }

    #[test]
    fn test_parse_csv_malformed() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let csv_data = "nome,idade\nJoão";  // Missing column
        temp_file.write_all(csv_data.as_bytes()).unwrap();
        
        let path = temp_file.path().with_extension("csv");
        std::fs::rename(temp_file.path(), &path).unwrap();

        let result = parse_file(path.to_str().unwrap());
        // Depending on csv crate flexible=true, it might be ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_fallback_txt() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let txt_data = "Este é um texto simples\nCom duas linhas.";
        temp_file.write_all(txt_data.as_bytes()).unwrap();
        
        let path = temp_file.path().with_extension("txt");
        std::fs::rename(temp_file.path(), &path).unwrap();

        let result = parse_file(path.to_str().unwrap()).unwrap();
        assert_eq!(result, txt_data);
    }
    
    #[test]
    fn test_parse_invalid_file() {
        let result = parse_file("/caminho/invalido/qnaoisjd/invalid.docx");
        assert!(result.is_err());
    }
}
