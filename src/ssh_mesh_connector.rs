use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, error};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

lazy_static::lazy_static! {
    // Map with Local Port as key and Node Info as value ((IP, KeyPath))
    pub static ref ACTIVE_MESH_TUNNELS: Arc<Mutex<HashMap<u16, (String, String)>>> = Arc::new(Mutex::new(HashMap::<u16, (String, String)>::new()));
}

pub struct MeshConnector;

impl MeshConnector {
    /// Inicia um túnel reverso/direto persistente em background operado pelo próprio motor Rust.
    /// Funciona convertendo tráfego local invisível em requisições seguras da malha.
    pub async fn establish_mesh_tunnel(
        remote_ip: String, 
        remote_user: String, 
        key_path: String, 
        local_port: u16, 
        remote_port: u16
    ) -> Result<(), String> {
        
        let target_uri = format!("{}@{}", remote_user, remote_ip);
        info!("🕸️ [Sovereign Mesh] Tecendo túnel P2P Criptografado para {} (Port Fwd {} -> {})", target_uri, local_port, remote_port);

        let forward_str = format!("{}:127.0.0.1:{}", local_port, remote_port);

        let mut ssh_cmd = Command::new("ssh");
        ssh_cmd.arg("-N")                      // Do not execute a command, just forward ports
               .arg("-o").arg("StrictHostKeyChecking=accept-new")
               .arg("-o").arg("ServerAliveInterval=30")
               .arg("-o").arg("ServerAliveCountMax=3")
               .arg("-o").arg("ExitOnForwardFailure=yes") // Force spawn to fail if port is occupied
               .arg("-L").arg(&forward_str)
               .arg("-i").arg(&key_path)
               .arg(&target_uri)
               .stdin(Stdio::null())
               .stdout(Stdio::null())
               .stderr(Stdio::piped());

        match ssh_cmd.spawn() {
            Ok(mut child) => {
                info!("✅ [Sovereign Mesh] Túnel de Cobre Ativado. Endpoint {} da Malha conectado de forma invisível via porta {}.", target_uri, local_port);
                
                // Grava o túnel no Estado Global (Tuple com Chave)
                tokio::spawn({
                    let t_uri = target_uri.clone();
                    let t_key = key_path.clone(); // O.S Identifiers
                    async move {
                        ACTIVE_MESH_TUNNELS.lock().await.insert(local_port, (t_uri, t_key));
                    }
                });

                // Monitorador de Integridade da Malha (Heal Loop eventual entrará aqui)
                tokio::spawn(async move {
                    if let Ok(status) = child.wait().await {
                        error!("❌ [Sovereign Mesh] Alerta de Falha Estrutural! O Túnel P2P da porta {} colapsou. (Exit: {})", local_port, status);
                        ACTIVE_MESH_TUNNELS.lock().await.remove(&local_port);
                    }
                });

                Ok(())
            },
            Err(e) => {
                error!("❌ [Sovereign Mesh] Falha drástica ao invocar driver SSH nativo O.S: {}", e);
                Err(format!("Process Spawn Error: {}", e))
            }
        }
    }
}
