#!/usr/bin/env python3
import sys
import json
import re
import datetime

try:
    import pandas as pd
except ImportError:
    import subprocess
    try:
        subprocess.check_call([sys.executable, "-m", "pip", "install", "-q", "pandas"])
        import pandas as pd
    except:
        print(json.dumps({"error": "Failed to install required pandas package"}))
        sys.exit(1)

def parse_markdown_blocks(raw_blocks):
    """
    Parses multiple raw strings formatted as:
    [CONTEXT: DADOS HISTÓRICOS BRUTOS REFERENTES AO MACRO INDICADOR IPCA]
    2024-01-01 | 0.42
    2024-02-01 | 0.83
    """
    datasets = {}
    
    # regex for getting table headers
    header_regex = re.compile(r'\[CONTEXT: DADOS HISTÓRICOS BRUTOS REFERENTES AO.*?([A-Z0-9_\-\.=\ \(\)]+)\]')
    # regex for generic date string and numbers  "2024-01 | USD 75.3 | BRL 350.2" or "2024-01-10 | 0.5"
    row_regex = re.compile(r'^(\d{4}-\d{2}(?:-\d{2})?)\s*\|\s*(.*)$')
    
    # Mapa semântico: normaliza nomes longos de headers para colunas curtas e legíveis
    SEMANTIC_MAP = {
        "BZ=F": "BRENT", "BRENT": "BRENT", "PETROLEO": "BRENT", "PETRÓLEO": "BRENT",
        "BRL=X": "DOLAR", "DOLAR": "DOLAR", "DÓLAR": "DOLAR", "USD": "DOLAR",
        "GASOLINA": "GASOLINA", "DIESEL": "DIESEL",
        "IPCA": "IPCA", "SELIC": "SELIC", "DESEMPREGO": "DESEMPREGO",
        "DOLAR_PTAX": "DOLAR_PTAX", "CAMBIO": "CAMBIO",
    }
    
    def normalize_ds_name(raw_name, block_text):
        """Normaliza o nome do dataset usando mapa semântico."""
        upper = raw_name.upper()
        for key, val in SEMANTIC_MAP.items():
            if key in upper:
                return val
        # Fallback: heurística pelo conteúdo do bloco
        block_upper = block_text.upper()
        if "PETROLEO" in block_upper or "BRENT" in block_upper or "BZ=F" in block_upper:
            return "BRENT"
        elif "BRL=X" in block_upper:
            return "DOLAR"
        elif "IPCA" in block_upper:
            return "IPCA"
        elif "GASOLINA" in block_upper:
            return "GASOLINA"
        return raw_name.strip()[:20]  # Truncar nomes longos
    
    for block in raw_blocks:
        current_ds_name = "UNKNOWN"
        lines = block.split('\n')
        
        # Encontrar o nome do dataset no header
        for line in lines:
            h_match = header_regex.search(line)
            if h_match:
                raw_header = h_match.group(1).strip()
                current_ds_name = normalize_ds_name(raw_header, block)
                break
                
        # Se for UNKNOWN tenta inferir por heurística simples
        if current_ds_name == "UNKNOWN":
            current_ds_name = normalize_ds_name("", block)
            if current_ds_name == "":
                current_ds_name = f"DATASET_{len(datasets)}"

        if current_ds_name in datasets:
            current_ds_name = f"{current_ds_name}_{len(datasets)}"
            
        data = []
        for line in lines:
            line = line.strip()
            if not line or line.startswith('['):
                continue
                
            r_match = row_regex.search(line)
            if r_match:
                date_str = r_match.group(1)
                vals_raw = r_match.group(2)
                
                # Extract first floating point found:
                # "USD 70.3 | BRL 300.5" -> get 70.3 as primary
                # We can try to extract all numbers, but let's take the first one or specifically map it.
                numbers = re.findall(r'-?\d+\.\d+|-?\d+', vals_raw)
                if numbers:
                    if 'USD' in vals_raw and 'BRL' in vals_raw and len(numbers) >= 2:
                        # Extract both explicit values directly from string
                        usd_val = float(numbers[0])
                        brl_val = float(numbers[1])
                        
                        # Only provide Cambio if USD is non-zero to avoid division by zero
                        if usd_val > 0.0:
                            cambio = round(brl_val / usd_val, 2)
                            data.append({'Date': date_str, f"{current_ds_name}_USD": usd_val, f"{current_ds_name}_BRL": brl_val, "Taxa_Cambio": cambio})
                        else:
                            data.append({'Date': date_str, f"{current_ds_name}_USD": usd_val, f"{current_ds_name}_BRL": brl_val})
                    else:
                        # Single asset fallback
                        val = float(numbers[-1]) if 'BRL' in vals_raw else float(numbers[0])
                        data.append({'Date': date_str, current_ds_name: val})
                    
        if data:
            df = pd.DataFrame(data)
            df['Date'] = pd.to_datetime(df['Date'], errors='coerce')
            df = df.dropna(subset=['Date'])
            df.set_index('Date', inplace=True)
            # Resample to monthly end ('ME' or 'M') to normalize Daily vs Monthly
            df = df.resample('ME').mean()
            datasets[current_ds_name] = df
            
    return datasets

def join_and_extract(raw_data_blocks):
    datasets_dict = parse_markdown_blocks(raw_data_blocks)
    
    if len(datasets_dict) == 0:
        return json.dumps({"error": "No temporal datasets found to join."})
        
    dfs = list(datasets_dict.values())
    
    if len(dfs) == 1:
        # Nothing to join
        final_df = dfs[0]
        matrix_str = final_df.to_markdown()
        output = f"> [!TIP]\n> **Matrix Engine**: Apenas um dataset identificado. Dados formatados nativamente.\n\n{matrix_str}"
        return json.dumps({"status": "success", "markdown": output})
        
    from functools import reduce
    # Outer join all datasets by Date
    merged_df = reduce(lambda left, right: pd.merge(left, right, on='Date', how='outer'), dfs)
    
    # Dedup: Se 'DOLAR' e 'Taxa_Cambio' coexistem e são idênticos (ou quase),
    # manter apenas 'DOLAR_CAMBIO' renomeado para clareza e eliminar a duplicata.
    if 'DOLAR' in merged_df.columns and 'Taxa_Cambio' in merged_df.columns:
        # Usar DOLAR como a coluna canônica (fonte oficial BRL=X), renomear
        merged_df.rename(columns={'DOLAR': 'DOLAR_CAMBIO'}, inplace=True)
        merged_df.drop(columns=['Taxa_Cambio'], inplace=True)
    
    # Sort chronological
    merged_df.sort_index(inplace=True)
    
    # Create the pearson correlation 
    corr_matrix = merged_df.corr(method='pearson').round(3)
    
    # -----------------------------
    # EPISTEMIC RULES
    # -----------------------------
    # 1. Forward Fill (ffill) safely allows last known prices/indicators to flow down to missing edges.
    merged_df.ffill(inplace=True)
    
    # Pre-calculate Annual Averages before changing index to strings
    try:
        annual_df = merged_df.resample('YE').mean()
        annual_df.index = annual_df.index.strftime('%Y')
        annual_md = annual_df.round(2).to_markdown()
    except Exception:
        annual_md = "N/A"
    
    # 2. Format the index to YYYY-MM
    merged_df.index = merged_df.index.strftime('%Y-%m')
    
    # 3. Any remaining NaNs (usually at start) - round first, then convert to string-safe fill.
    # Pandas 2.x+ rejects fillna(string) on float64 columns.
    merged_df = merged_df.round(2)
    for col in merged_df.columns:
        merged_df[col] = merged_df[col].astype(object).fillna("—")
    
    # Prepare Markdown Output
    table_md = merged_df.to_markdown()
    corr_md = corr_matrix.to_markdown()
    
    alert_box = (
        "> [!NOTE]\n"
        "> **Sovereign Symbiotic Pipeline**: Múltiplas séries temporais detectadas e fundidas nativamente pelo Backend.\n"
        "> Preenchimento automático `ffill()` aplicado para alinhar assimetrias de extração (Diário vs Mensal).\n\n"
        "### Matriz de Correlação de Pearson ($r$)\n"
        f"{corr_md}\n\n"
        "### Médias Anuais Consolidadas\n"
        f"{annual_md}\n\n"
        "### Time-Series Consolidada\n"
        f"{table_md}"
    )
    
    return json.dumps({"status": "success", "markdown": alert_box})

if __name__ == "__main__":
    if len(sys.argv) < 2:
        # Pode estar mandando por Stdin JSON (Ex: Tool call arguments)
        try:
            input_data = sys.stdin.read()
            payload = json.loads(input_data)
            blocks = payload.get("raw_data_blocks", [])
            print(join_and_extract(blocks))
        except Exception as e:
            print(json.dumps({"error": f"Invalid input format: {e}"}))
    else:
        # Modos CLI de teste
        mode = sys.argv[1].lower()
        print(json.dumps({"error": f"Modo nao suportado via argumentos. Use stdin json com raw_data_blocks."}))
