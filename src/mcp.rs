use std::path::{Path, PathBuf};
use serde_json::json;

/// Retorna os Schemas da OpenAI / Vercel SDK convertidos das extensões do Model Context Protocol
/// Que o Cíbrido suportará em modo nativo via OS.
pub fn get_mcp_tools() -> Vec<serde_json::Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "mcp_list_directory",
                "description": "[MCP] Lista os arquivos e pastas dentro de um diretório protegido.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho relativo ou absoluto" }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "mcp_read_file",
                "description": "[MCP] Carrega o conteúdo extraído em texto de um arquivo interno.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho do arquivo a ser lido" }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "mcp_write_file",
                "description": "[MCP] Modifica ou cria um arquivo local no disco de forma determinística.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Diretório e nome do arquivo" },
                        "content": { "type": "string", "description": "O Código bruto ou texto a ser implementado" }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "mcp_deep_research",
                "description": "[MCP] Faz a varredura profunda de uma URL HTTP/HTTPS (Deep Research), limpa anúncios e devolve o artigo em sintaxe semântica Markdown pura.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "URL pública do site/livro a ser extraído." }
                    },
                    "required": ["url"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "mcp_transcribe_audio",
                "description": "[MCP] Transcreve arquivos de áudio locais usando o cluster ASR (Whisper). Excelente para processar voz humana.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho absoluto do arquivo de áudio local" }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "mcp_ocr_image",
                "description": "[MCP] Extrai todo o texto estruturado de uma imagem ou fotografia de documento usando a engine de Visão.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho absoluto do arquivo da imagem local" }
                    },
                    "required": ["path"]
                }
            }
        })
    ]
}

/// A "Sandbox Layer": Garante que o Agente não tente escapar do /home/user/workspace especificado
fn validate_safe_path(vault_root: &Path, target: &str) -> std::io::Result<PathBuf> {
    let mut resolved = vault_root.to_path_buf();
    let target_path = Path::new(target);
    
    if target_path.is_absolute() {
        if target_path.starts_with(vault_root) {
            resolved = target_path.to_path_buf();
        } else {
             return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "MCP Security Violation: Filepath Root Escaping Locked."));
        }
    } else {
        resolved.push(target);
    }
    
    // Normalização canônica (evita payloads tipo `../../etc/passwd`)
    if let Ok(canon) = std::fs::canonicalize(&resolved) {
        if canon.starts_with(vault_root) {
            return Ok(canon);
        }
    } else {
         // O arquivo pode não existir ainda (No caso do mcp_write_file).
         // Validamos apenas se a pasta PAI do arquivo novo está permitida.
         if let Some(parent) = resolved.parent() {
             if let Ok(canon_parent) = std::fs::canonicalize(parent) {
                 if canon_parent.starts_with(vault_root) {
                     return Ok(resolved);
                 }
             }
         }
    }
    
    Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "MCP Security Violation: Path unresolvable inside restricted scope."))
}

/// Resolutor unificado (O Executante) de todas as capacidades MCP providenciadas ao Ollama  
pub async fn execute_mcp_tool(state: &std::sync::Arc<crate::AppState>, tool_name: &str, args: &serde_json::Value) -> String {
    let vault_root = &state.vault_path;
    match tool_name {
        "mcp_list_directory" => {
            let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match validate_safe_path(vault_root, path_str) {
                Ok(safe_path) => {
                    match std::fs::read_dir(&safe_path) {
                        Ok(entries) => {
                            let mut results = Vec::new();
                            for entry in entries.flatten() {
                                let name = entry.file_name().to_string_lossy().to_string();
                                let tag = if entry.path().is_dir() { "[DIR]" } else { "[FILE]" };
                                results.push(format!("{} {}", tag, name));
                            }
                            format!("Directory Listing of {}:\n{}", path_str, results.join("\n"))
                        },
                        Err(e) => format!("MCP Error reading directory: {}", e)
                    }
                },
                Err(e) => format!("MCP Access Error: {}", e)
            }
        },
        "mcp_read_file" => {
            let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match validate_safe_path(vault_root, path_str) {
                Ok(safe_path) => {
                    std::fs::read_to_string(&safe_path).unwrap_or_else(|e| format!("MCP Error reading file: {}", e))
                },
                Err(e) => format!("MCP Access Error: {}", e)
            }
        },
        "mcp_write_file" => {
            let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            match validate_safe_path(vault_root, path_str) {
                Ok(safe_path) => {
                    match std::fs::write(&safe_path, content) {
                        Ok(_) => format!("Success. The Content was safely injected at {}.", path_str),
                        Err(e) => format!("MCP Error writing file: {}", e)
                    }
                },
                Err(e) => format!("MCP Access Error: {}", e)
            }
        },
        "mcp_deep_research" => {
            let url_str = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if url_str.is_empty() {
                return "MCP Error: URL param is strictly required for Web Augmented Generation.".to_string();
            }
            let engine = crate::research::DeepResearchEngine::new(Some(state.db.clone()), Some(state.adblock_engine.clone()), Some(state.vault_path.clone()));
            match engine.scrape_url(url_str).await {
                Ok(markdown) => {
                    let slug = url_str.replace("https://", "").replace("http://", "").replace("/", "_");
                    let safe_slug = format!("{}.md", slug.chars().take(50).collect::<String>());
                    
                    let research_dir = vault_root.join("DeepResearch");
                    let _ = std::fs::create_dir_all(&research_dir);
                    let file_path = research_dir.join(safe_slug);
                    
                    match std::fs::write(&file_path, &markdown) {
                        Ok(_) => {
                            let preview: String = markdown.chars().take(1500).collect();
                            format!("Deep Research Extracted for [{}] and saved to Sensus Vault as {:?}.\n\n--- PREVIEW ---\n{}...\n\n(SUCCESS: The full {} byte document was securely saved to your disk for future RAG Retrieval.)", url_str, file_path, preview, markdown.len())
                        },
                        Err(e) => format!("MCP Error saving Deep Research file to disk: {}", e)
                    }
                },
                Err(e) => format!("MCP Web Scraping Error: {}", e)
            }
        },
        "mcp_transcribe_audio" => {
            let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match crate::multimodal::extract_text_from_audio(path_str).await {
                Ok(res) => {
                    if res.success {
                        format!("Audio Transcrito em {}: {}", res.language.unwrap_or("PT".into()), res.text.unwrap_or("".into()))
                    } else {
                        format!("Erro na transcrição: {:?}", res.error)
                    }
                },
                Err(e) => format!("Falha ao invocar pipeline Whisper: {}", e)
            }
        },
        "mcp_ocr_image" => {
            let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match crate::multimodal::extract_text_from_image(path_str).await {
                Ok(res) => {
                    if res.success {
                        format!("Texto Extraído da Imagem:\n{}", res.text.unwrap_or("".into()))
                    } else {
                        format!("Erro no OCR: {:?}", res.error)
                    }
                },
                Err(e) => format!("Falha ao invocar pipeline de Visão: {}", e)
            }
        },
        _ => format!("MCP Tool unrecognized by Engine: {}", tool_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn test_sandbox_allows_valid_paths() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        
        // Create a dummy file inside the pseudo-vault
        let file_path = root.join("allow_me.txt");
        File::create(&file_path).unwrap();

        let result = validate_safe_path(root, "allow_me.txt");
        assert!(result.is_ok(), "Sandbox should allow direct files inside the root");
        assert_eq!(result.unwrap(), fs::canonicalize(&file_path).unwrap());
    }

    #[test]
    fn test_sandbox_blocks_directory_traversal() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        
        let result = validate_safe_path(root, "../../../../etc/passwd");
        assert!(result.is_err(), "Sandbox MUST BLOCK relative directory traversal");
        
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn test_sandbox_allows_new_nested_file_creation() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        
        let nested_dir = root.join("src").join("internal");
        fs::create_dir_all(&nested_dir).unwrap();
        
        // Target is a file that doesn't exist yet, but in an allowed folder.
        let target = "src/internal/new_file.rs";
        
        let result = validate_safe_path(root, target);
        assert!(result.is_ok(), "Sandbox should allow future files if parent dir is safe");
    }
}
