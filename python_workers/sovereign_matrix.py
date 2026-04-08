#!/usr/bin/env python3
import sys
import json
import datetime
import urllib.request
import urllib.error

def fetch_finance(ticker, years):
    # Cross-Router: Forgive LLM mapping hallucinations for known macro items
    if ticker.upper() in ["GASOLINA", "DIESEL", "IPCA", "IGPM", "SELIC", "INPC", "OURO", "ARROZ"]:
        return fetch_macro(ticker.upper(), "BR", years)
        
    try:
        import yfinance as yf
        import pandas as pd
    except ImportError:
        print(json.dumps({"error": "Packages 'yfinance' and 'pandas' are missing. Run 'pip install yfinance pandas'."}))
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
        semantic_name = 'Barril de Petróleo (WTI Crude)'
    elif ticker.upper() == 'DOLAR' or ticker.upper() == 'USD':
        ticker = 'BRL=X'
        semantic_name = 'Taxa de Câmbio (Dólar / BRL)'
    elif ticker.upper() == 'PETROBRAS':
        ticker = 'PETR4.SA'
        semantic_name = 'Ações Petrobras (PETR4)'
        
        

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
            with urllib.request.urlopen(req, timeout=10) as response:
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
            with urllib.request.urlopen(req, timeout=10) as response:
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
                with urllib.request.urlopen(req, timeout=10) as response:
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
                    df = df.groupby('YearMonth').last()
                    
                    df_usd['YearMonth'] = df_usd.index.strftime('%Y-%m')
                    df_usd = df_usd.groupby('YearMonth').last()
                    
                    df = df.join(df_usd['Close'], rsuffix='_usd', how='inner')
                    df['Close'] = df['Close'] * df['Close_usd']
                    converted_to_brl = True
                    source_used += " | (+ Converted to BRL)"
            except:
                pass
                
    if not converted_to_brl:
        # Normal grouping if not already grouped by currency conversion
        try:
            if int(clean_years) > 1:
                df['YearMonth'] = df.index.strftime('%Y-%m')
                df = df.groupby('YearMonth').last()
        except:
            pass
            
    data_lines = []
    for index, row in df.iterrows():
        date_str = index if isinstance(index, str) else index.strftime('%Y-%m-%d')
        # Lida com casos onde row['Close'] seja NaN no Pandas
        if pd.isna(row.get('Close', float('nan'))):
             continue
        val = round(float(row['Close']), 2)
        data_lines.append(f"{date_str} | {val}")
        
    ctx_header = f"[CONTEXT: DADOS HISTÓRICOS BRUTOS REFERENTES AO ATIVO {ticker.upper()} ({semantic_name})]\n"
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
    # Lexical Forgiveness for LLMs that use descriptive queries
    if "GASOLINA" in indicator.upper(): indicator = "GASOLINA"
    elif "DIESEL" in indicator.upper(): indicator = "DIESEL"
    
    if country.upper() != 'BR':
        print(json.dumps({"error": "Currently only 'BR' macroeconomic indicators are supported natively."}))
        sys.exit(1)
        
    # === AUTOBAHN ROUTER (Dynamic Proxy Resolution) ===
    import os
    proxy_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dataset_proxies")
    proxy_file = os.path.join(proxy_dir, f"{indicator.lower()}.json")
    
    if os.path.exists(proxy_file):
        try:
            with open(proxy_file, "r", encoding="utf-8") as f:
                proxy_data = json.load(f)
            
            data_arr = proxy_data.get("data", [])
            data_lines = []
            for item in data_arr:
                date_str = item.get("date", "")
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
            

    code_map = {
        "IPCA": 433,
        "SELIC": 432,
        "IGPM": 189,
        "INPC": 188
    }
    
    ind_code = code_map.get(indicator.upper())
    if not ind_code:
        print(json.dumps({"error": f"Unknown macro indicator '{indicator}'. Supported: IPCA, SELIC, IGPM, INPC"}))
        sys.exit(1)
        
    end_date = datetime.datetime.now()
    start_date = end_date - datetime.timedelta(days=int(years)*365)
    
    start_str = start_date.strftime('%d/%m/%Y')
    end_str = end_date.strftime('%d/%m/%Y')
    
    url = f"https://api.bcb.gov.br/dados/serie/bcdata.sgs.{ind_code}/dados?formato=json&dataInicial={start_str}&dataFinal={end_str}"
    
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Sovereign-Pair/1.0'})
        with urllib.request.urlopen(req) as response:
            resp_data = json.loads(response.read().decode())
            data_lines = []
            for item in resp_data:
                data_lines.append(f"{item.get('data', '')} | {item.get('valor', '')}")
            
            ctx_header = f"[CONTEXT: DADOS HISTÓRICOS BRUTOS REFERENTES AO MACRO INDICADOR {indicator.upper()}]\n"
            data_compressed = ctx_header + "\n".join(data_lines)
            
            print(json.dumps({
                "status": "success",
                "source": f"Banco Central do Brasil (SGS {ind_code})",
                "indicator": indicator, 
                "country": country, 
                "period": f"{years}y", 
                "data_compressed": data_compressed
            }))
    except Exception as e:
        print(json.dumps({"error": f"Macro API Error: {str(e)}"}))

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
        
    else:
        print(json.dumps({"error": f"Unknown mode: {mode}. Use 'finance' or 'macro'."}))
