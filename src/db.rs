use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::env;
use tracing::info;

pub async fn init_pool() -> SqlitePool {
    // Escaneia a variável de ambiente ou injeta a raiz Cíbrida Master (Hardcoded fallback p/ o projeto)
    let db_path = env::var("DATABASE_URL").unwrap_or_else(|_| {
        let mut path = env::current_dir().expect("Sovereign: Current Dir Not Found");
        // Sobe um nível se estiver dentro de 'core'
        if path.ends_with("core") {
            path.pop();
        }
        path.push("data");
        path.push("sovereign_memory.db");
        
        let path_str = path.to_string_lossy().to_string();
        format!("sqlite:{}", path_str)
    });

    info!("🗄️ [Sovereign Core] Acoplando Banco Híbrido Cíbrido: {}", db_path);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_path)
        .await
        .expect("Sovereign Error: Falha crassa ao abrir a gaveta de memória SQLite");

    // Ativa PRAGMA WAL para velocidade Extrema igual ao Node Python antigo.
    let _ = sqlx::query("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;")
        .execute(&pool)
        .await;

    // Garante que a Engine Settings (Key Value) Exista
    let _ = sqlx::query("
        CREATE TABLE IF NOT EXISTS global_settings (
            id TEXT PRIMARY KEY,
            value_json TEXT NOT NULL
        );
    ").execute(&pool).await;

    // Garante a existência da Multi-Drive Tabela (Fase 32: Global Workspace Architecture)
    let _ = sqlx::query("
        CREATE TABLE IF NOT EXISTS workspaces (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
    ").execute(&pool).await;

    // Se o banco estiver vazio, engolimos a V1 Retrocompatível como 'Workspace 1' atómica
    let path_str = env::var("RAG_VAULT_PATH").unwrap_or_else(|_| {
        let mut path = env::current_dir().expect("Hostile Environment");
        if path.ends_with("core") { path.pop(); }
        path.push("Vault");
        path.to_string_lossy().into_owned()
    });

    let _ = sqlx::query("
        INSERT INTO workspaces (id, name, path)
        SELECT 1, 'Default Vault', ?
        WHERE NOT EXISTS (SELECT 1 FROM workspaces WHERE id = 1)
    ").bind(&path_str).execute(&pool).await;

    pool
}
