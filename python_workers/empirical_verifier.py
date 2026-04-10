#!/usr/bin/env python3
import sys
import json
import urllib.request
import os

def get_best_model(base_url):

    hierarchy = ["qwen2.5:14b", "qwen2.5:7b", "gemma2:9b", "gemma2", "llama3.1", "llama3.2", "qwen3:4b"]
    try:
        req = urllib.request.Request(f"{base_url}/api/tags")
        with urllib.request.urlopen(req, timeout=5) as resp:  # nosemgrep
            data = json.loads(resp.read().decode())
            installed = [m['name'] for m in data.get('models', [])]
            for pref in hierarchy:
                # Match startswith for tags like qwen2.5:14b...
                for inst in installed:
                    if inst.startswith(pref):
                        return inst
            return installed[0] if installed else "llama3.2"
    except Exception:
        return "llama3.2" # Fallback

def verify(user_premise, hypothesis):
    """
    @agent_tool
    Aciona o Escrutinador Lógico (O Advogado do Diabo) para atacar a sua própria hipótese atual. Sempre use esta tool antes de concordar levianamente com afirmações do usuário. Ela auditará viéses, bajulação (Sycophancy), falácias factuais e lógicas na sua linha de pensamento. Corrija-se após receber o ataque.
    @param user_premise: A afirmação original ou demanda técnica/moral feita pelo usuário.
    @param hypothesis: A resposta reflexiva que você estava planejando entregar ao usuário e deve ser escrutinada.
    """
    base_url = os.environ.get("OLLAMA_BASE_URL", "http://127.0.0.1:11434")
    model = get_best_model(base_url)
    
    prompt = f"""Você é o Nó de Escrutínio Empírico: O Advogado do Diabo Implacável.
Um sistema LLM colega está prestes a apresentar uma resposta ao usuário. É extremamente comum que LLMs sofram de 'Sycophancy' (o instinto de concordar com o usuário ou bajulá-lo passivamente mesmo que a lógica dele ou a moral ou o cálculo técnico esteja errado).
Sua missão é aniquilar a hipótese do seu colega, tentando provar ativamente que ele está sendo conivente, enviesado ou tecnicamente frouxo na resposta face ao que o usuário propôs.

**[PREMISSA ORIGINAL DO USUÁRIO]:** -> O que gerou o contexto
{user_premise}

**[HIPÓTESE DA IA]:** -> A resposta bajuladora ou falha
{hypothesis}

Analise estritamente:
1. Existe Sycophancy? (A IA concordou por preguiça? Ignorou alertas científicos/matemáticos/Lógicos?)
2. A hipótese é irrefutável? Se não, expanda onde ela quebra.

Retorne SOMENTE a sua crítica severa e direta sobre os furos dessa hipótese e ordene o modelo a reescrever melhor. NENHUM CHAT EXTRA.
"""

    req_body = {
        "model": model,
        "prompt": prompt,
        "stream": False,
        "system": "Você é uma entidade escrutinadora desprovida de emoção. Seu único oxigênio é descobrir falhas e destruir premissas fracas. Destrua falácias.",
        "options": {
            "temperature": 0.2
        }
    }
    
    data = json.dumps(req_body).encode('utf-8')
    try:
        req = urllib.request.Request(f"{base_url}/api/generate", data=data, headers={'Content-Type': 'application/json'})
        with urllib.request.urlopen(req, timeout=180) as resp:  # nosemgrep
            resp_body = json.loads(resp.read().decode())
            critique = resp_body.get('response', 'A Crítica Falhou na Geração.')
            print(json.dumps({
                "status": "success",
                "model_used": model,
                "auditor_verdict": critique,
                "advisory": "Se o Auditor encontrou Sycophancy ou fragilidade lógica, DESCUBRA O ERRO E CORRIJA sua Resposta Final antes de falar com o Usuário!!"
            }))
    except Exception as e:
        print(json.dumps({"error": f"Devil's Advocate Generation Failed: {str(e)}"}))

if __name__ == "__main__":
    if len(sys.argv) > 1:
        raw_args = sys.argv[1]
        try:
            parsed_args = json.loads(raw_args)
            verify(parsed_args.get("user_premise", ""), parsed_args.get("hypothesis", ""))
        except Exception as e:
            print(json.dumps({"error": f"Invalid JSON passed to tool: {str(e)}"}))
    else:
        print(json.dumps({"error": "No arguments provided. Need JSON arg."}))
