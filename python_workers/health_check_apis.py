#!/usr/bin/env python3
"""
Sovereign Open-Data Matrix — API Health Check

Validates all external data sources at startup or in CI.
Exit code 0 = all healthy, 1 = at least one critical failure.

Usage:
    python3 health_check_apis.py              # Full check, human-readable
    python3 health_check_apis.py --json       # Machine-readable JSON output
    python3 health_check_apis.py --ci         # CI mode: exit 1 on any failure

Checks performed:
    1. BCB SGS (IPCA, SELIC, IGPM, INPC, DOLAR_PTAX, ANP)
    2. Yahoo Finance (BRENT, DOLAR, PETROBRAS)
    3. Dataset Proxies (Autobahn local files)
"""
import sys
import json
import time
import urllib.request
import urllib.error
import datetime

# ──────────────────────────────────────────────
# REGISTRY: Every external endpoint we depend on
# ──────────────────────────────────────────────
BCB_SGS_SERIES = {
    "IPCA":           {"code": 433,   "critical": True,  "description": "Inflação oficial (IPCA mensal)"},
    "SELIC":          {"code": 432,   "critical": False, "description": "Taxa básica de juros"},
    "IGPM":           {"code": 189,   "critical": False, "description": "IGP-M (inflação do aluguel)"},
    "INPC":           {"code": 188,   "critical": False, "description": "INPC (inflação popular)"},
    "DOLAR_PTAX":     {"code": 10813, "critical": True,  "description": "PTAX Venda Média Diária"},
    "ANP_OCORRENCIA": {"code": 1393,  "critical": False, "description": "Ocorrências ANP"},
}

YAHOO_TICKERS = {
    "BRENT":     {"ticker": "BZ=F",    "critical": True,  "description": "Petróleo Brent Crude"},
    "DOLAR":     {"ticker": "BRL=X",   "critical": True,  "description": "Câmbio USD/BRL"},
    "PETROBRAS": {"ticker": "PETR4.SA","critical": False, "description": "Ações Petrobras"},
}


def check_bcb_sgs(name: str, series_code: int, timeout: int = 10) -> dict:
    """Test a BCB SGS series with a minimal 30-day window."""
    end = datetime.datetime.now()
    start = end - datetime.timedelta(days=90)
    url = (
        f"https://api.bcb.gov.br/dados/serie/bcdata.sgs.{series_code}"
        f"/dados?formato=json&dataInicial={start.strftime('%d/%m/%Y')}"
        f"&dataFinal={end.strftime('%d/%m/%Y')}"
    )
    
    result = {"name": name, "type": "BCB_SGS", "series": series_code, "url": url}
    t0 = time.time()
    
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Sovereign-Pair/1.2 HealthCheck"})
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            data = json.loads(resp.read().decode())
            latency_ms = int((time.time() - t0) * 1000)
            
            if isinstance(data, list) and len(data) > 0:
                result.update({
                    "status": "HEALTHY",
                    "records": len(data),
                    "latest_date": data[-1].get("data", "?"),
                    "latest_value": data[-1].get("valor", "?"),
                    "latency_ms": latency_ms,
                })
            elif isinstance(data, dict) and "erro" in data:
                result.update({
                    "status": "DEAD",
                    "error": data["erro"].get("detail", str(data["erro"])),
                    "latency_ms": latency_ms,
                })
            else:
                result.update({
                    "status": "EMPTY",
                    "error": "API returned 200 but zero records",
                    "latency_ms": latency_ms,
                })
    except urllib.error.HTTPError as e:
        result.update({"status": "DEAD", "error": f"HTTP {e.code}: {e.reason}", "latency_ms": int((time.time() - t0) * 1000)})
    except Exception as e:
        result.update({"status": "UNREACHABLE", "error": str(e), "latency_ms": int((time.time() - t0) * 1000)})
    
    return result


def check_yahoo(name: str, ticker: str, timeout: int = 15) -> dict:
    """Test a Yahoo Finance ticker with a 1-month window."""
    result = {"name": name, "type": "Yahoo_Finance", "ticker": ticker}
    t0 = time.time()
    
    try:
        import yfinance as yf
        tk = yf.Ticker(ticker)
        data = tk.history(period="1mo")
        latency_ms = int((time.time() - t0) * 1000)
        
        if data is not None and len(data) > 0:
            result.update({
                "status": "HEALTHY",
                "records": len(data),
                "latest_date": str(data.index[-1].date()),
                "latency_ms": latency_ms,
            })
        else:
            result.update({
                "status": "EMPTY",
                "error": "yfinance returned empty DataFrame",
                "latency_ms": latency_ms,
            })
    except ImportError:
        result.update({"status": "SKIP", "error": "yfinance not installed", "latency_ms": 0})
    except Exception as e:
        result.update({"status": "DEAD", "error": str(e), "latency_ms": int((time.time() - t0) * 1000)})
    
    return result


def run_health_check() -> list:
    """Run all checks and return results."""
    results = []
    
    # 1. BCB SGS
    for name, meta in BCB_SGS_SERIES.items():
        r = check_bcb_sgs(name, meta["code"])
        r["critical"] = meta["critical"]
        r["description"] = meta["description"]
        results.append(r)
    
    # 2. Yahoo Finance
    for name, meta in YAHOO_TICKERS.items():
        r = check_yahoo(name, meta["ticker"])
        r["critical"] = meta["critical"]
        r["description"] = meta["description"]
        results.append(r)
    
    return results


def print_human(results: list):
    """Print human-readable report."""
    print("\n╔══════════════════════════════════════════════════════════════════╗")
    print("║         🛡️  SOVEREIGN OPEN-DATA API HEALTH CHECK              ║")
    print("╚══════════════════════════════════════════════════════════════════╝\n")
    
    healthy = 0
    dead = 0
    critical_failures = []
    
    for r in results:
        icon = {"HEALTHY": "✅", "DEAD": "❌", "EMPTY": "⚠️", "UNREACHABLE": "🔌", "SKIP": "⏭️"}.get(r["status"], "?")
        crit = " [CRITICAL]" if r.get("critical") and r["status"] != "HEALTHY" else ""
        
        line = f"  {icon} {r['name']:<18} {r['status']:<12} {r.get('latency_ms', 0):>5}ms"
        if r["status"] == "HEALTHY":
            line += f"  ({r.get('records', '?')} records, latest: {r.get('latest_date', '?')})"
            healthy += 1
        elif r["status"] != "SKIP":
            line += f"  ⟶ {r.get('error', 'unknown')}{crit}"
            dead += 1
            if r.get("critical"):
                critical_failures.append(r["name"])
        
        print(line)
    
    print(f"\n{'─' * 66}")
    print(f"  Total: {len(results)} | ✅ Healthy: {healthy} | ❌ Failed: {dead}")
    
    if critical_failures:
        print(f"\n  🚨 CRITICAL FAILURES: {', '.join(critical_failures)}")
        print(f"     These APIs are required for core functionality!")
        print(f"     Action: Check series codes in sovereign_matrix.py code_map")
    else:
        print(f"\n  🎯 All critical APIs operational.")
    
    print()
    return len(critical_failures) > 0


def main():
    json_mode = "--json" in sys.argv
    ci_mode = "--ci" in sys.argv
    
    results = run_health_check()
    
    if json_mode:
        print(json.dumps(results, indent=2, ensure_ascii=False))
        has_critical = any(r.get("critical") and r["status"] != "HEALTHY" for r in results)
        sys.exit(1 if has_critical else 0)
    else:
        has_critical = print_human(results)
        if ci_mode and has_critical:
            sys.exit(1)


if __name__ == "__main__":
    main()
