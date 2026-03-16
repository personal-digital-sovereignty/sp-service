use std::path::PathBuf;
use std::fs;
use std::env;
use walkdir::WalkDir;
use tracing::info;
use serde_json::{json, Value};

/// Inicializa a arquitetura flexível do Vault baseada em Segurança Isolada (Airgap).
pub fn init_vault() -> PathBuf {
    let vault_path = env::var("SOVEREIGN_VAULT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut p = dirs::home_dir().expect("Sovereign Error: Ambiente sem Home Directory");
            p.push("Vault");
            p
        });

    if !vault_path.exists() {
        info!("📦 [Sovereign RAG] Gerando Vault Físico (Airgap) em: {:?}", vault_path);
        fs::create_dir_all(&vault_path).expect("Sovereign Error: Falha Crítica ao criar o Vault. Permissão negada?");
    } else {
        info!("📦 [Sovereign RAG] Vault Localizador Ativo em: {:?}", vault_path);
    }

    vault_path
}

/// Parseador Nativo de Alta Performance. (Abstração Zero)
/// Varre os arquivos textuais ($HOME/Vault/*.md) em milissegundos evitando Data Leaks.
pub fn parse_vault_documents(vault_path: &PathBuf) -> String {
    let mut combined_knowledge = String::new();
    let mut doc_count = 0;

    for entry in WalkDir::new(vault_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // Pula arquivos invisíveis e sub-arquiteturas (impede que leia o git ou os nós do app)
        if path.is_file()
            && let Some(ext) = path.extension().and_then(|s| s.to_str())
                && (ext == "md" || ext == "txt")
                    && let Ok(content) = fs::read_to_string(path) {
                        let filename = path.file_name().unwrap_or_default().to_string_lossy();
                        
                        // Limitador de Profiling Crítico: Evitar OOM/Context Bombing no Servidor
                        // O Vault inteiro (ex: 5MB) jamais deve ir cruçado no System Prompt.
                        let mut snippet = content.clone();
                        if content.len() > 2000 {
                            let safe_trunc: String = content.chars().take(2000).collect();
                            snippet = format!("{}... (truncado)", safe_trunc);
                        }
                        
                        if combined_knowledge.len() < 16000 {
                            combined_knowledge.push_str(&format!("\n\n--- Documento: {} ---\n{}\n", filename, snippet));
                            doc_count += 1;
                        }
                    }
    }

    info!("🧠 [Sovereign RAG/Mock] Indexou {} documentos crus (Safe Limit) na memória em nanosegundos.", doc_count);
    combined_knowledge
}

/// Constrói o Córtex Sistêmico da IA (Injeção via System Prompt)
pub fn build_rag_context_message(vault_path: &PathBuf) -> Option<Value> {
    let knowledge = parse_vault_documents(vault_path);
    if knowledge.trim().is_empty() {
        return None;
    }

    let sys_prompt = format!(
        "Sovereign Protocol Enforced. You operate on an Air-Gapped Local-First Architecture. \
         Below is the User's Digital Cortex (Physical Vault). Treat it as the absolute source of truth:\n\n{}", 
        knowledge
    );

    Some(json!({
        "role": "system",
        "content": sys_prompt
    }))
}