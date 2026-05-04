use axum::async_trait;
use anyhow::Result;

/// Contrato Base para a camada de Persistência Híbrida.
/// Esta abstração permitirá futuramente trocar o `SqlitePool` para 
/// bancos baseados em nuvem (Postgres) ou em memória sem afetar o core da API.
#[async_trait]
pub trait Repository: Send + Sync {
    /// Testa a conexão com o banco subjacente
    async fn ping(&self) -> Result<()>;
}
