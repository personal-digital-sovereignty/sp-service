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

    // CARREGAMENTO NATIVO DO SCHEMA MESTRE CIBRIDO (EPIC 4)
    let _ = sqlx::query(include_str!("schemas/001_sensus_init.sql")).execute(&pool).await;

    // CARREGAMENTO DO EPHEMERAL RAG SCHEMA (MÓDULO DE NOTÍCIAS)
    let _ = sqlx::query(include_str!("schemas/002_ephemeral_knowledge.sql")).execute(&pool).await;

    // PATCH AUTOMIGRATION (MATRIX CAPABILITIES): Injela as novas colunas silenciosamente sem destruir DBs antigos
    let new_cols = vec!["is_master", "is_scribe", "is_auditor", "is_agent", "is_coder", "is_chat", "is_project"];
    for col in new_cols {
        let qs = format!("ALTER TABLE model_capabilities ADD COLUMN {} BOOLEAN DEFAULT 0", col);
        let _ = sqlx::query(&qs).execute(&pool).await; // Ignora o erro se a coluna já existir
    }
    
    // PATCH AUTOMIGRATION (OFFLINE SYNC FALLBACK)
    let _ = sqlx::query("ALTER TABLE model_capabilities ADD COLUMN is_installed BOOLEAN DEFAULT 1").execute(&pool).await;

    // PATCH 1.2.0 (MULTI-TENANCY): Resgata históricos antigos sem ID e os prende ao Workspace Primário
    let _ = sqlx::query("UPDATE chat_sessions SET workspace_id = '1' WHERE workspace_id IS NULL OR workspace_id = '' OR workspace_id = 'default'")
        .execute(&pool)
        .await;

    // Seed Initial Trusted Sources (Ignora caso o domínio já exista)
    let initial_tier_1_sources = vec![
        "istoedinheiro.com.br", "infomoney.com.br", "valorinveste.globo.com", "bloomberg.com", 
        "reuters.com", "exame.com"
    ];
    let initial_tier_2_sources = vec![
        "g1.globo.com", "cnnbrasil.com.br", "bbc.com", "folha.uol.com.br", "estadao.com.br"
    ];

    for source in initial_tier_1_sources {
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query("INSERT OR IGNORE INTO trusted_sources (id, domain, tier, category) VALUES (?, ?, 1, 'jornalismo_financeiro')")
            .bind(&uuid_str)
            .bind(source)
            .execute(&pool)
            .await;
    }

    for source in initial_tier_2_sources {
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query("INSERT OR IGNORE INTO trusted_sources (id, domain, tier, category) VALUES (?, ?, 2, 'jornalismo_geral')")
            .bind(&uuid_str)
            .bind(source)
            .execute(&pool)
            .await;
    }

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
