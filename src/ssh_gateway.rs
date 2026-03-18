use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, error, debug};
use std::env;
use sqlx::Row;
use serde_json::Value;

/// The structure that abstracts the OCI Sandbox connection.
pub struct SshGateway;

impl SshGateway {
    /// Executes a strictly isolated bash/python script on the Oracle Cloud VM.
    /// Captures Stdout and Stderr to feed back into the Sovereign Pair 'ReWOO Solver'.
    pub async fn execute_sandboxed_script(script_payload: &str, db: sqlx::SqlitePool) -> Result<String, String> {
        
        // Fetch Zero-Trust Configs from Sovereign KMS (sqlite global_settings)
        let row = sqlx::query("SELECT value_json FROM global_settings WHERE id = 'system_settings'")
            .fetch_optional(&db)
            .await
            .map_err(|e| format!("Database connection err: {}", e))?;

        let mut target_ip = String::new();
        let mut target_user = String::new();
        let mut key_path = String::new();

        if let Some(r) = row {
            let val: String = r.get("value_json");
            let parsed: Value = serde_json::from_str(&val).unwrap_or(serde_json::json!({}));
            
            target_ip = parsed.get("oci_sandbox_ip").and_then(|v| v.as_str()).unwrap_or("").to_string();
            target_user = parsed.get("oci_sandbox_user").and_then(|v| v.as_str()).unwrap_or("").to_string();
            key_path = parsed.get("oci_sandbox_key").and_then(|v| v.as_str()).unwrap_or("").to_string();
        }

        if target_ip.is_empty() || target_user.is_empty() || key_path.is_empty() {
            error!("❌ [Zero-Trust Gateway] Credenciais OCI não estão declaradas no KMS do Banco.");
            return Err("Missing Oracle Cloud Connection Parameters in Sovereign KMS Settings.".to_string());
        }

        let target_uri = format!("{}@{}", target_user, target_ip);

        info!("🛡️ [Zero-Trust Gateway] Opening SSH Pipe to Oracle VM: {}", target_uri);
        debug!("🛡️ [Zero-Trust Gateway] SSH Key Auth: {}", key_path);

        // Dispara o subprocesso CLI do OpenSSH de forma nativa e injeta o Payload via Stdin
        let mut ssh_cmd = Command::new("ssh");
        ssh_cmd.arg("-o").arg("StrictHostKeyChecking=accept-new")
               .arg("-o").arg("ConnectTimeout=5")
               .arg("-i").arg(&key_path)
               .arg(&target_uri)
               .arg("bash")     // Roda como bash remotor
               .stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .stderr(Stdio::piped());

        let mut child = match ssh_cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                error!("❌ [Zero-Trust Gateway] SSH Fork Error: {}", e);
                return Err(format!("SSH Fork Error: {}", e));
            }
        };

        // Pipe o script gerado pelo O.S (The Coder) para dentro do terminal Ubuntu remoto
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            if let Err(e) = stdin.write_all(script_payload.as_bytes()).await {
                error!("❌ [Zero-Trust Gateway] Falha ao injetar script no Stdin SSH: {}", e);
            }
        }

        // Aguarda a execução final
        match child.wait_with_output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

                if output.status.success() {
                    info!("✅ [Zero-Trust Gateway] The Coder (Oracle) execution success.");
                    Ok(format!("STDOUT:\n{}", stdout))
                } else {
                    error!("⚠️ [Zero-Trust Gateway] The Coder script returned Exit Code != 0.");
                    Err(format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr))
                }
            }
            Err(e) => {
                error!("❌ [Zero-Trust Gateway] Process Wait Error: {}", e);
                Err(format!("Runtime Execution failed: {}", e))
            }
        }
    }
}
