#!/usr/bin/env python3
import sys
import json
import datetime
import urllib.request
import urllib.error

def normalize_date(raw):
    s = str(raw).strip()
    if not s: return raw
    try:
        if '/' in s:
            parts = s.split('/')
            if len(parts) == 3:
                return f"{parts[2]}-{parts[1]}"
        elif '-' in s:
            parts = s.split('-')
            if len(parts) >= 2:
                return f"{parts[0]}-{parts[1]}"
    except Exception:
        pass
    return s

def fetch_finance(ticker, years):
    # Cross-Router: Check standard Macro indicators
    if ticker.upper() in ["IPCA", "IGPM", "SELIC", "INPC"]:
        return fetch_macro(ticker.upper(), "BR", years)
        
    # Dynamically check Autobahn Proxies (Forgive financial mapping of local datasets)
    import os
    proxy_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dataset_proxies")
    if os.path.exists(proxy_dir):
        for f_name in os.listdir(proxy_dir):
            if f_name.endswith(".json"):
                base_name = f_name[:-5]
                if base_name.lower() in ticker.lower() or ticker.lower() in base_name.lower():
                    return fetch_macro(base_name.upper(), "BR", years)
        
    try:
        import yfinance as yf
        import pandas as pd
    except ImportError:
        import subprocess
        import sys
        try:
            subprocess.check_call([sys.executable, "-m", "pip", "install", "-q", "yfinance", "pandas"])
            import yfinance as yf
            import pandas as pd
        except Exception as e:
            print(json.dumps({"error": f"Packages 'yfinance' and 'pandas' are missing. Auto-healing failed: {str(e)}"}))
            sys.exit(1)
        
    clean_years = years.replace('y', '').replace('Y', '')
    if not clean_years.isdigit():
        clean_years = "1"
        
    period = f"{clean_years}y"
    
    # ═══════════════════════════════════════════════════════════════════════════
    # SOVEREIGN TICKER RESOLVER — 4-Pass SQLite + Auto-Learning
    # ═══════════════════════════════════════════════════════════════════════════
    # Fluxo de resolução (sem hardcode):
    #   [1] Exact match  : WHERE search_key = normalizado          → O(log n)
    #   [2] Prefix match : WHERE search_key LIKE 'MAGAZINE%'       → O(log n)
    #   [3] Fuzzy match  : WHERE search_key LIKE '%LUIZA%'          → O(n)
    #   [4] yfinance live: testa ".SA" e ticker puro              → rede
    #       → HIT: INSERT INTO ticker_registry (auto-aprendizado)
    #       → MISS: erro descritivo ao LLM
    #
    # TICKER_MAP_FALLBACK abaixo é usado APENAS quando o banco não é encontrado
    # (ex: primeiro boot antes da migration). Mantido como emergência offline.
    # ═══════════════════════════════════════════════════════════════════════════

    import sqlite3 as _sqlite3
    import unicodedata as _uni

    def _normalize_key(name: str) -> str:
        nfkd = _uni.normalize("NFKD", name)
        ascii_ = "".join(c for c in nfkd if not _uni.combining(c))
        return ascii_.upper().replace(" ", "_").replace("-", "_").replace(".", "_")

    def _find_db() -> str | None:
        """Localiza sovereign_memory.db: env var > XDG > macOS > Windows > Linux > busca ascendente."""
        import os as _os
        # 1. DATABASE_URL env var (prod / containers)
        db_url = _os.getenv("DATABASE_URL", "")
        if db_url:
            candidate = db_url.replace("sqlite:", "").split("?")[0]
            if _os.path.exists(candidate):
                return candidate
        # 2. XDG_DATA_HOME (Linux custom)
        xdg = _os.getenv("XDG_DATA_HOME", "")
        if xdg:
            candidate = _os.path.join(xdg, "sovereign-pair", "data", "sovereign_memory.db")
            if _os.path.exists(candidate):
                return candidate
        # 3. macOS ~/Library/Application Support
        import sys as _sys
        if _sys.platform == "darwin":
            candidate = _os.path.join(_os.path.expanduser("~"), "Library", "Application Support",
                                      "sovereign-pair", "data", "sovereign_memory.db")
            if _os.path.exists(candidate):
                return candidate
        # 4. Windows %LOCALAPPDATA%
        local_app_data = _os.getenv("LOCALAPPDATA", "")
        if local_app_data:
            candidate = _os.path.join(local_app_data, "sovereign-pair", "data", "sovereign_memory.db")
            if _os.path.exists(candidate):
                return candidate
        # 5. Linux ~/.local/share (XDG default)
        candidate = _os.path.join(_os.path.expanduser("~"), ".local", "share",
                                  "sovereign-pair", "data", "sovereign_memory.db")
        if _os.path.exists(candidate):
            return candidate
        # 6. Busca ascendente a partir do script (desenvolvimento local)
        cur = _os.path.dirname(_os.path.abspath(__file__))
        for _ in range(6):
            candidate = _os.path.join(cur, "sovereign_memory.db")
            if _os.path.exists(candidate):
                return candidate
            candidate2 = _os.path.join(cur, "data", "sovereign_memory.db")
            if _os.path.exists(candidate2):
                return candidate2
            parent = _os.path.dirname(cur)
            if parent == cur:
                break
            cur = parent
        return None

    def _resolve_from_db(db_path: str, norm_key: str, raw_ticker: str):
        """Retorna (yf_symbol, full_name) ou None. DM5 FIX: try/finally garante conn.close()."""
        try:
            conn = _sqlite3.connect(db_path, timeout=3)
            try:
                conn.row_factory = _sqlite3.Row
                c = conn.cursor()
                # Passe 1 — Exact match
                c.execute(
                    "SELECT yf_symbol, full_name FROM ticker_registry "
                    "WHERE search_key = ? AND is_active = 1 LIMIT 1",
                    (norm_key,),
                )
                row = c.fetchone()
                if row:
                    return row["yf_symbol"], row["full_name"]
                # Passe 2 — Prefix match
                c.execute(
                    "SELECT yf_symbol, full_name FROM ticker_registry "
                    "WHERE search_key LIKE ? AND is_active = 1 ORDER BY length(search_key) LIMIT 1",
                    (f"{norm_key}%",),
                )
                row = c.fetchone()
                if row:
                    return row["yf_symbol"], row["full_name"]
                # Passe 3 — Fuzzy match (parte do nome)
                parts = norm_key.split("_")
                for part in parts:
                    if len(part) < 3:
                        continue
                    c.execute(
                        "SELECT yf_symbol, full_name FROM ticker_registry "
                        "WHERE search_key LIKE ? AND is_active = 1 "
                        "ORDER BY length(search_key) LIMIT 1",
                        (f"%{part}%",),
                    )
                    row = c.fetchone()
                    if row:
                        return row["yf_symbol"], row["full_name"]
                return None
            finally:
                conn.close()
        except Exception:
            return None

    def _auto_learn(db_path: str, norm_key: str, yf_sym: str, full_name: str) -> None:
        """Persiste ticker descoberto dinamicamente (auto-aprendizado). DM2 FIX: popula last_verified_at."""
        try:
            conn = _sqlite3.connect(db_path, timeout=3)
            try:
                conn.execute(
                    """INSERT OR IGNORE INTO ticker_registry
                       (search_key, yf_symbol, full_name, market, query_type_hint, is_active, source, last_verified_at)
                       VALUES (?, ?, ?, 'OTHER', 'price', 1, 'yfinance_dynamic', datetime('now'))""",
                    (norm_key, yf_sym, full_name),
                )
                conn.commit()
            finally:
                conn.close()
        except Exception:
            pass

    # ── TICKER_MAP_FALLBACK (emergência offline — banco não encontrado) ───────
    TICKER_MAP_FALLBACK = {
        'BRENT': ('BZ=F', 'Petróleo Brent'), 'WTI': ('CL=F', 'Petróleo WTI'),
        'GOLD': ('GC=F', 'Ouro'), 'SILVER': ('SI=F', 'Prata'),
        'DOLAR': ('BRL=X', 'Dólar/BRL'), 'USD': ('BRL=X', 'Dólar/BRL'),
        'EURO': ('EURBRL=X', 'Euro/BRL'),
        'PETROBRAS': ('PETR4.SA', 'Petrobras'), 'PETR4': ('PETR4.SA', 'Petrobras'),
        'NUBANK': ('NU', 'NuBank'), 'NU': ('NU', 'NuBank'),
        'VALE': ('VALE3.SA', 'Vale'), 'VALE3': ('VALE3.SA', 'Vale'),
        'ITAU': ('ITUB4.SA', 'Itaú'), 'BRADESCO': ('BBDC4.SA', 'Bradesco'),
        'BANCO_DO_BRASIL': ('BBAS3.SA', 'Banco do Brasil'), 'BB': ('BBAS3.SA', 'BB'),
        'AMBEV': ('ABEV3.SA', 'Ambev'),
        'MAGAZINE': ('MGLU3.SA', 'Magazine Luiza'), 'MAGALU': ('MGLU3.SA', 'Magazine Luiza'),
        'MGLU3': ('MGLU3.SA', 'Magazine Luiza'), 'MGLU': ('MGLU3.SA', 'Magazine Luiza'),
        'MAGAZINE_LUIZA': ('MGLU3.SA', 'Magazine Luiza'),
        'WEG': ('WEGE3.SA', 'WEG'), 'SUZANO': ('SUZB3.SA', 'Suzano'),
        'JBS': ('JBSS3.SA', 'JBS'), 'ELETROBRAS': ('ELET3.SA', 'Eletrobras'),
        'LOCALIZA': ('RENT3.SA', 'Localiza'), 'HAPVIDA': ('HAPV3.SA', 'Hapvida'),
        'SANTANDER': ('SANB11.SA', 'Santander BR'),
        'NVIDIA': ('NVDA', 'NVIDIA'), 'NVDA': ('NVDA', 'NVIDIA'),
        'APPLE': ('AAPL', 'Apple'), 'MICROSOFT': ('MSFT', 'Microsoft'),
        'TESLA': ('TSLA', 'Tesla'), 'NIKE': ('NKE', 'Nike'),
        'NOVO_NORDISK': ('NVO', 'Novo Nordisk'), 'OZEMPIC': ('NVO', 'Ozempic→NVO'),
        'ELI_LILLY': ('LLY', 'Eli Lilly'), 'MOUNJARO': ('LLY', 'Mounjaro→LLY'),
    }

    # ── Resolução principal ──────────────────────────────────────────────────
    semantic_name = ticker
    norm_key = _normalize_key(ticker)
    resolved = False

    db_path = _find_db()
    if db_path:
        result = _resolve_from_db(db_path, norm_key, ticker)
        if result:
            ticker, semantic_name = result
            resolved = True

    if not resolved and norm_key in TICKER_MAP_FALLBACK:
        ticker, semantic_name = TICKER_MAP_FALLBACK[norm_key]
        resolved = True

    # Passe 4: yfinance live + auto-aprendizado
    if not resolved:
        import sys as _sys
        _candidates = []
        if not ticker.endswith('.SA'):
            _candidates.append(f"{ticker.upper()}.SA")
        _candidates.append(ticker.upper())

        for _cand in _candidates:
            try:
                _t = yf.Ticker(_cand)
                _test = _t.history(period="5d")
                if not _test.empty:
                    info = _t.info or {}
                    _full = info.get("longName") or info.get("shortName") or f"Ativo ({_cand})"
                    semantic_name = _full
                    ticker = _cand
                    resolved = True
                    if db_path:
                        _auto_learn(db_path, norm_key, _cand, _full)
                    break
            except Exception:
                continue

        if not resolved:
            print(json.dumps({
                "error": (
                    f"Ticker '{ticker}' não reconhecido pelo Sovereign Ticker Registry "
                    f"(4 passes: exact/prefix/fuzzy/yfinance). "
                    f"Tickers testados: {_candidates}. "
                    f"Verifique o símbolo exato (ex: PETR4, NU, VALE3, MGLU3, BRENT, ITUB4) "
                    f"ou use o nome da empresa — o resolvedor encontrará automaticamente."
                )
            }))
            _sys.exit(1)

    start_date = (datetime.datetime.now() - datetime.timedelta(days=int(clean_years)*365)).strftime('%Y-%m-%d')
    end_date = datetime.datetime.now().strftime('%Y-%m-%d')
    
    # === LAYER 1: YFINANCE (Official Library) ===
    df = pd.DataFrame()
    last_error = ""
    source_used = "Yahoo Finance Open-Data"
    try:
        t = yf.Ticker(ticker)
        df = t.history(period=period)
    except Exception as e:
        last_error = f"Layer 1 (yfinance) failed: {e}"
        
    # === LAYER 2: YAHOO RAW API (Browser Spoofing) ===
    if df.empty:
        try:
            import urllib.request
            import time
            start_ts = int(time.mktime(time.strptime(start_date, '%Y-%m-%d')))
            end_ts = int(time.mktime(time.strptime(end_date, '%Y-%m-%d')))
            url = f"https://query1.finance.yahoo.com/v8/finance/chart/{ticker}?period1={start_ts}&period2={end_ts}&interval=1d"
            req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36'})
            with urllib.request.urlopen(req, timeout=10) as response:  # nosemgrep
                raw_json = json.loads(response.read().decode())
                timestamps = raw_json['chart']['result'][0]['timestamp']
                closes = raw_json['chart']['result'][0]['indicators']['quote'][0]['close']
                dates = [datetime.datetime.fromtimestamp(ts) for ts in timestamps]
                df = pd.DataFrame({'Close': closes}, index=dates)
                source_used = "Yahoo Finance Raw API (Browser Spoofed)"
        except Exception as e:
            last_error = f"Layer 2 (Yahoo Raw) failed: {e}"

    # === LAYER 3: BRAPI.DEV (Para ativos brasileiros) ===
    if df.empty and ticker.endswith('.SA'):
        try:
            br_ticker = ticker.replace('.SA', '')
            url = f"https://brapi.dev/api/quote/{br_ticker}?range={period}&interval=1d"
            req = urllib.request.Request(url, headers={'User-Agent': 'Sovereign-Worker/1.0'})
            with urllib.request.urlopen(req, timeout=10) as response:  # nosemgrep
                raw_json = json.loads(response.read().decode())
                results = raw_json['results'][0]['historicalDataPrice']
                dates = [datetime.datetime.fromtimestamp(x['date']) for x in results]
                closes = [x['close'] for x in results]
                df = pd.DataFrame({'Close': closes}, index=dates)
                source_used = "Brapi.dev Open API (BR Native)"
        except Exception as e:
            last_error = f"Layer 3 (Brapi) failed: {e}"

    # === LAYER 4: STOOQ (CSV Endpoint / Global) ===
    if df.empty:
        try:
            stooq_ticker = ticker.replace('.SA', '.BR') if '.SA' in ticker else ticker
            url = f"https://stooq.com/q/d/l/?s={stooq_ticker}&d1={start_date.replace('-','')}&d2={end_date.replace('-','')}&i=d"
            df = pd.read_csv(url, index_col='Date', parse_dates=True)
            if 'Close' not in df.columns:
                df = pd.DataFrame() # CSV is missing data/blocked
            else:
                source_used = "Stooq CSV Database"
        except Exception as e:
            last_error = f"Layer 4 (Stooq) failed: {e}"

    if df.empty:
        print(json.dumps({"error": f"No financial data found for {ticker} across all 4 Multi-Node layers! Last error: {last_error}"}))
        sys.exit(1)
        
    # ══════════════════════════════════════════════════════════════════════════
    # DC3 FIX: Circuit Breaker roda ANTES de qualquer agrupamento/conversão,
    # sobre dados brutos diários — detecta spikes corrompidos sem suavização.
    # ══════════════════════════════════════════════════════════════════════════
    SANITY_BOUNDS: dict = {
        'BZ=F': (5.0, 250.0), 'CL=F': (5.0, 250.0),  # Petróleo (USD/barril)
        'GC=F': (500.0, 5000.0), 'SI=F': (5.0, 200.0),  # Ouro/Prata
        'PL=F': (200.0, 3000.0), 'PA=F': (100.0, 4000.0),
        'HG=F': (1.0, 20.0),   # Cobre (USD/lb)
        'NG=F': (0.5, 25.0),   # Gás Natural (USD/MMBtu)
        'ZS=F': (4.0, 30.0),   # Soja (USD/bushel)
        'ZC=F': (2.0, 10.0),   # Milho
        'ZW=F': (2.0, 15.0),   # Trigo
        'KC=F': (0.5, 6.0),    # Café Arábica (USD/lb)
        'SB=F': (0.05, 1.0),   # Açúcar (USD/lb)
        'CT=F': (0.3, 3.0),    # Algodão (USD/lb)
    }
    bounds = SANITY_BOUNDS.get(ticker)
    if bounds and not df.empty and 'Close' in df.columns:
        max_v = float(df['Close'].max())
        min_v = float(df['Close'].min())
        if not (bounds[0] <= min_v and max_v <= bounds[1]):
            print(json.dumps({"error": f"CRÍTICO (Circuit Breaker): Anomalia em {ticker}. "
                              f"Valores USD (Max: {round(max_v, 2)}, Min: {round(min_v, 2)}) "
                              f"fora dos limites físicos {bounds}. Abortando para prevenir "
                              f"alucinação estatística."}))
            sys.exit(1)

    # ══════════════════════════════════════════════════════════════════════════
    # DC4 FIX: Conversão BRL para TODOS os períodos (não apenas years>1).
    # Agrupa sempre por mês para alinhamento do join USD→BRL.
    # DC4b FIX: left join preserva meses sem câmbio (exibidos só em USD).
    # DL2 FIX: log explícito quando conversão BRL falha.
    # ══════════════════════════════════════════════════════════════════════════
    CONVERT_TO_BRL = {
        'BZ=F', 'CL=F',           # Petróleo Brent e WTI
        'GC=F', 'SI=F',           # Ouro e Prata
        'PL=F', 'PA=F',           # Platina e Paládio
        'HG=F',                   # Cobre
        'NG=F',                   # Gás Natural
        'ZS=F', 'ZC=F', 'ZW=F',  # Soja, Milho, Trigo (CBOT)
        'ZM=F', 'ZL=F',           # Farelo e Óleo de Soja
        'KC=F', 'RC=F',           # Café Arábica e Robusta
        'SB=F', 'CT=F',           # Açúcar e Algodão
        'CC=F', 'OJ=F',           # Cacau e Suco de Laranja
        'LE=F', 'HE=F',           # Boi Gordo e Suíno
        'LB=F',                   # Madeira Serrada
    }
    converted_to_brl = False
    if ticker in CONVERT_TO_BRL:
        # Fetch BRL=X to do Currency Conversion
        df_usd = pd.DataFrame()
        try:
            t_usd = yf.Ticker('BRL=X')
            df_usd = t_usd.history(period=period)
        except Exception:
            pass
            
        if df_usd.empty:
            try:
                import time
                start_ts = int(time.mktime(time.strptime(start_date, '%Y-%m-%d')))
                end_ts = int(time.mktime(time.strptime(end_date, '%Y-%m-%d')))
                url = f"https://query1.finance.yahoo.com/v8/finance/chart/BRL=X?period1={start_ts}&period2={end_ts}&interval=1d"
                req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36'})
                with urllib.request.urlopen(req, timeout=10) as response:  # nosemgrep
                    raw_json = json.loads(response.read().decode())
                    timestamps = raw_json['chart']['result'][0]['timestamp']
                    closes = raw_json['chart']['result'][0]['indicators']['quote'][0]['close']
                    dates = [datetime.datetime.fromtimestamp(ts) for ts in timestamps]
                    df_usd = pd.DataFrame({'Close': closes}, index=dates)
            except Exception:
                pass
                
        if not df_usd.empty:
            try:
                # Agrupa por mês para alinhamento (necessário para todos os períodos)
                df['YearMonth'] = df.index.strftime('%Y-%m')
                df = df.groupby('YearMonth').mean()
                
                df_usd['YearMonth'] = df_usd.index.strftime('%Y-%m')
                df_usd = df_usd.groupby('YearMonth').mean()
                
                # DC4b FIX: left join preserva meses do ativo sem câmbio disponível
                df = df.join(df_usd['Close'], rsuffix='_usd', how='left')
                df['Close_brl'] = df['Close'] * df['Close_usd']
                converted_to_brl = True
                source_used += " | (+ Converted to BRL)"
            except Exception:
                # DL2 FIX: log explícito de falha na conversão
                source_used += " | (⚠️ BRL conversion JOIN FAILED — showing USD only)"
        else:
            # DL2 FIX: BRL=X indisponível (rate limit ou API down)
            source_used += " | (⚠️ BRL=X unavailable — showing USD only)"
                
    if not converted_to_brl:
        # Agrupamento mensal para períodos longos (sem conversão BRL)
        try:
            if int(clean_years) > 1:
                df['YearMonth'] = df.index.strftime('%Y-%m')
                df = df.groupby('YearMonth').mean()
        except Exception:
            pass


    data_lines = []
    for index, row in df.iterrows():
        date_str = index if isinstance(index, str) else index.strftime('%Y-%m')
        # Lida com casos onde row['Close'] seja NaN no Pandas
        if pd.isna(row.get('Close', float('nan'))):
             continue
        val = round(float(row['Close']), 2)
        if converted_to_brl and 'Close_brl' in row and not pd.isna(row['Close_brl']):
            val_brl = round(float(row['Close_brl']), 2)
            data_lines.append(f"{date_str} | USD {val} | BRL {val_brl}")
        else:
            data_lines.append(f"{date_str} | {val}")
        
    brl_warning = " - ATENÇÃO: VALORES CÂMBIO DUPLO EXPOSTO NO RAW (USD/BRL)]" if converted_to_brl else "]"
    ctx_header = f"[CONTEXT: DADOS HISTÓRICOS BRUTOS REFERENTES AO ATIVO {ticker.upper()} ({semantic_name}){brl_warning}\n"
    data_compressed = ctx_header + "\n".join(data_lines)
        
    print(json.dumps({
        "status": "success",
        "source": source_used,
        "ticker": ticker, 
        "semantic_name": semantic_name,
        "period": period, 
        "data_compressed": data_compressed
    }))

def fetch_macro(indicator, country, years):
    if country.upper() != 'BR':
        print(json.dumps({"error": "Currently only 'BR' macroeconomic indicators are supported natively."}))
        sys.exit(1)
        
    # === AUTOBAHN ROUTER (Dynamic Proxy Resolution) ===
    import os
    proxy_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dataset_proxies")
    
    # Autobahn Wildcard Resolver: If the LLM uses a descriptive phrase (e.g. "PRECO DA GASOLINA"),
    # we scan the proxies folder to see if any proxy filename (e.g. "gasolina") is a substring.
    if os.path.exists(proxy_dir):
        for f_name in os.listdir(proxy_dir):
            if f_name.endswith(".json"):
                base_name = f_name[:-5].lower()
                if base_name in indicator.lower() or indicator.lower() in base_name:
                    indicator = base_name.upper()
                    break

    proxy_file = os.path.join(proxy_dir, f"{indicator.lower()}.json")
    
    if os.path.exists(proxy_file):
        try:
            with open(proxy_file, "r", encoding="utf-8") as f:
                proxy_data = json.load(f)
            
            data_arr = proxy_data.get("data", [])
            data_lines = []
            if len(data_arr) > 0:
                try:
                    import pandas as pd
                    df = pd.DataFrame(data_arr)
                    # Support multiple date keys
                    if 'date' in df.columns:
                        df['date_col'] = pd.to_datetime(df['date'], errors='coerce')
                    else:
                        df['date_col'] = pd.to_datetime(df.index, errors='coerce')
                        
                    val_col = 'value' if 'value' in df.columns else 'close' if 'close' in df.columns else None
                    if val_col and df['date_col'].notna().any():
                        df = df.set_index('date_col').resample('ME').last().ffill()
                        
                        for idx, row in df.iterrows():
                            val = row[val_col]
                            if pd.notna(val):
                                data_lines.append(f"{idx.strftime('%Y-%m')} | {round(float(val),2)}")
                except Exception:  # noqa: F841
                    # Fallback to crude extraction if Pandas fails
                    for item in data_arr:
                        date_str = normalize_date(item.get("date", ""))
                        val = item.get("value", item.get("close", ""))
                        data_lines.append(f"{date_str} | {val}")
                
            ctx_header = f"[CONTEXT: DADOS HISTÓRICOS BRUTOS REFERENTES AO MACRO INDICADOR {indicator.upper()}]\n"
            data_compressed = ctx_header + "\n".join(data_lines)
            
            # Enrich Autobahn Proxy with required macro payload schema
            print(json.dumps({
                "status": "success",
                "source": proxy_data.get("source", "Sovereign Autobahn Proxy Vault"),
                "indicator": indicator.upper(),
                "country": country.upper(),
                "period": f"{years}y",
                "data_compressed": data_compressed
            }))
            sys.exit(0)
        except Exception as e:
            print(json.dumps({"error": f"Autobahn Proxy Error resolving '{indicator}': {str(e)}"}))
            sys.exit(1)

    # Fallback Chain: cada indicador tem uma série primária e alternativas.
    # Se a primária falhar (404/erro), a próxima é tentada automaticamente.
    # Formato: [(sgs_code, label), ...]
    FALLBACK_CHAINS = {
        "IPCA":           [(433, "IPCA Mensal")],
        "SELIC":          [(432, "SELIC Meta"), (4189, "SELIC Diária")],
        "IGPM":           [(189, "IGP-M")],
        "INPC":           [(188, "INPC")],
        "DOLAR_PTAX":     [(10813, "PTAX Venda Média"), (1, "Dólar Comercial Compra"), (3698, "Dólar Livre Venda")],
        "CAMBIO":         [(10813, "PTAX Venda Média"), (1, "Dólar Comercial Compra")],
        "USD":            [(10813, "PTAX Venda Média"), (1, "Dólar Comercial Compra")],
        "ANP_OCORRENCIA": [(1393, "ANP Ocorrências")],
        "ANP_PRODUCAO":   [(1393, "ANP Produção")],
        "PETROLEO_SGS":   [(1393, "Petróleo SGS")],
    }

    chain = FALLBACK_CHAINS.get(indicator.upper())
    if not chain:
        print(json.dumps({"error": f"Unknown macro indicator '{indicator}'. Supported: {', '.join(FALLBACK_CHAINS.keys())}"}))
        sys.exit(1)
        
    end_date = datetime.datetime.now()
    start_date = end_date - datetime.timedelta(days=int(years)*365)
    
    start_str = start_date.strftime('%d/%m/%Y')
    end_str = end_date.strftime('%d/%m/%Y')
    
    last_error = None
    import time
    for ind_code, label in chain:
        url = f"https://api.bcb.gov.br/dados/serie/bcdata.sgs.{ind_code}/dados?formato=json&dataInicial={start_str}&dataFinal={end_str}"
        
        max_retries = 3
        base_delay = 2
        for attempt in range(max_retries):
            try:
                req = urllib.request.Request(url, headers={'User-Agent': 'Sovereign-Pair/1.3'})
                with urllib.request.urlopen(req, timeout=30) as response:  # nosemgrep
                    resp_data = json.loads(response.read().decode())
                    
                    # Verificar se é erro estruturado do BCB
                    if isinstance(resp_data, dict) and "erro" in resp_data:
                        last_error = f"SGS {ind_code} ({label}): {resp_data['erro'].get('detail', 'Unknown')}"
                        break  # Tentar próximo fallback se BCB enviar erro válido
                    
                    data_lines = []
                    for item in resp_data:
                        date_str = normalize_date(item.get('data', ''))
                        data_lines.append(f"{date_str} | {item.get('valor', '')}")
                    
                    ctx_header = f"[CONTEXT: DADOS HISTÓRICOS BRUTOS REFERENTES AO MACRO INDICADOR {indicator.upper()}]\n"
                    data_compressed = ctx_header + "\n".join(data_lines)
                    
                    source_label = f"Banco Central do Brasil (SGS {ind_code} — {label})"
                    if len(chain) > 1 and ind_code != chain[0][0]:
                        source_label += " [FALLBACK — série primária indisponível]"
                    
                    print(json.dumps({
                        "status": "success",
                        "source": source_label,
                        "indicator": indicator, 
                        "country": country, 
                        "period": f"{years}y",
                        "data_compressed": data_compressed
                    }))
                    sys.exit(0)
                    
            except urllib.error.HTTPError as e:
                last_error = f"SGS {ind_code} ({label}): HTTP {e.code} — {e.reason}"
                if attempt < max_retries - 1:
                    time.sleep(base_delay)
                    base_delay *= 2
            except Exception as e:
                last_error = f"SGS {ind_code} ({label}): {str(e)}"
                if attempt < max_retries - 1:
                    time.sleep(base_delay)
                    base_delay *= 2
    
    # Todos os fallbacks falharam
    print(json.dumps({"error": f"Macro API Error: All {len(chain)} fallback(s) failed for '{indicator}'. Last: {last_error}"}))

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(json.dumps({"error": "Usage: sovereign_matrix.py <finance|macro> <args...>"}))
        sys.exit(1)
        
    mode = sys.argv[1].lower()
    
    if mode == "finance":
        # Usage: sovereign_matrix.py finance BRENT 5
        ticker = sys.argv[2]
        years = sys.argv[3] if len(sys.argv) > 3 else "1"
        fetch_finance(ticker, years)
        
    elif mode == "macro":
        # Usage: sovereign_matrix.py macro IPCA BR 5
        indicator = sys.argv[2]
        country = sys.argv[3] if len(sys.argv) > 3 else "BR"
        years = sys.argv[4] if len(sys.argv) > 4 else "1"
        fetch_macro(indicator, country, years)
        
    elif mode == "futures":
        # Usage: sovereign_matrix.py futures BRENT_FUTURE 5
        ticker = sys.argv[2]
        years = sys.argv[3] if len(sys.argv) > 3 else "1"
        fetch_finance(ticker, years)
        
    else:
        print(json.dumps({"error": f"Unknown mode: {mode}. Use 'finance', 'macro', or 'futures'."}))
