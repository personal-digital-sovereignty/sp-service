#!/usr/bin/env python3
import sys
import json
import urllib.request
import urllib.parse
import xml.etree.ElementTree as ET

def fetch_arxiv(query):
    try:
        url = f"http://export.arxiv.org/api/query?search_query=all:{urllib.parse.quote(query)}&start=0&max_results=3"
        req = urllib.request.Request(url, headers={'User-Agent': 'Sovereign-Pair'})
        response = urllib.request.urlopen(req, timeout=15)
        data = response.read().decode('utf-8')
        
        root = ET.fromstring(data)
        namespace = {'atom': 'http://www.w3.org/2005/Atom'}
        entries = root.findall('atom:entry', namespace)
        
        results = []
        for entry in entries:
            title = entry.find('atom:title', namespace).text.strip().replace('\n', ' ')
            summary = entry.find('atom:summary', namespace).text.strip().replace('\n', ' ')
            results.append(f"Title: {title}\nAbstract: {summary[:800]}...\n")
            
        final_str = "\n".join(results)
        if not final_str: final_str = "Nenhum resultado encontrado no arXiv."
        
        print(json.dumps({
            "status": "success",
            "source": "arXiv API",
            "query": query,
            "data_compressed": final_str
        }))
    except Exception as e:
        print(json.dumps({"error": f"arXiv API Error: {str(e)}"}))

def fetch_pubmed(query):
    try:
        url = f"https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term={urllib.parse.quote(query)}&retmode=json&retmax=3"
        req = urllib.request.Request(url, headers={'User-Agent': 'Sovereign-Pair'})
        response = urllib.request.urlopen(req, timeout=15)
        data = json.loads(response.read().decode('utf-8'))
        
        id_list = data.get("esearchresult", {}).get("idlist", [])
        if not id_list:
            print(json.dumps({"status": "success", "source": "PubMed API", "data_compressed": "Nenhum resultado encontrado no PubMed."}))
            return
            
        ids = ",".join(id_list)
        sum_url = f"https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id={ids}&retmode=json"
        s_req = urllib.request.Request(sum_url, headers={'User-Agent': 'Sovereign-Pair'})
        s_res = urllib.request.urlopen(s_req, timeout=15)
        s_data = json.loads(s_res.read().decode('utf-8'))
        
        results = []
        for uid in id_list:
            item = s_data.get("result", {}).get(uid, {})
            title = item.get("title", "")
            results.append(f"PubMed Paper: {title} [ID: {uid}]")
            
        print(json.dumps({
            "status": "success",
            "source": "PubMed API",
            "query": query,
            "data_compressed": "\n".join(results)
        }))
    except Exception as e:
        print(json.dumps({"error": f"PubMed API Error: {str(e)}"}))

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(json.dumps({"error": "Usage: academic_matrix.py <query> <discipline>"}))
        sys.exit(1)
        
    query = sys.argv[1]
    discipline = sys.argv[2].lower()
    
    if "arxiv" in discipline or "physics" in discipline or "math" in discipline or "compute" in discipline:
        fetch_arxiv(query)
    elif "pubmed" in discipline or "bio" in discipline or "med" in discipline:
        fetch_pubmed(query)
    else:
        fetch_arxiv(query)
