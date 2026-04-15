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
    except:
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
    
    semantic_name = ticker
    if ticker.upper() == 'BRENT':
        ticker = 'BZ=F'
        semantic_name = 'Barril de Petróleo (BRENT Crude)'
    elif ticker.upper() == 'WTI':
        ticker = 'CL=F'
        semantic_name = 'Barril de Petróleo (WTI Crude Spot)'
    elif ticker.upper() == 'DOLAR' or ticker.upper() == 'USD':
        ticker = 'BRL=X'
        semantic_name = 'Taxa de Câmbio (Dólar / BRL)'
    elif ticker.upper() == 'PETROBRAS':
        ticker = 'PETR4.SA'
        semantic_name = 'Ações Petrobras (PETR4)'
    elif ticker.upper() == 'BRENT_FUTURE':
        ticker = 'BZ=F'
        semantic_name = 'Contrato Futuro Brent (Especulativo)'
    elif ticker.upper() == 'WTI_FUTURE':
        ticker = 'CL=F'
        semantic_name = 'Contrato Futuro WTI (Especulativo)'
    elif ticker.upper() == 'GOLD_FUTURE':
        ticker = 'GC=F'
        semantic_name = 'Contrato Futuro Ouro (Especulativo)'
    elif ticker.upper() == 'DI_FUTURE':
        ticker = 'DI1F27.SA' # Proxy genérico para DI
        semantic_name = 'Contrato DI Futuro (Especulativo)'
        
        

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
        last_error_layer1 = f"Layer 1 (yfinance) failed: {e}"
        
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
        
    converted_to_brl = False
    if ticker in ['BZ=F', 'CL=F']:
        # Fetch BRL=X to do Currency Conversion
        df_usd = pd.DataFrame()
        try:
            t_usd = yf.Ticker('BRL=X')
            df_usd = t_usd.history(period=period)
        except:
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
            except:
                pass
                
        if not df_usd.empty:
            try:
                if int(clean_years) > 1:
                    df['YearMonth'] = df.index.strftime('%Y-%m')
                    df = df.groupby('YearMonth').mean()
                    
                    df_usd['YearMonth'] = df_usd.index.strftime('%Y-%m')
                    df_usd = df_usd.groupby('YearMonth').mean()
                    
                    df = df.join(df_usd['Close'], rsuffix='_usd', how='inner')
                    df['Close_brl'] = df['Close'] * df['Close_usd']
                    converted_to_brl = True
                    source_used += " | (+ Converted to BRL)"
            except:
                pass
                
    if not converted_to_brl:
        # Normal grouping if not already grouped by currency conversion
        try:
            if int(clean_years) > 1:
                df['YearMonth'] = df.index.strftime('%Y-%m')
                df = df.groupby('YearMonth').mean()
        except:
            pass
            
    # === ANALYST B: CIRCUIT BREAKER (DATA QUALITY ASSERTION) ===
    if ticker in ['BZ=F', 'CL=F'] and not df.empty and 'Close' in df.columns:
        max_usd = float(df['Close'].max())
        min_usd = float(df['Close'].min())
        if max_usd > 200.0 or min_usd < 5.0:
            print(json.dumps({"error": f"CRÍTICO (Circuit Breaker): Anomalia estrutural no Ticker {ticker}. Valor histórico em USD (Max: {round(max_usd, 2)}, Min: {round(min_usd, 2)}) rompeu barreira de viabilidade física do mercado de petróleo bruto. Abortando injeção para prevenir alucinação estatística (Dissonância Cognitiva) na Mente Mestra."}))
            sys.exit(1)
            
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
                except Exception as e:
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
    for ind_code, label in chain:
        url = f"https://api.bcb.gov.br/dados/serie/bcdata.sgs.{ind_code}/dados?formato=json&dataInicial={start_str}&dataFinal={end_str}"
        
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'Sovereign-Pair/1.2'})
            with urllib.request.urlopen(req, timeout=30) as response:  # nosemgrep
                resp_data = json.loads(response.read().decode())
                
                # Verificar se é erro estruturado do BCB
                if isinstance(resp_data, dict) and "erro" in resp_data:
                    last_error = f"SGS {ind_code} ({label}): {resp_data['erro'].get('detail', 'Unknown')}"
                    continue  # Tentar próximo fallback
                
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
            continue  # Tentar próximo fallback
        except Exception as e:
            last_error = f"SGS {ind_code} ({label}): {str(e)}"
            continue  # Tentar próximo fallback
    
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
