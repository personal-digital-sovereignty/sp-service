use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, error, debug};
use reqwest::Client;
use crate::mesh_router::MeshRouter;

/// The structure that abstracts the OCI Sandbox connection.
pub struct SshGateway;

impl SshGateway {
    /// Executes a strictly isolated bash/python script on the Oracle Cloud VM.
    /// Captures Stdout and Stderr to feed back into the Sovereign Pair 'ReWOO Solver'.
    pub async fn execute_sandboxed_script(script_payload: &str, db: sqlx::SqlitePool) -> Result<String, String> {
        let target_uri: String;
        let key_path;

        let client = Client::new();
        // 1. TENTA ROTEAMENTO INTELIGENTE NA MALHA (Mesh Router P2P)
        if let Some((mesh_uri, mesh_key)) = MeshRouter::find_best_coder_node(&client).await {
            info!("🌐 [Zero-Trust Gateway] O Orquestrador P2P sequestrou o deploy! Executando Job no Nó da Malha: {}", mesh_uri);
            target_uri = mesh_uri;
            key_path = mesh_key;
        } else {
            // 2. FALLBACK PARA O BANCO DE DADOS LOCAL OCI
            // GAP-O02 FIXED: Reads from the Master OracleNodeConfig which intelligently merges `oracle_node` com o `secops_vault` CRUD.
            let config = crate::oracle_worker::load_oracle_config(&db).await;
            
            key_path = config.resolve_key_path();
            let target_ip = config.ip;
            let target_user = config.user;

            if target_ip.is_empty() || key_path.is_empty() {
                error!("❌ [Zero-Trust Gateway] Nenhum Nó de Malha Sandboxed encontrado E credenciais OCI do KMS ausentes.");
                return Err("Missing Zero-Trust Sandbox Parameters. No Mesh Nodes available and no KMS configs.".to_string());
            }
            target_uri = format!("{}@{}", target_user, target_ip);
            info!("🛡️ [Zero-Trust Gateway] Opening SSH Pipe to Oracle Cloud VM: {}", target_uri);
        }

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
