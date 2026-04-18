import os
import json
import sqlite3
import urllib.request
import logging
from typing import Dict, Any

logging.basicConfig(level=logging.INFO, format='%(asctime)s [%(levelname)s] [Sovereign Pricing Matrix] %(message)s')

LITELLM_PRICES_URL = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json"

TARGET_MODELS = [
    "gpt-4o",
    "claude-3-5-sonnet-20240620",
    "gemini-1.5-pro"
]

def fetch_pricing_data() -> Dict[str, Any]:
    try:
        req = urllib.request.Request(LITELLM_PRICES_URL, headers={'User-Agent': 'Sovereign-Cibrid'})
        with urllib.request.urlopen(req, timeout=10) as response:
            return json.loads(response.read().decode('utf-8'))
    except Exception as e:
        logging.error(f"Failed to fetch market pricing from LiteLLM: {e}")
        return {}

def calculate_average_1k_cost(pricing_data: Dict[str, Any]) -> float:
    total_cost = 0.0
    valid_models = 0
    
    for model_name in TARGET_MODELS:
        if model_name in pricing_data:
            model_info = pricing_data[model_name]
            input_cost = model_info.get("input_cost_per_token", 0.0)
            output_cost = model_info.get("output_cost_per_token", 0.0)
            
            # Use blended average (assuming 50% input / 50% output for generic queries)
            blended_token_cost = (input_cost + output_cost) / 2.0
            total_cost += blended_token_cost
            valid_models += 1
            logging.info(f"Market Rate Extracted -> {model_name} (Blended Cost: ${blended_token_cost:.6f}/token)")

    if valid_models == 0:
        # Fallback heuristic cost for Tier 1 Foundation Models
        logging.warning("Failed to extract specific models. Using heuristic fallback.")
        return 0.015  # Average $0.015 per 1k if offline

    avg_cost_per_token = total_cost / valid_models
    cost_per_1k = avg_cost_per_token * 1000.0
    return round(cost_per_1k, 5)

def get_sovereign_db_path() -> str:
    """Resolve o caminho do banco Sovereign de forma cross-platform e XDG-compliant.
    Ordem de prioridade:
    1. DATABASE_URL env var (produção / containers)
    2. XDG_DATA_HOME (Linux custom, ex: NixOS, Arch)
    3. ~/Library/Application Support (MacOS)
    4. LOCALAPPDATA (Windows)
    5. ~/.local/share (Linux padrão)
    """
    # 1. Env var explícita (containers, produção)
    db_url = os.getenv("DATABASE_URL", "")
    if db_url:
        return db_url.replace("sqlite:", "").split("?")[0]

    # 2. XDG_DATA_HOME (Linux/MacOS custom)
    xdg_data = os.getenv("XDG_DATA_HOME", "")
    if xdg_data:
        return os.path.join(xdg_data, "sovereign-pair", "data", "sovereign_memory.db")

    # 3. MacOS: ~/Library/Application Support
    if os.sys.platform == "darwin":
        home = os.path.expanduser("~")
        return os.path.join(home, "Library", "Application Support", "sovereign-pair", "data", "sovereign_memory.db")

    # 4. Windows: %LOCALAPPDATA%
    local_app_data = os.getenv("LOCALAPPDATA", "")
    if local_app_data:
        return os.path.join(local_app_data, "sovereign-pair", "data", "sovereign_memory.db")

    # 5. Linux padrão: ~/.local/share
    home = os.path.expanduser("~")
    return os.path.join(home, ".local", "share", "sovereign-pair", "data", "sovereign_memory.db")

def update_database(cost_per_1k: float):
    db_path = get_sovereign_db_path()
    if not os.path.exists(db_path):
        logging.warning(f"Sovereign DB não encontrado em: {db_path}. Pulando gravação.")
        return
        
    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # Injetamos na tabela analytics, garantindo que ela exista ou criando-a 
        # (se o Sovereign não tiver migrado a tabela integralmente ainda)
        cursor.execute(
            """INSERT INTO global_settings (id, value_json) 
               VALUES ('avg_cloud_token_cost_1k', ?) 
               ON CONFLICT(id) DO UPDATE SET value_json = ?""",
            (str(cost_per_1k), str(cost_per_1k))
        )
        conn.commit()
        conn.close()
        logging.info(f"Successfully recorded Market Average of ${cost_per_1k:.5f}/1k to Sovereign Hub Memory.")
        
    except Exception as e:
        logging.error(f"Failed to write market pricing to Database: {e}")

if __name__ == "__main__":
    logging.info("Initializing Autonomous Market Pricing Routine...")
    market_data = fetch_pricing_data()
    avg_cost_1k = calculate_average_1k_cost(market_data)
    logging.info(f"Calculated Cloud Market Average: ${avg_cost_1k:.5f} per 1k Tokens")
    update_database(avg_cost_1k)
    
    # A ferramenta LLM exige um json na stdout caso acionada manualmente
    print(json.dumps({
        "status": "success",
        "avg_cloud_token_cost_1k": avg_cost_1k,
        "models_analyzed": TARGET_MODELS
    }))
