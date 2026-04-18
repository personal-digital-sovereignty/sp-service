#!/usr/bin/env python3
import sys
import json
import sqlite3
import os
import urllib.request
import urllib.parse
import urllib.error

def get_db_path():
    """Resolve o DB Sovereign de forma cross-platform e XDG-compliant.
    LIN-02 FIX: Nome do banco corrigido (era SovereignHub_OS_System.db → sovereign_memory.db)
    """
    # 1. Env var explícita (containers, produção)
    db_url = os.getenv("DATABASE_URL", "")
    if db_url:
        return db_url.replace("sqlite:", "").split("?")[0]

    # 2. XDG_DATA_HOME (NixOS, Arch, Fedora custom)
    xdg_data = os.getenv("XDG_DATA_HOME", "")
    if xdg_data:
        return os.path.join(xdg_data, "sovereign-pair", "data", "sovereign_memory.db")

    # 3. MacOS: ~/Library/Application Support
    if sys.platform == "darwin":
        home_dir = os.path.expanduser("~")
        return os.path.join(home_dir, "Library", "Application Support", "sovereign-pair", "data", "sovereign_memory.db")

    # 4. Windows: %LOCALAPPDATA%
    local_app_data = os.getenv("LOCALAPPDATA", "")
    if local_app_data:
        return os.path.join(local_app_data, "sovereign-pair", "data", "sovereign_memory.db")

    # 5. Linux padrão: ~/.local/share (XDG Base Dir spec)
    home_dir = os.path.expanduser("~")
    return os.path.join(home_dir, ".local", "share", "sovereign-pair", "data", "sovereign_memory.db")


def check_tenant_key(provider_name):
    # Conectividade direta de leitura com o Sovereign SecOps Vault
    db_path = get_db_path()
    if not os.path.exists(db_path):
        return None
        
    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("SELECT api_key_value FROM tenant_api_keys WHERE provider_name = ?", (provider_name,))
        row = cursor.fetchone()
        conn.close()
        
        if row:
            return row[0] # Retorna a cifra (AES-GCM at-rest) criptografada
    except Exception:
        pass
        
    return None

def fetch_cultural_data(query, source):
    # Boilerplate: Verifica no SecOps Vault se o User Tenant possui uma chave privada!
    vault_entry = check_tenant_key(f"{source.upper()}_API_KEY")
    
    auth_status = "Using Public Open-Endpoints (Rate Limited / Free Tier)"
    if vault_entry:
        auth_status = "SecOps Vault Key Detected! Bypassing public limits."
        
    try:
        results = []
        if source.upper() == "TMDB":
            if not vault_entry:
                results.append(f"TMDB (The Movie Database): Chave Mestra Ausente. Para consultar bilheteria e enredo real de '{query}', cadastre a 'TMDB_API_KEY' no Vault Corporativo Sovereign.")
            else:
                results.append(f"TMDB request for '{query}' interceptado. A chave exsite no Vault, mas precisa ser injetada de forma segura na Rust Engine. (Mock data returned)")

        elif source.upper() == "RAWG":
            if not vault_entry:
                results.append(f"RAWG.io (Video Games Database): Chave Mestra Ausente. Cadastre a 'RAWG_API_KEY' no Vault para extrair recepção (Metacritic) e lançamentos do jogo '{query}'.")
            else:
                results.append(f"RAWG.io request para '{query}' interceptado e Mocked pois a chave existe.")
                
        elif source.upper() == "MUSICBRAINZ":
            url = f"https://musicbrainz.org/ws/2/artist/?query={urllib.parse.quote(query)}&fmt=json"
            req = urllib.request.Request(url, headers={"User-Agent": "SovereignPair/1.2.0 ( admin@local )"})
            try:
                with urllib.request.urlopen(req) as response:
                    payload = json.loads(response.read().decode())
                    artists = payload.get("artists", [])
                    if artists:
                        artist = artists[0]
                        results.append(f"### MusicBrainz Data for: {artist.get('name')}")
                        results.append(f"- Tipo: {artist.get('type', 'Unknown')} | País Origem: {artist.get('country', 'Unknown')}")
                        tags = [t.get("name") for t in artist.get("tags", [])][:7]
                        results.append(f"- Gêneros Relacionados: {', '.join(tags)}")
                        if "life-span" in artist:
                            results.append(f"- Período Ativo: {artist['life-span'].get('begin', '?')} to {artist['life-span'].get('end', 'Present')}")
                        
                        artist_id = artist.get("id")
                        if artist_id:
                            rel_url = f"https://musicbrainz.org/ws/2/release-group?artist={artist_id}&type=album|ep&fmt=json"
                            rel_req = urllib.request.Request(rel_url, headers={"User-Agent": "SovereignPair/1.2.0 ( admin@local )"})
                            try:
                                with urllib.request.urlopen(rel_req) as rel_response:
                                    rel_payload = json.loads(rel_response.read().decode())
                                    groups = rel_payload.get("release-groups", [])
                                    releases = []
                                    for g in groups[:15]:
                                        date = g.get('first-release-date', '????')[:4]
                                        releases.append(f"[{date}] {g.get('title')} ({g.get('primary-type', 'Album')})")
                                    results.append("\n### TOP DISCOGRAFIA (Compactada para RAG):\n" + "\n".join(releases))
                            except Exception as e:
                                results.append(f"[Aviso] Falha ao extrair discografia profunda: {str(e)}")
                    else:
                        results.append(f"MusicBrainz: Nenhum artista/banda encontrado para '{query}'.")
            except urllib.error.URLError as e:
                results.append(f"MusicBrainz Erro de Conexão: {str(e)}")
                
        elif source.upper() == "THEMET":
            url = f"https://collectionapi.metmuseum.org/public/collection/v1/search?q={urllib.parse.quote(query)}"
            try:
                with urllib.request.urlopen(url) as response:
                    payload = json.loads(response.read().decode())
                    obj_ids = payload.get("objectIDs")
                    if obj_ids:
                        results.append(f"### The Met Collection: Encontrados {payload.get('total')} itens históricos para '{query}'.")
                        for oid in obj_ids[:3]:
                            o_url = f"https://collectionapi.metmuseum.org/public/collection/v1/objects/{oid}"
                            try:
                                with urllib.request.urlopen(o_url) as o_res:
                                    o_data = json.loads(o_res.read().decode())
                                    title = o_data.get('title', 'Art')
                                    author = o_data.get('artistDisplayName') or 'Desconhecido'
                                    date = o_data.get('objectDate') or 'Sem Data'
                                    dept = o_data.get('department', '')
                                    results.append(f"- '{title}' por {author} ({date}) | Depto: {dept}")
                            except Exception:
                                pass
                    else:
                        results.append(f"The Met: Nenhum artefato de arte ou autor encontrado para '{query}'.")
            except urllib.error.URLError as e:
                results.append(f"The Met Collection Erro: {str(e)}")
                
        elif source.upper() == "WIKIPEDIA":
            url = f"https://pt.wikipedia.org/api/rest_v1/page/summary/{urllib.parse.quote(query.replace(' ', '_'))}"
            req = urllib.request.Request(url, headers={"User-Agent": "SovereignPair/1.2.0 ( admin@local )"})
            try:
                with urllib.request.urlopen(req) as response:
                    payload = json.loads(response.read().decode())
                    results.append(f"### Wikipedia Abstract (PT-BR Público)\n{payload.get('extract', 'Resumo não encontrado.')}")
            except urllib.error.HTTPError as e:
                if e.code == 404:
                    results.append(f"Wikipedia: O termo exato '{query}' não possui página direta ou sofre de disambiguação. Tente refinar a busca no RAG.")
                else:
                    results.append(f"Wikipedia HTTP Error: {e.code}")

        else:
            results.append(f"Consultando banco cultural genérico sobre '{query}'. Algoritmo desconhecido.")

        final_str = "\n".join(results)
        
        print(json.dumps({
            "status": "success",
            "source": f"Sovereign Cultural Bridge ({source})",
            "auth": auth_status,
            "query": query,
            "data_compressed": final_str
        }))
    except Exception as e:
        print(json.dumps({"error": f"Cultural Matrix Error: {str(e)}"}))

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(json.dumps({"error": "Usage: culture_matrix.py <query> <source>"}))
        sys.exit(1)
        
    query = sys.argv[1]
    source = sys.argv[2]
    
    fetch_cultural_data(query, source)
