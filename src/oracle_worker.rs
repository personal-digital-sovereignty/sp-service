// ============================================================
// Sovereign Pair — Oracle Cloud Worker
//
// Executa Python workers na instância Oracle via SSH exec.
// Estratégia: SSH exec direto — sem portas expostas, sem HTTP.
//
// Fluxo:
//   1. Ler config oracle_node de global_settings
//   2. ssh ubuntu@ORACLE "~/sovereign-venv/bin/python ~/sovereign-workers/<script> <args>"
//   3. Capturar stdout como resultado
//   4. Fallback automático para execução local se disabled/falha
// ============================================================

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tracing::{info, warn, error};

/// Configuração do nó Oracle lida do global_settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleNodeConfig {
    pub ip: String,
    pub user: String,
    pub key_path: String,
    pub ollama_tunnel_port: u16,
    pub enabled: bool,
    /// Hook para Cold Storage — preparado, sem implementação total
    pub cold_storage_enabled: bool,
    pub workers_dir: String,
    pub venv_path: String,
}

impl Default for OracleNodeConfig {
    fn default() -> Self {
        Self {
            ip: String::new(),
            user: "ubuntu".to_string(),
            key_path: "~/.ssh/id_ed25519".to_string(),
            ollama_tunnel_port: 41434,
            enabled: false,
            cold_storage_enabled: false,
            workers_dir: "~/sovereign-workers".to_string(),
            venv_path: "~/sovereign-venv/bin/python".to_string(),
        }
    }
}

impl OracleNodeConfig {
    /// Resolve ~ para o home directory real do usuário
    pub fn resolve_key_path(&self) -> String {
        if self.key_path.starts_with('~') {
            if let Ok(home) = std::env::var("HOME") {
                return self.key_path.replacen('~', &home, 1);
            }
        }
        self.key_path.clone()
    }

    pub fn ssh_target(&self) -> String {
        format!("{}@{}", self.user, self.ip)
    }

    pub fn is_ready(&self) -> bool {
        self.enabled && !self.ip.is_empty() && !self.user.is_empty()
    }
}

/// Carrega OracleNodeConfig do banco de dados
pub async fn load_oracle_config(db: &sqlx::SqlitePool) -> OracleNodeConfig {
    let row = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'oracle_node'")
        .fetch_optional(db)
        .await
        .ok()
        .flatten();

    if let Some(row) = row {
        use sqlx::Row;
        if let Ok(json_str) = row.try_get::<String, _>("value_json") {
            if let Ok(config) = serde_json::from_str::<OracleNodeConfig>(&json_str) {
                return config;
            }
        }
    }
    OracleNodeConfig::default()
}

/// Resultado da execução de um worker
#[derive(Debug)]
pub struct WorkerResult {
    pub stdout: String,
    pub success: bool,
    pub execution_site: WorkerSite,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkerSite {
    Oracle,
    Local,
}

impl std::fmt::Display for WorkerSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerSite::Oracle => write!(f, "Oracle Cloud"),
            WorkerSite::Local => write!(f, "Local"),
        }
    }
}

/// Escapa strings para uso seguro no Shell Bash remoto (Previne CWE-78)
pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace("'", "'\\''"))
}

/// Executa um script Python no Oracle via SSH exec.
/// Retorna o stdout completo ou um erro descritivo.
pub async fn ssh_exec_worker(
    config: &OracleNodeConfig,
    script_name: &str,
    args: &[&str],
) -> Result<String, String> {
    let key_path = config.resolve_key_path();
    let safe_args = args.iter().map(|a| shell_escape(a)).collect::<Vec<_>>().join(" ");
    let safe_script = shell_escape(script_name);
    
    // config.venv_path e workers_dir são validados no momento do salvamento (no Axum Handler)
    // para garantir que não contêm metas de injeção.
    let remote_cmd = format!(
        "{} {}/{} {}",
        config.venv_path,
        config.workers_dir,
        safe_script,
        safe_args
    );

    info!(
        "☁️ [Oracle Worker] Executing '{}' on {} via SSH exec",
        script_name,
        config.ssh_target()
    );

    let output = tokio::process::Command::new("ssh")
        .arg("-i").arg(&key_path)
        .arg("-o").arg("StrictHostKeyChecking=accept-new")
        .arg("-o").arg("ConnectTimeout=15")
        .arg("-o").arg("BatchMode=yes")
        .arg(config.ssh_target())
        .arg(remote_cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("SSH spawn error: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        info!("☁️ [Oracle Worker] '{}' completed OK ({} bytes)", script_name, stdout.len());
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Remote execution failed: {}", &stderr[..stderr.len().min(300)]))
    }
}

/// Smart dispatcher: tenta Oracle primeiro, cai para local em falha.
/// Retorna WorkerResult com o site real de execução.
pub async fn dispatch_worker(
    db: &sqlx::SqlitePool,
    script_name: &str,
    args: &[&str],
    local_fallback: impl std::future::Future<Output = Result<String, String>>,
) -> WorkerResult {
    let config = load_oracle_config(db).await;

    if config.is_ready() {
        match ssh_exec_worker(&config, script_name, args).await {
            Ok(output) => {
                return WorkerResult {
                    stdout: output,
                    success: true,
                    execution_site: WorkerSite::Oracle,
                };
            }
            Err(e) => {
                warn!("☁️ [Oracle Worker] Remote execution failed, falling back to local: {}", e);
            }
        }
    } else if config.enabled && config.ip.is_empty() {
        warn!("☁️ [Oracle Worker] oracle_node enabled but IP not configured — using local");
    }

    // Fallback local
    match local_fallback.await {
        Ok(output) => WorkerResult {
            stdout: output,
            success: true,
            execution_site: WorkerSite::Local,
        },
        Err(e) => {
            error!("☁️ [Oracle Worker] Local fallback also failed: {}", e);
            WorkerResult {
                stdout: String::new(),
                success: false,
                execution_site: WorkerSite::Local,
            }
        }
    }
}

/// Verifica conectividade e retorna status do nó Oracle
pub async fn ping_oracle_node(config: &OracleNodeConfig) -> OracleStatus {
    if !config.is_ready() {
        return OracleStatus {
            reachable: false,
            ollama_alive: false,
            ollama_models: vec![],
            latency_ms: 0,
            error: Some(if config.ip.is_empty() {
                "IP not configured".to_string()
            } else {
                "Oracle node disabled".to_string()
            }),
        };
    }

    let key_path = config.resolve_key_path();
    let t0 = std::time::Instant::now();

    // Testa SSH + verifica Ollama em um único round-trip
    let probe = tokio::process::Command::new("ssh")
        .arg("-i").arg(&key_path)
        .arg("-o").arg("StrictHostKeyChecking=accept-new")
        .arg("-o").arg("ConnectTimeout=8")
        .arg("-o").arg("BatchMode=yes")
        .arg(config.ssh_target())
        // Testa SSH e lista modelos Ollama em um único RTT
        .arg("ollama list 2>/dev/null && echo '---ALIVE---'")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    let latency_ms = t0.elapsed().as_millis() as u64;

    match probe {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let ollama_alive = stdout.contains("---ALIVE---");
            let models: Vec<String> = stdout.lines()
                .filter(|l| !l.starts_with("NAME") && !l.contains("---ALIVE---") && !l.is_empty())
                .map(|l| l.split_whitespace().next().unwrap_or("").to_string())
                .filter(|s| !s.is_empty())
                .collect();

            OracleStatus {
                reachable: true,
                ollama_alive,
                ollama_models: models,
                latency_ms,
                error: None,
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            OracleStatus {
                reachable: false,
                ollama_alive: false,
                ollama_models: vec![],
                latency_ms,
                error: Some(stderr[..stderr.len().min(200)].to_string()),
            }
        }
        Err(e) => OracleStatus {
            reachable: false,
            ollama_alive: false,
            ollama_models: vec![],
            latency_ms,
            error: Some(format!("SSH error: {}", e)),
        },
    }
}

#[derive(Debug, Serialize)]
pub struct OracleStatus {
    pub reachable: bool,
    pub ollama_alive: bool,
    pub ollama_models: Vec<String>,
    pub latency_ms: u64,
    pub error: Option<String>,
}

// ─── Oracle Auto-Provisioner (Master -> Replica SHA-256) ──────────────────

/// Sincroniza workers locais para o Oracle validando hash SHA-256. (GAP-OR-02)
/// Mestre: "./python_workers" locais (Sovereign Core)
/// Réplica: "workers_dir" no nó OCI remoto.
pub async fn provision_oracle_workers(config: &OracleNodeConfig) -> Result<(), String> {
    if !config.is_ready() {
        return Ok(());
    }
    
    let local_dir = std::path::Path::new("../python_workers");
    let fallback_dir = std::path::Path::new("python_workers");
    let target_dir = if local_dir.exists() {
        local_dir
    } else if fallback_dir.exists() {
        fallback_dir
    } else {
        warn!("☁️ [Oracle Provisioner] Local directory 'python_workers' not found.");
        return Ok(());
    };

    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    let mut entries: Vec<_> = std::fs::read_dir(target_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|res| res.ok())
        .collect();
    
    // Sort logic to guarantee deterministic hash
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "py") {
            if let Ok(content) = std::fs::read(&path) {
                hasher.update(&content);
            }
        }
    }
    let full_hash = format!("{:x}", hasher.finalize());
    let local_hash = if full_hash.is_empty() { String::new() } else { full_hash };

    let remote_cmd = format!("ls -1 {}/*.py 2>/dev/null | sort | xargs cat 2>/dev/null | sha256sum | awk '{{print $1}}'", config.workers_dir);
    let key_path = config.resolve_key_path();
    
    let probe = tokio::process::Command::new("ssh")
        .arg("-i").arg(&key_path)
        .arg("-o").arg("StrictHostKeyChecking=accept-new")
        .arg("-o").arg("BatchMode=yes")
        .arg(config.ssh_target())
        .arg(remote_cmd)
        .output()
        .await
        .map_err(|e| format!("SSH Hash Probe error: {}", e))?;

    let remote_hash = String::from_utf8_lossy(&probe.stdout).trim().to_string();

    if remote_hash == local_hash && !local_hash.is_empty() {
        let print_hash = if local_hash.len() > 8 { &local_hash[..8] } else { &local_hash };
        info!("☁️ [Oracle Provisioner] SHA-256 Validado ({}). Replica em sync nativo.", print_hash);
        return Ok(());
    }

    info!("☁️ [Oracle Provisioner] Diferença SHA-256 detectada. Acionando Mestre -> Réplica via Rsync...");

    // Cria o dir remoto de formatação pra previnir failure
    let _ = tokio::process::Command::new("ssh")
        .arg("-i").arg(&key_path)
        .arg("-o").arg("StrictHostKeyChecking=accept-new")
        .arg("-o").arg("BatchMode=yes")
        .arg(config.ssh_target())
        .arg(format!("mkdir -p {}", config.workers_dir))
        .output()
        .await;

    let rsync_ssh_arg = format!("ssh -i {} -o StrictHostKeyChecking=accept-new -o BatchMode=yes", key_path);
    let rsync_target = format!("{}:{}/", config.ssh_target(), config.workers_dir);
    let src_dir = format!("{}/", target_dir.to_string_lossy());

    let rsync_cmd = tokio::process::Command::new("rsync")
        .arg("-avz")
        .arg("--exclude").arg("__pycache__")
        .arg("-e").arg(rsync_ssh_arg)
        .arg(src_dir)
        .arg(rsync_target)
        .output()
        .await
        .map_err(|e| format!("Rsync command failed: {}", e))?;

    if rsync_cmd.status.success() {
        let print_hash = if local_hash.len() > 8 { &local_hash[..8] } else { &local_hash };
        info!("✅ [Oracle Provisioner] Sync concluído com mestre. Hash unificado em {}", print_hash);
        Ok(())
    } else {
        Err(format!("Rsync falhou: {}", String::from_utf8_lossy(&rsync_cmd.stderr)))
    }
}

// ─── Axum Handler: GET /v1/settings/oracle_node ───────────────────────────

pub async fn get_oracle_node_handler(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::AppState>>,
) -> axum::response::Json<serde_json::Value> {
    let config = load_oracle_config(&state.db).await;
    axum::response::Json(serde_json::to_value(&config).unwrap_or(serde_json::json!({})))
}

// ─── Axum Handler: POST /v1/settings/oracle_node ──────────────────────────

pub async fn set_oracle_node_handler(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::AppState>>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> axum::response::Json<serde_json::Value> {
    // 🛡️ SECURITY AUDIT & GAP PREVENTION
    // 1. Prevent raw SSH keys from ever being stored in the database.
    // The user must provide a *path*, not the *content* of the key.
    if let Some(key_path) = payload.get("key_path").and_then(|v| v.as_str()) {
        let key_lower = key_path.to_lowercase();
        if key_lower.contains("begin openssh private key") || 
           key_lower.contains("begin rsa private key") ||
           key_lower.contains("ssh-rsa aaa") ||
           key_lower.contains("ssh-ed25519 aaa") ||
           key_lower.contains("\n") ||
           key_path.len() > 255 {
            error!("🚨 [SECURITY GAP PREVENTED] Attempted to save a raw SSH Private Key instead of a file path in oracle_node config.");
            return axum::response::Json(serde_json::json!({
                "status": "error", 
                "message": "SECURITY ALERT: You must provide a FILE PATH to the SSH key (e.g. ~/.ssh/id_ed25519), NOT the raw key payload. The system prevented saving your private key to the database."
            }));
        }
    }

    // 2. Validate Command Injection paths
    // Prevent ; & | < > $ ` \ ' " space
    let invalid_chars = [';', '&', '|', '<', '>', '$', '`', '\\', '\'', '"', ' ', '\n', '\r', '\t'];
    
    if let Some(workers_dir) = payload.get("workers_dir").and_then(|v| v.as_str()) {
        if workers_dir.contains(&invalid_chars[..]) {
            return axum::response::Json(serde_json::json!({
                "status": "error", 
                "message": "SECURITY ALERT: workers_dir contains invalid shell characters. Only alphanumeric, dashes, dots, slashes and ~ are allowed."
            }));
        }
    }

    if let Some(venv_path) = payload.get("venv_path").and_then(|v| v.as_str()) {
        if venv_path.contains(&invalid_chars[..]) {
            return axum::response::Json(serde_json::json!({
                "status": "error", 
                "message": "SECURITY ALERT: venv_path contains invalid shell characters. Only alphanumeric, dashes, dots, slashes and ~ are allowed."
            }));
        }
    }

    // 2. Validate Schema (Technical Debt Prevention)
    // Avoid saving malformed JSON that will fail silently on load
    if serde_json::from_value::<OracleNodeConfig>(payload.clone()).is_err() {
        return axum::response::Json(serde_json::json!({
            "status": "error", 
            "message": "Invalid configuration schema. Must match OracleNodeConfig structure."
        }));
    }

    let json_str = payload.to_string();
    let result = sqlx::query(
        "INSERT INTO global_settings (id, value_json) VALUES ('oracle_node', ?)
         ON CONFLICT(id) DO UPDATE SET value_json = excluded.value_json"
    )
    .bind(&json_str)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            info!("☁️ [Oracle Node] Configuration updated cleanly and securely");
            axum::response::Json(serde_json::json!({"status": "ok"}))
        }
        Err(e) => axum::response::Json(serde_json::json!({"status": "error", "message": e.to_string()}))
    }
}

// ─── Axum Handler: GET /v1/mesh/oracle_status ─────────────────────────────

pub async fn oracle_status_handler(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::AppState>>,
) -> axum::response::Json<serde_json::Value> {
    let config = load_oracle_config(&state.db).await;
    let status = ping_oracle_node(&config).await;
    axum::response::Json(serde_json::to_value(&status).unwrap_or(serde_json::json!({})))
}

// ─── Cold Storage Hook (preparado, sem implementação total) ───────────────

/// Sincroniza dados locais para storage remoto Oracle.
///
/// # Cold Storage — Arquitetura Futura
///
/// Quando `cold_storage_enabled == true`:
/// - rsync do Vault local → remoto via SSH
/// - Snapshot SQLite → remoto a cada N horas
/// - Compressão zstd + rotação de N dias
///
/// Por ora este stub documenta o contrato sem implementação.
#[allow(dead_code)]
pub async fn sync_to_cold_storage(_config: &OracleNodeConfig) -> Result<(), String> {
    todo!("Cold Storage: implementar rsync + snapshot SQLite → Oracle quando ativado")
}
