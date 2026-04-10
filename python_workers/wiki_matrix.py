#!/usr/bin/env python3
import sys
import json
import urllib.request
import urllib.parse
import gzip

def fetch_wikipedia(query, lang):
    try:
        # Padrão MediaWiki API
        query_safe = urllib.parse.quote(query)
        url = f"https://{lang}.wikipedia.org/w/api.php?action=query&prop=extracts&exintro=true&explaintext=true&format=json&titles={query_safe}"
        
        req = urllib.request.Request(url, headers={'User-Agent': 'Sovereign-Pair/1.0', 'Accept-Encoding': 'gzip'})
        response = urllib.request.urlopen(req, timeout=15)
        
        if response.info().get('Content-Encoding') == 'gzip':
            data = gzip.decompress(response.read()).decode('utf-8')
        else:
            data = response.read().decode('utf-8')
            
        jdata = json.loads(data)
        pages = jdata.get("query", {}).get("pages", {})
        
        results = []
        for page_id, page_info in pages.items():
            if page_id == "-1":
                continue
            title = page_info.get("title", "")
            extract = page_info.get("extract", "").replace('\n', ' ')
            results.append(f"Title: {title} | Extract: {extract[:1000]}...")
            
        if not results:
            final_str = "Nenhum artigo encontrado ou a entidade não existe nesta língua."
        else:
            final_str = "\n".join(results)
            
        print(json.dumps({
            "status": "success",
            "source": f"Wikipedia ({lang})",
            "query": query,
            "data_compressed": final_str
        }))
    except Exception as e:
        print(json.dumps({"error": f"Wikipedia API Error: {str(e)}"}))

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(json.dumps({"error": "Usage: wiki_matrix.py <query> <lang>"}))
        sys.exit(1)
        
    query = sys.argv[1]
    lang = sys.argv[2][:2].lower()
    
    fetch_wikipedia(query, lang)
