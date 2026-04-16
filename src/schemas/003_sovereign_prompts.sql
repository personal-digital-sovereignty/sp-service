-- Sovereign Prompt Vault (v1.2.4+)
-- Tabela para armazenamento dinâmico de prompts e regras do sistema.
-- Prompts core (SP-9XXX) são imutáveis via UI e protegidos por SHA-256.

CREATE TABLE IF NOT EXISTS sovereign_prompts (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL CHECK(category IN ('core','scribe','auditor','tool_schema','user')),
    title TEXT NOT NULL,
    prompt_text TEXT NOT NULL,
    placeholders TEXT DEFAULT '[]',
    is_core BOOLEAN DEFAULT 0,
    is_active BOOLEAN DEFAULT 1,
    version INTEGER DEFAULT 1,
    integrity_hash TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT DEFAULT 'system'
);

-- Índice para hot-reload rápido por slug
CREATE INDEX IF NOT EXISTS idx_prompts_slug ON sovereign_prompts(slug);
CREATE INDEX IF NOT EXISTS idx_prompts_active ON sovereign_prompts(is_active, is_core);
