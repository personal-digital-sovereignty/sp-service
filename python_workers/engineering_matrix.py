#!/usr/bin/env python3
import sys
import json
import urllib.request
import urllib.parse
import gzip

def fetch_stackexchange(topic):
    try:
        url = f"https://api.stackexchange.com/2.3/search/advanced?order=desc&sort=relevance&q={urllib.parse.quote(topic)}&site=stackoverflow&filter=withbody"
        req = urllib.request.Request(url, headers={'User-Agent': 'Sovereign-Pair', 'Accept-Encoding': 'gzip'})
        response = urllib.request.urlopen(req, timeout=15)
        
        if response.info().get('Content-Encoding') == 'gzip':
            data = gzip.decompress(response.read()).decode('utf-8')
        else:
            data = response.read().decode('utf-8')
            
        jdata = json.loads(data)
        items = jdata.get("items", [])
        
        results = []
        for item in items[:3]:
            title = item.get("title", "")
            score = item.get("score", 0)
            body = item.get("body", "").replace('\n', ' ')
            results.append(f"Q: {title} [Score: {score}] - Body: {body[:500]}...")
            
        final_str = "\n".join(results)
        if not final_str: final_str = "Nenhum resultado encontrado no StackExchange."
        
        print(json.dumps({
            "status": "success",
            "source": "StackExchange API (StackOverflow)",
            "topic": topic,
            "data_compressed": final_str
        }))
    except Exception as e:
        print(json.dumps({"error": f"StackExchange API Error: {str(e)}"}))

def fetch_github(topic):
    try:
        url = f"https://api.github.com/search/code?q={urllib.parse.quote(topic)}&per_page=3"
        req = urllib.request.Request(url, headers={'User-Agent': 'Sovereign-Pair/1.0'})
        response = urllib.request.urlopen(req, timeout=15)
        data = json.loads(response.read().decode('utf-8'))
        
        items = data.get("items", [])
        results = []
        for item in items:
            repo = item.get("repository", {}).get("full_name", "")
            path = item.get("path", "")
            url_html = item.get("html_url", "")
            results.append(f"Repo: {repo} | Path: {path} | URL: {url_html}")
            
        final_str = "\n".join(results)
        if not final_str: final_str = "Nenhum resultado encontrado no Github."
        
        print(json.dumps({
            "status": "success",
            "source": "Github Search API",
            "topic": topic,
            "data_compressed": final_str
        }))
    except Exception as e:
        print(json.dumps({"error": f"Github API Error: {str(e)}"}))

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(json.dumps({"error": "Usage: engineering_matrix.py <topic> <source>"}))
        sys.exit(1)
        
    topic = sys.argv[1]
    source = sys.argv[2].lower()
    
    if "stack" in source or "overflow" in source:
        fetch_stackexchange(topic)
    elif "github" in source or "git" in source:
        fetch_github(topic)
    else:
        fetch_stackexchange(topic)
