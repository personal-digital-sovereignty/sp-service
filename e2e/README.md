# sp-service E2E Tests (LOCAIS APENAS)

**⚠️ IMPORTANTE:** Estes testes **NÃO** rodam no CI/CD do GitHub.

**Motivo:** Testes E2E exigem o sp-service rodando em `localhost:8080`, o que requer:
- Docker ou binário compilado
- Ollama configurado (opcional)
- Python workers provisionados
- Tempo de startup (~30s)

**Uso:** Executar **apenas localmente** durante desenvolvimento.

---

## 📋 Pré-requisitos

1. **sp-service rodando:**
   ```bash
   cd ..
   cargo run --release
   # Aguardar: "Listening on 127.0.0.1:8080"
   ```

2. **Python dependencies:**
   ```bash
   pip install -r requirements.txt
   ```

---

## Test Structure

```
e2e/
├── tests/
│   ├── conftest.py              # Shared fixtures
│   ├── test_health.py           # Health check tests
│   ├── test_chat_api.py         # Chat API tests
│   ├── test_rag_pipeline.py     # RAG pipeline tests
│   ├── test_deep_research.py    # Deep research tests
│   └── test_tools.py            # Python workers tools tests
├── requirements.txt
└── README.md
```

---

## Test Categories

### Health Tests (test_health.py)
- `/health` endpoint
- `/ready` endpoint
- `/v1/telemetry` endpoint
- Service availability

### Chat Tests (test_chat_api.py)
- Basic chat (non-streaming)
- Streaming chat
- Multi-turn conversation
- Tool calling
- Session persistence

### RAG Tests (test_rag_pipeline.py)
- Document ingestion
- Vector search
- RAG chat
- Complete pipeline flow

### Deep Research Tests (test_deep_research.py)
- Basic deep research
- Workflow stages validation
- Macro economy queries
- Tool integration

### Tools Tests (test_tools.py)
- Financial ticker fetching
- Macroeconomic indicators
- Sub-researcher dispatch
- Multi-tool integration

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SP_SERVICE_URL` | http://localhost:8080 | sp-service API URL |

### Example

```bash
export SP_SERVICE_URL="http://localhost:8080"
pytest tests/ -v
```

---

## Running Specific Tests

### Run health tests only
```bash
pytest tests/test_health.py -v
```

### Run chat tests only
```bash
pytest tests/test_chat_api.py -v
```

### Run RAG tests only
```bash
pytest tests/test_rag_pipeline.py -v
```

### Run deep research tests only
```bash
pytest tests/test_deep_research.py -v
```

### Run tools tests only
```bash
pytest tests/test_tools.py -v
```

### Run with coverage
```bash
pytest tests/ --cov=. --cov-report=html
```

### Run single test
```bash
pytest tests/test_health.py::TestHealthEndpoints::test_health_endpoint -v
```

---

## API Endpoints Tested

| Endpoint | Method | Test File |
|----------|--------|-----------|
| `/health` | GET | test_health.py |
| `/ready` | GET | test_health.py |
| `/v1/telemetry` | GET | test_health.py |
| `/v1/chat/completions` | POST | test_chat_api.py |
| `/v1/rag/ingest` | POST | test_rag_pipeline.py |
| `/v1/rag/search` | GET | test_rag_pipeline.py |
| `/v1/rag/chat` | POST | test_rag_pipeline.py |
| `/v1/research/deep` | POST | test_deep_research.py |
| `/v1/tools/fetch_financial_ticker` | POST | test_tools.py |
| `/v1/tools/fetch_macroeconomy` | POST | test_tools.py |
| `/v1/tools/dispatch_sub_researcher` | POST | test_tools.py |

---

## Troubleshooting

### Service not available
```
Error: Connection refused at http://localhost:8080
```

**Solution:** Start sp-service before running tests:
```bash
cargo run --release
```

### Tests timing out
```
Error: Request timeout after 30s
```

**Solution:** Increase timeout in conftest.py or check if Ollama is running.

### Model not found
```
Error: Model 'qwen3:8b' not found
```

**Solution:** Pull the required model:
```bash
ollama pull qwen3:8b
```

---

## Expected Test Results

### Passing
```
============================= test session starts ==============================
collected 35 items

tests/test_health.py .......                                             [ 20%]
tests/test_chat_api.py ........                                          [ 42%]
tests/test_rag_pipeline.py ......                                        [ 60%]
tests/test_deep_research.py .......                                      [ 80%]
tests/test_tools.py .......                                              [100%]

========================= 35 passed in 120.5s ==============================
```

### With Failures
```
============================= test session starts ==============================
collected 35 items

tests/test_health.py .......                                             [ 20%]
tests/test_chat_api.py ...F....                                          [ 42%]
...

=================================== FAILURES ===================================
____________________ TestChatCompletions.test_chat_basic _____________________

self = <test_chat_api.TestChatCompletions object at 0x...>

    def test_chat_basic(self, base_url, session):
>       assert response.status_code == 200
E       assert 503 == 200

========================= 1 failed, 34 passed in 120.5s ========================
```

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'
      
      - name: Install dependencies
        run: |
          cd e2e
          pip install -r requirements.txt
      
      - name: Start sp-service
        run: |
          cargo build --release
          ./target/release/sp-service &
          sleep 10
      
      - name: Run E2E tests
        run: |
          cd e2e
          pytest tests/ -v --tb=short
```

---

## Contact

- Repository: https://github.com/Personal-Digital-Sovereignty/sp-service
- Email: jefersonlopes@proton.me
