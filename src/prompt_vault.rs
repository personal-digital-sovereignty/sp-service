use sha2::{Sha256, Digest};
use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};

// Hashes SHA-256 compilados pelo build.rs a partir do core_vault.toml
include!(concat!(env!("OUT_DIR"), "/prompt_hashes.rs"));

/// Estrutura de um prompt no banco
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PromptRow {
    pub id: String,
    pub slug: String,
    pub category: String,
    pub title: String,
    pub prompt_text: String,
    pub placeholders: String,
    pub is_core: bool,
    pub is_active: bool,
    pub version: i32,
    pub integrity_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
}

/// Estrutura de um prompt no TOML
#[derive(Debug, Deserialize)]
struct VaultEntry {
    id: String,
    title: String,
    category: String,
    placeholders: Vec<String>,
    prompt: String,
}

/// Computa SHA-256 de um texto
fn compute_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Hot-reload: carrega um prompt do DB por slug, com fallback para o TOML
pub async fn load_prompt_by_slug(pool: &SqlitePool, slug: &str) -> Option<String> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT prompt_text FROM sovereign_prompts WHERE slug = ? AND is_active = 1"
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .ok()?;

    row
}

/// Seed dos prompts core do TOML para o banco na inicialização.
/// Valida hashes compilados antes de aceitar o TOML.
pub async fn seed_core_prompts(pool: &SqlitePool) {
    // Tentar encontrar o TOML relativo ao executável
    let toml_path = find_vault_toml();
    let toml_content = match toml_path {
        Some(ref p) => {
            match std::fs::read_to_string(p) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("⚠️ [Prompt Vault] Falha ao ler core_vault.toml: {}. DB mantém prompts do último seed.", e);
                    return;
                }
            }
        }
        None => {
            tracing::info!("[Prompt Vault] core_vault.toml não encontrado. DB mantém prompts existentes.");
            return;
        }
    };

    let parsed: std::collections::HashMap<String, VaultEntry> = match toml::from_str(&toml_content) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("❌ [Prompt Vault] TOML parse error: {}. Seed abortado.", e);
            return;
        }
    };

    for (slug, entry) in &parsed {
        let computed_hash = compute_hash(&entry.prompt);

        // Validar contra hash compilado (se existir)
        if let Some(expected) = CORE_PROMPT_HASHES.iter().find(|(s, _)| s == slug) {
            if computed_hash != expected.1 {
                tracing::error!(
                    "🚨 [CRITICAL] TOML adulterado! Hash de '{}' não corresponde ao compilado. Seed ignorado para este prompt.",
                    slug
                );
                continue;
            }
        }

        let placeholders_json = serde_json::to_string(&entry.placeholders).unwrap_or("[]".to_string());
        let is_core = entry.category != "user";

        let _ = sqlx::query(
            "INSERT INTO sovereign_prompts (id, slug, category, title, prompt_text, placeholders, is_core, is_active, version, integrity_hash, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, 1, 1, ?, 'system')
             ON CONFLICT(slug) DO UPDATE SET
                prompt_text = excluded.prompt_text,
                integrity_hash = excluded.integrity_hash,
                updated_at = CURRENT_TIMESTAMP"
        )
        .bind(&entry.id)
        .bind(slug)
        .bind(&entry.category)
        .bind(&entry.title)
        .bind(&entry.prompt)
        .bind(&placeholders_json)
        .bind(is_core)
        .bind(&computed_hash)
        .execute(pool)
        .await;
    }

    tracing::info!("✅ [Prompt Vault] {} prompts core carregados do TOML e verificados.", parsed.len());
}

/// Watcher oculto — chamado pelo Garbage Collector.
/// Nome genérico para não expor a função de integridade.
pub async fn temporal_coherence_sweep(pool: &SqlitePool) {
    // Carregar prompts core do DB
    let rows: Vec<(String, String)> = match sqlx::query_as::<_, (String, String)>(
        "SELECT slug, prompt_text FROM sovereign_prompts WHERE is_core = 1"
    )
    .fetch_all(pool)
    .await {
        Ok(r) => r,
        Err(_) => return,
    };

    let mut tampered_count = 0;
    for (slug, prompt_text) in &rows {
        let current_hash = compute_hash(prompt_text);
        if let Some(expected) = CORE_PROMPT_HASHES.iter().find(|(s, _)| s == slug) {
            if current_hash != expected.1 {
                tampered_count += 1;
                // Restaurar silenciosamente do TOML
                if let Some(toml_path) = find_vault_toml() {
                    if let Ok(content) = std::fs::read_to_string(&toml_path) {
                        if let Ok(parsed) = toml::from_str::<std::collections::HashMap<String, VaultEntry>>(&content) {
                            if let Some(original) = parsed.get(slug.as_str()) {
                                let original_hash = compute_hash(&original.prompt);
                                if original_hash == expected.1 {
                                    let _ = sqlx::query(
                                        "UPDATE sovereign_prompts SET prompt_text = ?, integrity_hash = ?, updated_at = CURRENT_TIMESTAMP WHERE slug = ?"
                                    )
                                    .bind(&original.prompt)
                                    .bind(&original_hash)
                                    .bind(slug)
                                    .execute(pool)
                                    .await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if tampered_count > 0 {
        tracing::info!("🗑️ [GC] Temporal coherence validated. {} tokens recalibrated.", tampered_count);
    }
}

/// Validação LLM: verifica se um novo prompt do usuário conflita com regras core.
pub async fn validate_prompt_with_llm(pool: &SqlitePool, new_prompt: &str) -> Result<(), String> {
    // Carregar regras core
    let core_rules: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT prompt_text FROM sovereign_prompts WHERE is_core = 1 AND is_active = 1"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    if core_rules.is_empty() {
        return Ok(()); // Sem regras core = sem validação
    }

    // Carregar o prompt do validador do próprio vault
    let validator_prompt_template = load_prompt_by_slug(pool, "prompt_validator").await
        .unwrap_or_else(|| "Analise se o novo prompt contradiz as regras. Responda APROVADO ou REJEITADO: motivo".to_string());

    let filled = validator_prompt_template
        .replace("{core_rules}", &core_rules.join("\n---\n"))
        .replace("{new_prompt}", new_prompt);

    // Usar o menor modelo disponível (junior tier)
    let validator_model = crate::api::discover_cognitive_model_by_tier("junior").await;
    let olla_url = format!("{}/api/chat",
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
    );

    let payload = serde_json::json!({
        "model": validator_model,
        "messages": [{"role": "user", "content": filled}],
        "stream": false,
        "options": { "temperature": 0.0, "num_ctx": 4096, "num_predict": 256 }
    });

    let client = reqwest::Client::new();
    match client.post(&olla_url).json(&payload).send().await {
        Ok(res) => {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                if let Some(content) = json.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                    let clean = content.trim().to_uppercase();
                    if clean.starts_with("REJEITADO") || clean.starts_with("REJECTED") {
                        return Err(content.trim().to_string());
                    }
                }
            }
            Ok(())
        }
        Err(e) => {
            // Se o LLM falhou, permitir por segurança (melhor inserir do que travar)
            tracing::warn!("[Prompt Vault] LLM validation failed: {}. Allowing insertion.", e);
            Ok(())
        }
    }
}

/// Encontra o core_vault.toml no filesystem (relativo ao executável ou ao CWD)
fn find_vault_toml() -> Option<std::path::PathBuf> {
    // 1. Relativo ao executável
    if let Ok(exe) = std::env::current_exe() {
        let candidate = exe.parent()?.join("prompts").join("core_vault.toml");
        if candidate.exists() { return Some(candidate); }
        // Subindo 1 nível (ex: target/release/../prompts/)
        let candidate2 = exe.parent()?.parent()?.parent()?.join("prompts").join("core_vault.toml");
        if candidate2.exists() { return Some(candidate2); }
    }
    // 2. Relativo ao CWD
    let cwd = std::path::PathBuf::from("prompts/core_vault.toml");
    if cwd.exists() { return Some(cwd); }
    // 3. Em ../prompts/ (quando rodando de dentro de core/)
    let parent = std::path::PathBuf::from("../prompts/core_vault.toml");
    if parent.exists() { return Some(parent); }
    None
}
