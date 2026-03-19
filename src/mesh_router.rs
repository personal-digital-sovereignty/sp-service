use reqwest::Client;
use tracing::{info, debug};
use crate::api_mesh::HardwareProfile;

/// O Mestre Direcionador da Malha (O Cérebro Dourado)
/// Inspeciona passivamente as portas Criptografadas ativas locais para buscar 'Capabilities' antes de despachar cargas.
pub struct MeshRouter;

impl MeshRouter {
    /// Varre a malha atrás de uma câmara de detonação limpa e purgada para rodar Scripts Python/Bash invasivos (The Coder).
    pub async fn find_best_coder_node(client: &Client) -> Option<(String, String)> {
        let tunnels = crate::ssh_mesh_connector::ACTIVE_MESH_TUNNELS.lock().await;
        
        for (port, (uri, key)) in tunnels.iter() {
            let handshake_url = format!("http://127.0.0.1:{}/v1/mesh/handshake", port);
            
            // Timeout curto pra não travar a esteira do Cíbrido
            if let Ok(res) = client.get(&handshake_url).timeout(std::time::Duration::from_millis(800)).send().await {
                if let Ok(profile) = res.json::<HardwareProfile>().await {
                    debug!("Mesh Node Profile @ {}: {:?}", uri, profile);
                    if profile.is_sandbox_isolated && profile.accepts_agent_delegation {
                        info!("🚀 [Mesh Router] Achou Oásis Sandbox! O Coder irá despachar carga para: {}", uri);
                        return Some((uri.clone(), key.clone()));
                    }
                }
            }
        }
        info!("⚠️ [Mesh Router] Nenhum Sandbox Zero-Trust disponível na malha ativa. Fallback ativado.");
        None
    }

    /// Varre a malha atrás do melhor Colosso de GPU para rodar inferências complexas de Raciocínio (The Doctor).
    pub async fn find_best_doctor_node(client: &Client) -> Option<u16> {
        let tunnels = crate::ssh_mesh_connector::ACTIVE_MESH_TUNNELS.lock().await;
        
        let mut best_port = None;
        let mut max_ram = 0;

        for (port, (_uri, _key)) in tunnels.iter() {
            let handshake_url = format!("http://127.0.0.1:{}/v1/mesh/handshake", port);
            
            if let Ok(res) = client.get(&handshake_url).timeout(std::time::Duration::from_millis(800)).send().await {
                if let Ok(profile) = res.json::<HardwareProfile>().await {
                    if (profile.has_gpu || profile.has_npu) && profile.accepts_agent_delegation {
                        // Prioriza o nó com mais RAM gráfica / de sistema disponível se houver conflito de GPUs
                        if profile.available_ram_mb > max_ram {
                            max_ram = profile.available_ram_mb;
                            best_port = Some(*port);
                        }
                    }
                }
            }
        }
        
        if let Some(port) = best_port {
            info!("🧠 [Mesh Router] O Mestre Visualizador despachará o LLM Proxying para o cluster mágico na porta remota mapeada local: {}", port);
        }
        best_port
    }
}
