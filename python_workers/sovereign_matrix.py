#!/usr/bin/env python3
import sys
import json
import datetime
import urllib.request
import urllib.error

def fetch_finance(ticker, years):
    try:
        import yfinance as yf
        import pandas as pd
    except ImportError:
        print(json.dumps({"error": "Packages 'yfinance' and 'pandas' are missing. Run 'pip install yfinance pandas'."}))
        sys.exit(1)
        
    try:
        period = f"{years}y"
        
        # Sovereign Financial Ticker Maps
        if ticker.upper() == 'BRENT':
            ticker = 'BZ=F' # Brent Crude Oil Futures
        elif ticker.upper() == 'WTI':
            ticker = 'CL=F' # Crude Oil Futures
        elif ticker.upper() == 'DOLAR' or ticker.upper() == 'USD':
            ticker = 'BRL=X' # USD to BRL
        elif ticker.upper() == 'PETROBRAS':
            ticker = 'PETR4.SA'
            
        t = yf.Ticker(ticker)
        df = t.history(period=period)
        
        if df.empty:
            print(json.dumps({"error": f"No financial data found for ticker {ticker}"}))
            sys.exit(1)
            
        # Context Window Protection: If querying more than 1 year, aggregate to monthly closing prices
        try:
            if int(years) > 1:
                # Group by Month to prevent 1200+ row overflow
                df['YearMonth'] = df.index.strftime('%Y-%m')
                df = df.groupby('YearMonth').last()
        except:
            pass # fallback to RAW if pandas grouping fails
            
        data = []
        for index, row in df.iterrows():
            date_str = index if isinstance(index, str) else index.strftime('%Y-%m-%d')
            data.append({
                "date": date_str,
                "close": round(row['Close'], 2)
            })
            
        print(json.dumps({
            "status": "success",
            "source": "Yahoo Finance Open-Data",
            "ticker": ticker, 
            "period": period, 
            "data": data
        }))
        
    except Exception as e:
        print(json.dumps({"error": f"Finance API Error: {str(e)}"}))

def fetch_macro(indicator, country, years):
    if country.upper() != 'BR':
        print(json.dumps({"error": "Currently only 'BR' macroeconomic indicators are supported natively."}))
        sys.exit(1)
        
    # Banco Central do Brasil SGS (Sistema Gerenciador de Séries Temporais)
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
            print(json.dumps({
                "status": "success",
                "source": f"Banco Central do Brasil (SGS {ind_code})",
                "indicator": indicator, 
                "country": country, 
                "period": f"{years}y", 
                "data": resp_data
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
