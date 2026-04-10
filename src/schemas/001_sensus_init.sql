-- ============================================================================
-- SOVEREIGN PAIR - CIBRID MASTER SCHEMA LAYER
-- ============================================================================

-- ---------------------------------------------------------
-- 1. CONFIGURAÇÕES GLOBAIS E AMBIENTE
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS global_settings (
    id TEXT PRIMARY KEY,
    value_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workspaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ---------------------------------------------------------
-- 2. VAULT & INGESTÃO CÍBRIDA (RAG)
-- ---------------------------------------------------------
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

-- ---------------------------------------------------------
-- 3. INTERAÇÕES DE AGENTE & CHAT
-- ---------------------------------------------------------
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

CREATE TABLE IF NOT EXISTS rlhf_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_role TEXT NOT NULL,
    content TEXT NOT NULL,
    thumbs_up BOOLEAN NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ---------------------------------------------------------
-- 4. INTELIGÊNCIA & ROTEAMENTO SQS
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS routing_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    target_model TEXT NOT NULL,
    latency_badge TEXT NOT NULL,
    icon TEXT NOT NULL,
    is_active BOOLEAN DEFAULT 1,
    order_index INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS knowledge_gaps (
    id TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    frequency INTEGER DEFAULT 1,
    context TEXT NOT NULL,
    sentiment TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status TEXT DEFAULT 'pending',          -- Fallback nativo
    resolution_content TEXT                 -- Fallback nativo
);

CREATE TABLE IF NOT EXISTS evaluations (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    user_query TEXT NOT NULL,
    rag_context TEXT NOT NULL,
    ai_response TEXT NOT NULL,
    faithfulness_score INTEGER DEFAULT 0,
    precision_score INTEGER DEFAULT 0,
    status TEXT DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS research_staging (
    id TEXT PRIMARY KEY,
    directive TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ---------------------------------------------------------
-- 5. TELEMETRIA E HARDWARE LLM
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS remote_models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    icon_url TEXT,
    latency_ms INTEGER DEFAULT 0,
    cost_per_1k REAL DEFAULT 0.0,
    success_rate REAL DEFAULT 1.0,
    status TEXT DEFAULT 'Operational',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS model_capabilities (
    model_name TEXT PRIMARY KEY,
    parameter_size REAL NOT NULL,
    supports_tools BOOLEAN DEFAULT 0,
    is_reasoner BOOLEAN DEFAULT 0,
    template TEXT,
    last_checked TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS model_hallucinations (
    id TEXT PRIMARY KEY,
    model_name TEXT NOT NULL,
    lies_detected INTEGER DEFAULT 0,
    queries_processed INTEGER DEFAULT 0,
    last_lied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS model_metrics (
    model_name TEXT PRIMARY KEY,
    total_tokens INTEGER DEFAULT 0,
    total_duration_ms INTEGER DEFAULT 0,
    first_used_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ---------------------------------------------------------
-- 6. SEGURANÇA E AUTO-BLINDAGEM
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS security_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    blocked BOOLEAN NOT NULL DEFAULT 1,
    message TEXT NOT NULL,
    source TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS trusted_sources (
    id TEXT PRIMARY KEY,
    domain TEXT UNIQUE NOT NULL,
    tier INTEGER NOT NULL,
    category TEXT,
    is_active BOOLEAN DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS domain_extraction_ledger (
    id TEXT PRIMARY KEY,
    domain TEXT UNIQUE NOT NULL,
    technique_html_success BOOLEAN DEFAULT 0,
    technique_js_success BOOLEAN DEFAULT 0,
    technique_ghost_success BOOLEAN DEFAULT 0,
    last_search_prompt TEXT,
    quarantine_until DATETIME,
    last_attempted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    status TEXT DEFAULT 'verified'
);

-- ---------------------------------------------------------
-- 7. KANBAN MANAGER (PLAN & EXECUTE)
-- ---------------------------------------------------------
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
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_archived BOOLEAN DEFAULT 0,
    columns_json TEXT DEFAULT '["To Do", "In Progress", "Done"]'
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

-- ---------------------------------------------------------
-- 8. SOVEREIGN API GATEWAY (SANDBOX & O-DATA)
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS public_api_directory (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    description TEXT,
    auth TEXT,
    https TEXT,
    cors TEXT,
    category TEXT
);
