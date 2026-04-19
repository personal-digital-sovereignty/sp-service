-- =====================================================================
-- SOVEREIGN TICKER REGISTRY  (Migration 004)
-- Auto-populado via brapi.dev no boot. Aprende dinamicamente via
-- yfinance quando resolve tickers desconhecidos (source='yfinance_dynamic').
-- Fonte de verdade: elimina o TICKER_MAP hardcoded em sovereign_matrix.py.
-- =====================================================================

CREATE TABLE IF NOT EXISTS ticker_registry (
    id               INTEGER  PRIMARY KEY AUTOINCREMENT,
    search_key       TEXT     NOT NULL UNIQUE,  -- UPPERCASE, espaço→'_', sem acento
    yf_symbol        TEXT     NOT NULL,          -- símbolo Yahoo Finance exato
    full_name        TEXT,                        -- nome descritivo legível
    sector           TEXT,                        -- sub-setor (ex: 'energia', 'saúde')
    market           TEXT     NOT NULL DEFAULT 'B3',
        -- 'B3' | 'NYSE' | 'NASDAQ' | 'FUTURES' | 'FX' | 'INDEX' | 'ETF'
        -- 'CRYPTO' | 'KOSDAQ' | 'TSE' | 'LSE' | 'OTC' | 'EPA' | 'OTHER'
    query_type_hint  TEXT     NOT NULL DEFAULT 'price',
        -- 'price'      → sempre fetch_financial_ticker
        -- 'dual'       → marca/brand conhecida fora do financeiro → tent. dual-tool
        -- 'news_first' → contexto web antes de preço
        -- 'sector_etf' → índice setorial — provavelmente série histórica
    is_active        INTEGER  NOT NULL DEFAULT 1,    -- 0=delisted
    last_verified_at DATETIME,                        -- última verificação yfinance
    source           TEXT     NOT NULL DEFAULT 'seed'
        -- 'seed' | 'brapi' | 'yfinance_dynamic' | 'manual'
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ticker_search  ON ticker_registry(search_key);
CREATE        INDEX IF NOT EXISTS idx_ticker_symbol  ON ticker_registry(yf_symbol);
CREATE        INDEX IF NOT EXISTS idx_ticker_market  ON ticker_registry(market);
CREATE        INDEX IF NOT EXISTS idx_ticker_active  ON ticker_registry(is_active);
