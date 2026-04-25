import requests
import time

API_URL = "http://127.0.0.1:38001/v1/chat/completions"

# Teste de Payload Gigante (5MB de texto)
def test_giant_payload():
    print("🚀 Sending 5MB payload to Sovereign Core...")
    giant_text = "Esta é uma linha de teste para estresse de memória. " * 100000 
    payload = {
        "model": "llama3.2:3b",
        "messages": [
            {"role": "user", "content": giant_text}
        ],
        "stream": False,
        "max_tokens": 10
    }
    
    start = time.time()
    try:
        # Usamos timeout longo pois o parsing de 5MB de JSON pode levar tempo
        response = requests.post(API_URL, json=payload, timeout=30)
        elapsed = time.time() - start
        print(f"✅ Response received in {elapsed:.2f}s. Status: {response.status_code}")
        if response.status_code == 200:
            print("Successfully parsed giant payload.")
        else:
            print(f"Error: {response.text}")
    except Exception as e:
        print(f"❌ Failed: {e}")

if __name__ == "__main__":
    test_giant_payload()
