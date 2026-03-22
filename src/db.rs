use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::env;
use tracing::info;

pub async fn init_pool() -> SqlitePool {
    // Escaneia a variável de ambiente ou injeta a raiz Cíbrida Master (Hardcoded fallback p/ o projeto)
    // Escaneia a variável de ambiente ou usa a pasta nativa do Sistema Operacional (Evita crash Sidecar/AppImage)
    let db_path = env::var("DATABASE_URL").unwrap_or_else(|_| {
        let mut path = dirs::data_local_dir().expect("Sovereign: SO Data Local Dir Not Found");
        path.push("sovereign-pair");
        path.push("data");
        
        // Garante que a estrutura da pasta exista antes que o SQLite tente criar o arquivo
        if !path.exists() {
            std::fs::create_dir_all(&path).expect("Sovereign: Falha ao criar arvore de dados do O.S");
        }
        
        path.push("sovereign_memory.db");
        
        let path_str = path.to_string_lossy().to_string();
        // O mode=rwc obriga o libsqlite3 a criar o arquivo físico caso ele não exista na pasta
        format!("sqlite:{}?mode=rwc", path_str)
    });

    info!("🗄️ [Sovereign Core] Acoplando Banco Híbrido Cíbrido: {}", db_path);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_path)
        .await
        .expect("Sovereign Error: Falha crassa ao abrir a gaveta de memória SQLite");

    // Ativa PRAGMA WAL para velocidade Extrema igual ao Node Python antigo e Foreign Keys.
    let _ = sqlx::query("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON;")
        .execute(&pool)
        .await;

    // Garante que a Engine Settings (Key Value) Exista
    let _ = sqlx::query("
        CREATE TABLE IF NOT EXISTS global_settings (
            id TEXT PRIMARY KEY,
            value_json TEXT NOT NULL
        );
    ").execute(&pool).await;

    // Garante a existência da Multi-Drive Tabela
    let _ = sqlx::query("
        CREATE TABLE IF NOT EXISTS workspaces (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
    ").execute(&pool).await;

    // Nova Tabela Mestre de Documentos (Cíbrida Rust) mapeada ao Workspace
    let _ = sqlx::query("
        CREATE TABLE IF NOT EXISTS sensus_documents (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            file_path TEXT NOT NULL UNIQUE,
            content_raw TEXT,
            summary TEXT,
            last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS sovereign_chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid_reference TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            text_content TEXT NOT NULL,
            metadata_json TEXT
        );
        CREATE TABLE IF NOT EXISTS rlhf_feedback (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_role TEXT NOT NULL,
            content TEXT NOT NULL,
            thumbs_up BOOLEAN NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS chat_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            folder_name TEXT,
            tags_json TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            thumbs_up BOOLEAN DEFAULT 0,
            thumbs_down BOOLEAN DEFAULT 0,
            is_hidden BOOLEAN DEFAULT 0,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS security_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            severity TEXT NOT NULL,
            blocked BOOLEAN NOT NULL DEFAULT 1,
            message TEXT NOT NULL,
            source TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
                 "
    ).execute(&pool).await;

    // Garante Tabelas de Kanban (Projects e Tasks)
    let _ = sqlx::query("
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            name TEXT NOT NULL,
            purpose TEXT,
            traction_status TEXT,
            next_action TEXT,
            energy_level TEXT,
            progress_percent INTEGER DEFAULT 0,
            friction_radar TEXT,
            deadline TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            tenant_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT,
            priority TEXT,
            order_index INTEGER DEFAULT 0,
            deadline TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS project_documents (
            project_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            PRIMARY KEY(project_id, file_path),
            FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
    ").execute(&pool).await;

    // Migrations to support Custom Columns and Archiving on existing databases (fail silently if already exists)
    let _ = sqlx::query("ALTER TABLE projects ADD COLUMN is_archived BOOLEAN DEFAULT 0;").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE projects ADD COLUMN columns_json TEXT DEFAULT '[\"To Do\", \"In Progress\", \"Done\"]';").execute(&pool).await;

    let path_str = env::var("RAG_VAULT_PATH").unwrap_or_else(|_| {
        let mut path = env::current_dir().expect("Hostile Environment");
        if path.ends_with("core") { path.pop(); }
        path.push("Vault");
        path.to_string_lossy().into_owned()
    });

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces").fetch_one(&pool).await.unwrap_or(0);
    if count == 0 {
        let _ = sqlx::query("INSERT INTO workspaces (id, name, path) VALUES (1, 'Origin Vault', ?)").bind(&path_str).execute(&pool).await;
    }

    pool
}
