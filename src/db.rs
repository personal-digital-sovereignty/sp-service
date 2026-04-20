use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::env;
use tracing::info;

pub async fn init_pool() -> SqlitePool {
    // P2-03: Resolve DATABASE_URL ou constrói path padrão cross-platform sem PANICs
    let db_path = env::var("DATABASE_URL").unwrap_or_else(|_| {
        let base = dirs::data_local_dir().unwrap_or_else(|| {
            eprintln!("❌ [Sovereign Boot] data_local_dir() não resolveu. Usando diretório atual.");
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });
        let path = base.join("sovereign-pair").join("data");

        if !path.exists() {
            if let Err(e) = std::fs::create_dir_all(&path) {
                eprintln!("❌ [Sovereign Boot] Falha ao criar arvore de dados: {}. Verifique permissões.", e);
                std::process::exit(1);
            }
        }

        let path_str = path.join("sovereign_memory.db").to_string_lossy().to_string();
        format!("sqlite:{}?mode=rwc", path_str)
    });

    info!("🗄️ [Sovereign Core] Acoplando Banco Híbrido Cíbrido: {}", db_path);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_path)
        .await
        .unwrap_or_else(|e| {
            eprintln!("❌ [Sovereign Boot] Falha crítica ao abrir SQLite ({}): {}. Verifique permissões e DATABASE_URL.", db_path, e);
            std::process::exit(1);
        });

    // Ativa PRAGMA WAL para velocidade Extrema igual ao Node Python antigo e Foreign Keys.
    let _ = sqlx::query("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON;")
        .execute(&pool)
        .await;

    // CARREGAMENTO NATIVO DO SCHEMA MESTRE CIBRIDO (EPIC 4)
    // DC1 FIX: raw_sql suporta multi-statement explicitamente (CREATE TABLE + CREATE INDEX)
    let _ = sqlx::raw_sql(include_str!("schemas/001_sensus_init.sql")).execute(&pool).await;

    // CARREGAMENTO DO EPHEMERAL RAG SCHEMA (MÓDULO DE NOTÍCIAS)
    let _ = sqlx::raw_sql(include_str!("schemas/002_ephemeral_knowledge.sql")).execute(&pool).await;

    // CARREGAMENTO DO SOVEREIGN PROMPT VAULT SCHEMA
    let _ = sqlx::raw_sql(include_str!("schemas/003_sovereign_prompts.sql")).execute(&pool).await;

    // CARREGAMENTO DO SOVEREIGN TICKER REGISTRY (MIGRATION 004)
    let _ = sqlx::raw_sql(include_str!("schemas/004_ticker_registry.sql")).execute(&pool).await;

    // Seed core prompts do TOML (com verificação SHA-256)
    crate::prompt_vault::seed_core_prompts(&pool).await;

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
        let mut path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        path.push("Vault");
        path.to_string_lossy().into_owned()
    });

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces").fetch_one(&pool).await.unwrap_or(0);
    if count == 0 {
        let _ = sqlx::query("INSERT INTO workspaces (id, name, path) VALUES (1, 'Origin Vault', ?)").bind(&path_str).execute(&pool).await;
    }

    pool
}
