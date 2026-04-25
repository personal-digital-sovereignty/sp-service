import requests
import json
import threading
import time
import sys

# Sovereign Pair Stream Benchmarker
# Valida estabilidade de streaming sob concorrência

API_URL = "http://127.0.0.1:38001/v1/chat/completions"
CONCURRENT_CLIENTS = 3
MAX_TOKENS = 100

def run_client(client_id):
    payload = {
        "model": "llama3.2:3b", # Garante que usa Ollama local
        "messages": [
            {"role": "user", "content": f"Client {client_id}: Escreva um parágrafo técnico sobre sistemas distribuídos e concorrência em SQLite."}
        ],
        "stream": True,
        "max_tokens": MAX_TOKENS
    }
    
    start_time = time.time()
    tokens = 0
    try:
        response = requests.post(API_URL, json=payload, stream=True, timeout=60)
        print(f"[{client_id}] Connection established. Status: {response.status_code}")
        
        for line in response.iter_lines():
            if line:
                line_str = line.decode('utf-8')
                if line_str.startswith("data: "):
                    data_content = line_str[6:]
                    if data_content == "[DONE]":
                        break
                    
                    try:
                        chunk = json.loads(data_content)
                        if 'choices' in chunk and len(chunk['choices']) > 0:
                            delta = chunk['choices'][0].get('delta', {})
                            if 'content' in delta:
                                tokens += 1
                                # print(f"[{client_id}] {delta['content']}", end="", flush=True)
                    except json.JSONDecodeError:
                        continue
                        
        elapsed = time.time() - start_time
        tps = tokens / elapsed if elapsed > 0 else 0
        print(f"\n✅ Client {client_id} DONE. Tokens: {tokens}, Time: {elapsed:.2f}s, TPS: {tps:.2f}")
        
    except Exception as e:
        print(f"❌ Client {client_id} FAILED: {e}")

if __name__ == "__main__":
    print(f"🚀 Starting Stream Benchmark with {CONCURRENT_CLIENTS} clients...")
    threads = []
    for i in range(CONCURRENT_CLIENTS):
        t = threading.Thread(target=run_client, args=(i,))
        threads.append(t)
        t.start()
        
    for t in threads:
        t.join()
    
    print("\n🏁 Benchmark Finalized.")
