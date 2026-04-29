use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

// 🕸️ **Sovereign Mesh | Malha P2P Criptografada**
// 
// Gerencia túneis SSH persistentes que conectam múltiplos Nós Sovereign.
// Permite que o tráfego de inferência e sincronização de conhecimento flua 
// de forma segura por redes públicas sem exposição de portas no firewall.
lazy_static::lazy_static! {
    /// Mapa de Túneis Ativos: Porta Local -> (URI Remota, Caminho da Chave).
    pub static ref ACTIVE_MESH_TUNNELS: Arc<Mutex<HashMap<u16, (String, String)>>> = Arc::new(Mutex::new(HashMap::<u16, (String, String)>::new()));
}

pub struct MeshConnector;

impl MeshConnector {
    /// 🛠️ **Establish Mesh Tunnel | Port Forwarding Seguro**
    /// 
    /// Inicia um processo SSH nativo em background para criar um túnel de porta.
    /// - **local_port**: Porta no host local que será mapeada.
    /// - **remote_port**: Porta no host remoto onde o serviço (ex: Ollama) escuta.
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

/// ☁️ **Oracle Node Watchdog | Gestão de Offload Cloud**
/// 
/// Monitora continuamente a configuração do Nó Oracle no SQLite.
/// Implementa **Hot-Reload Connectivity**: se o usuário alterar o IP ou 
/// desabilitar o nó na UI, o Watchdog derruba o túnel atual e sincroniza 
/// o estado imediatamente sem interrupção do serviço principal.
pub async fn auto_connect_oracle_node(db: sqlx::SqlitePool) {
    tokio::spawn(async move {
        let mut current_child: Option<tokio::process::Child> = None;
        let mut active_port: Option<u16> = None;
        let mut last_config = crate::oracle_worker::load_oracle_config(&db).await;

        loop {
            // Hot-reload: ler config recente do SQLite a cada ciclo
            let config = crate::oracle_worker::load_oracle_config(&db).await;
            
            // Detecta mutações na configuração feitas pelo usuário via Frontend
            let config_mutated = config.ip != last_config.ip 
                || config.enabled != last_config.enabled
                || config.ollama_tunnel_port != last_config.ollama_tunnel_port
                || config.user != last_config.user
                || config.key_path != last_config.key_path;

            // Se houve mutação, matar a conexão atual para forçar reconexão limpa
            if config_mutated {
                if let Some(mut child) = current_child.take() {
                    info!("☁️ [Oracle Mesh] Configuração mutada detectada. Derrubando túnel nativo para Hot-Reload.");
                    let _ = child.kill().await;
                }
                if let Some(port) = active_port.take() {
                    ACTIVE_MESH_TUNNELS.lock().await.remove(&port);
                }
                last_config = config.clone();
            }

            // Se o Oracle Target foi desligado na UI, apenas dormimos
            if !config.is_ready() {
                if current_child.is_some() {
                    let mut child = current_child.take().unwrap();
                    let _ = child.kill().await;
                    if let Some(port) = active_port.take() {
                        ACTIVE_MESH_TUNNELS.lock().await.remove(&port);
                    }
                    info!("☁️ [Oracle Mesh] Nó desabilitado. Túnel encerrado.");
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                continue;
            }

            let tunnel_port = config.ollama_tunnel_port;
            let ssh_target = config.ssh_target();
            let key_path = config.resolve_key_path();

            // Se não existe processo pingando, constrói e spuwna um localmente
            if current_child.is_none() {
                info!("☁️ [Oracle Mesh] Connection activated → {} (Port Fwd localhost:{} → 11434)", ssh_target, tunnel_port);
                let forward_str = format!("{}:127.0.0.1:11434", tunnel_port);
                let mut ssh_cmd = tokio::process::Command::new("ssh");

                ssh_cmd.arg("-N")
                       .arg("-o").arg("StrictHostKeyChecking=accept-new")
                       .arg("-o").arg("ServerAliveInterval=30")
                       .arg("-o").arg("ServerAliveCountMax=3")
                       .arg("-o").arg("ExitOnForwardFailure=yes")
                       .arg("-L").arg(&forward_str)
                       .arg("-i").arg(&key_path)
                       .arg(&ssh_target)
                       .stdin(std::process::Stdio::null())
                       .stdout(std::process::Stdio::null())
                       .stderr(std::process::Stdio::piped())
                       .kill_on_drop(true);

                match ssh_cmd.spawn() {
                    Ok(child) => {
                        info!("✅ [Oracle Mesh] Túnel Ollama Offload Subiu limpo na porta {}", tunnel_port);
                        current_child = Some(child);
                        active_port = Some(tunnel_port);
                        ACTIVE_MESH_TUNNELS.lock().await.insert(tunnel_port, (ssh_target.clone(), key_path.clone()));

                        // GAP-OR-02: Dispara o Provisioner Auto-Sync (Master -> Replica SHA-256) em background
                        let sync_config = config.clone();
                        tokio::spawn(async move {
                            if let Err(e) = crate::oracle_worker::provision_oracle_workers(&sync_config).await {
                                warn!("❌ [Oracle Mesh] Provisioner Auto-Sync falhou: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        warn!("❌ [Oracle Mesh] Falha drástica no OS SSH spawn: {}. Retry em 30s.", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                        continue;
                    }
                }
            }

            // Watchdog Loop — Wait for process death or timeout
            if let Some(mut child) = current_child.take() {
                tokio::select! {
                    // Timeout seguro de checagem. Se atingiu 5s sem quebrar, processo tá vivo.
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                        current_child = Some(child); // devolve pra Option e segue loop vitalino
                    }
                    status = child.wait() => {
                        error!("❌ [Oracle Mesh] Ocorreu Colapso Físico no Tunnel SSH de Offload (Exit {:?}). Retry engatilhado em 30s.", status);
                        if let Some(port) = active_port.take() {
                            ACTIVE_MESH_TUNNELS.lock().await.remove(&port);
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    }
                }
            }
        }
    });
}
