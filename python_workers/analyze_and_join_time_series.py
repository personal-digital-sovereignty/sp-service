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
    # IMPORTANTE: chaves mais longas/específicas DEVEM vir antes das mais curtas!
    # Ex: DOLAR_PTAX antes de DOLAR, senão "DOLAR" match prematuramente.
    SEMANTIC_MAP = [
        ("BZ=F", "BRENT"), ("BRENT", "BRENT"), ("PETROLEO", "BRENT"), ("PETRÓLEO", "BRENT"),
        ("DOLAR_PTAX", "DOLAR_PTAX"),  # Antes de DOLAR!
        ("BRL=X", "DOLAR"), ("DÓLAR", "DOLAR"),
        ("GASOLINA", "GASOLINA"), ("DIESEL", "DIESEL"),
        ("IPCA", "IPCA"), ("SELIC", "SELIC"), ("DESEMPREGO", "DESEMPREGO"),
        ("CAMBIO", "CAMBIO"),
    ]
    
    def normalize_ds_name(raw_name, block_text):
        """Normaliza o nome do dataset usando mapa semântico."""
        upper = raw_name.upper()
        for key, val in SEMANTIC_MAP:
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
    
    # Resolução de colunas de câmbio:
    # - DOLAR (BRL=X Yahoo): Taxa spot interbancária COM prêmio de risco/forward premium
    # - DOLAR_PTAX (BCB SGS 10813): Média ponderada de transações REAIS (câmbio oficial)
    # - Taxa_Cambio: Derivada do ticker BRENT (BRL÷USD) — redundante quando PTAX existe
    #
    # Regra: "Dá-se por real o fechamento pago, não a promessa."
    # PTAX é o índice oficial para impostos, PPI da Petrobras e impacto ao consumidor.
    # BRL=X é complementar para análise de tendência e volatilidade.
    has_ptax = 'DOLAR_PTAX' in merged_df.columns
    has_dolar = 'DOLAR' in merged_df.columns
    has_taxa = 'Taxa_Cambio' in merged_df.columns

    if has_dolar:
        # Renomear DOLAR (BRL=X) para DOLAR_SPOT para clareza semântica
        merged_df.rename(columns={'DOLAR': 'DOLAR_SPOT'}, inplace=True)

    if has_taxa:
        if has_ptax:
            # PTAX é a fonte oficial — Taxa_Cambio derivada é redundante
            merged_df.drop(columns=['Taxa_Cambio'], inplace=True)
        elif has_dolar:
            # Sem PTAX disponível, Taxa_Cambio é derivada e DOLAR_SPOT já cobre
            merged_df.drop(columns=['Taxa_Cambio'], inplace=True)
        else:
            # Taxa_Cambio é o único câmbio disponível — renomear para DOLAR_CAMBIO
            merged_df.rename(columns={'Taxa_Cambio': 'DOLAR_CAMBIO'}, inplace=True)
    
    # Sort chronological
    merged_df.sort_index(inplace=True)
    
    # Create the pearson correlation 
    corr_matrix = merged_df.corr(method='pearson').round(3)
    
    # -----------------------------
    # EPISTEMIC RULES (v1.2.4+)
    # -----------------------------
    # Discriminated ffill: Stock variables (prices that "stay" until changed) get ffill(limit=3).
    # Flow variables (IPCA — unique monthly measurement) are NEVER forward-filled.
    # This prevents ghost inflation values (e.g. Apr/2026 = copy of Mar/2026).
    STOCK_COLUMNS = ['GASOLINA', 'DOLAR_SPOT', 'DOLAR_PTAX', 'DOLAR_CAMBIO', 'DIESEL',
                     'BRENT_USD', 'BRENT_BRL', 'SELIC']
    FLOW_COLUMNS = ['IPCA']  # Variáveis de fluxo mensal — NaN até dado real chegar

    for col in merged_df.columns:
        if col in FLOW_COLUMNS:
            pass  # NUNCA ffill — NaN é semanticamente correto
        elif col in STOCK_COLUMNS or col not in FLOW_COLUMNS:
            merged_df[col] = merged_df[col].ffill(limit=3)  # Máximo 3 meses sem dado real
    
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
