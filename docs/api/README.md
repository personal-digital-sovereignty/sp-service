# sp-service — API Documentation

**Base URL:** `http://localhost:8080`  
**Version:** 1.4.0-dev  
**Protocol:** HTTP/1.1, SSE (Server-Sent Events)

---

## 📋 Índice

### Core Endpoints
- [`GET /health`](#get-health) — Health check
- [`GET /ready`](#get-ready) — Readiness probe
- [`GET /v1/telemetry`](#get-v1telemetry) — Hardware telemetry

### Chat & Inference
- [`POST /v1/chat/completions`](#post-v1chatcompletions) — Chat with streaming
- [`POST /v1/research/deep`](#post-v1researchdeep) — Deep research (web-augmented)

### RAG Pipeline
- [`POST /v1/rag/ingest`](#post-v1ragingest) — Upload documents
- [`GET /v1/rag/search`](#get-v1ragsearch) — Vector search
- [`POST /v1/rag/chat`](#post-v1ragchat) — RAG-powered chat

### Tools & Workers
- [`POST /v1/tools/fetch_financial_ticker`](#post-v1toolsfetch_financial_ticker) — Financial data
- [`POST /v1/tools/fetch_macroeconomy`](#post-v1toolsfetch_macroeconomy) — Macro indicators
- [`POST /v1/tools/dispatch_sub_researcher`](#post-v1toolsdispatch_sub_researcher) — Web research

### Settings & Configuration
- [`GET /v1/settings`](#get-v1settings) — Get settings
- [`PUT /v1/settings`](#put-v1settings) — Update settings
- [`POST /v1/settings/kms`](#post-v1settingskms) — Configure KMS

### Projects & Tasks
- [`GET /v1/projects`](#get-v1projects) — List projects
- [`POST /v1/projects`](#post-v1projects) — Create project
- [`GET /v1/projects/{id}/tasks`](#get-v1projectsidtasks) — List tasks

### Network & Mesh
- [`GET /v1/network/pair`](#get-v1networkpair) — P2P pairing token
- [`POST /v1/network/connect`](#post-v1networkconnect) — Connect to node

---

## 🔌 Endpoint Details

### `GET /health`

Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "version": "1.4.0-dev",
  "timestamp": "2026-05-03T10:00:00Z"
}
```

**Status Codes:**
- `200 OK` — Service is healthy
- `503 Service Unavailable` — Service is unhealthy

---

### `GET /ready`

Readiness probe (checks database, models, workers).

**Response:**
```json
{
  "ready": true,
  "checks": {
    "database": "ok",
    "ollama": "ok",
    "python_workers": "ok"
  }
}
```

---

### `GET /v1/telemetry`

Hardware telemetry (VRAM, RAM, CPU, GPU).

**Response:**
```json
{
  "ram": {
    "used_mb": 8192,
    "total_mb": 32768,
    "percent": 25.0
  },
  "vram": {
    "used_mb": 6144,
    "total_mb": 16384,
    "percent": 37.5
  },
  "cpu": {
    "percent": 12.5,
    "cores": 8
  },
  "gpu": {
    "name": "NVIDIA GeForce RTX 4070",
    "load_percent": 45.0
  }
}
```

---

### `POST /v1/chat/completions`

Chat with streaming SSE support.

**Request:**
```json
{
  "model": "qwen3:8b",
  "messages": [
    {"role": "user", "content": "Olá"}
  ],
  "stream": true,
  "workspace_id": "default",
  "session_id": null,
  "deep_research": false,
  "rewoo_enabled": false
}
```

**Response (SSE Stream):**
```
data: {"id":"chat-123","object":"chat.completion.chunk","choices":[{"delta":{"content":"Olá"}}]}

data: {"id":"chat-123","object":"chat.completion.chunk","choices":[{"delta":{"content":"! Como"}}]}

data: [DONE]
```

**Status Codes:**
- `200 OK` — Success
- `400 Bad Request` — Invalid payload
- `503 Service Unavailable` — Model not available

---

### `POST /v1/research/deep`

Deep research (web-augmented generation).

**Request:**
```json
{
  "query": "Análise macroeconômica Brasil 2025",
  "agentic_mode": true,
  "max_iterations": 10,
  "tools": ["financial", "macro", "academic"]
}
```

**Response (SSE Stream):**
```
data: {"stage":"planning","message":"Breaking down query..."}

data: {"stage":"tool_call","tool":"fetch_macroeconomy","params":{"indicators":["IPCA","SELIC"]}}

data: {"stage":"synthesis","message":"Analyzing data..."}

data: {"stage":"complete","artifact_id":"research-456"}
```

---

### `POST /v1/rag/ingest`

Upload documents to RAG pipeline.

**Request:** `multipart/form-data`

| Field | Type | Description |
|-------|------|-------------|
| `file` | file | Document (.md, .pdf, .txt, .docx) |
| `project_id` | string | Project identifier |
| `tags` | string | Comma-separated tags |

**Response:**
```json
{
  "id": "doc-789",
  "filename": "relatorio.pdf",
  "chunks_created": 45,
  "status": "processed"
}
```

---

### `GET /v1/rag/search`

Vector search.

**Query Parameters:**
- `q` (required): Search query
- `limit` (optional): Max results (default: 5)
- `threshold` (optional): Similarity threshold (default: 0.7)

**Response:**
```json
{
  "results": [
    {
      "document_id": "doc-789",
      "content": "O IPCA acumulado em 12 meses foi de 4.5%...",
      "score": 0.92,
      "metadata": {
        "source": "relatorio.pdf",
        "page": 3
      }
    }
  ]
}
```

---

### `POST /v1/rag/chat`

RAG-powered chat.

**Request:**
```json
{
  "query": "Qual a inflação acumulada?",
  "use_rag": true,
  "top_k": 5
}
```

**Response:**
```json
{
  "answer": "O IPCA acumulado em 12 meses foi de 4.5%...",
  "sources": [
    {
      "document_id": "doc-789",
      "relevance": 0.92
    }
  ]
}
```

---

### `POST /v1/tools/fetch_financial_ticker`

Fetch financial data.

**Request:**
```json
{
  "ticker": "PETR4.SA",
  "metrics": ["close", "volume", "market_cap"],
  "period": "12M"
}
```

**Response:**
```json
{
  "ticker": "PETR4.SA",
  "name": "Petróleo Brasileiro S.A.",
  "data": [
    {"date": "2026-04-30", "close": 38.50, "volume": 45000000}
  ],
  "currency": "BRL"
}
```

---

### `POST /v1/tools/fetch_macroeconomy`

Fetch macroeconomic indicators.

**Request:**
```json
{
  "indicators": ["IPCA", "SELIC", "IGPM"],
  "period": "12M"
}
```

**Response:**
```json
{
  "indicators": [
    {
      "name": "IPCA",
      "value": 4.5,
      "unit": "%",
      "date": "2026-04-30"
    },
    {
      "name": "SELIC",
      "value": 13.75,
      "unit": "% a.a.",
      "date": "2026-05-01"
    }
  ]
}
```

---

### `POST /v1/tools/dispatch_sub_researcher`

Web research tool.

**Request:**
```json
{
  "query": "O que é o programa PROSUB da Marinha?",
  "max_results": 10
}
```

**Response:**
```json
{
  "results": [
    {
      "title": "Marinha lança novo edital do PROSUB",
      "url": "https://example.com/noticia",
      "snippet": "O Programa de Desenvolvimento de Submarinos..."
    }
  ]
}
```

---

### `GET /v1/settings`

Get current settings.

**Response:**
```json
{
  "ollama_host": "127.0.0.1:11434",
  "default_model": "qwen3:8b",
  "rag_top_k": 5,
  "body_limit_bytes": 52428800,
  "providers": {
    "qwen": {"configured": true},
    "nvidia": {"configured": false},
    "openrouter": {"configured": true}
  }
}
```

---

### `PUT /v1/settings`

Update settings.

**Request:**
```json
{
  "ollama_host": "127.0.0.1:11434",
  "default_model": "phi4:14b",
  "rag_top_k": 10
}
```

**Response:**
```json
{
  "status": "updated",
  "message": "Settings saved successfully"
}
```

---

### `POST /v1/settings/kms`

Configure KMS with provider API key.

**Request:**
```json
{
  "provider": "qwen",
  "api_key": "sk-...",
  "endpoint": "https://dashscope.aliyuncs.com"
}
```

**Response:**
```json
{
  "status": "configured",
  "provider": "qwen",
  "encrypted": true
}
```

---

### `GET /v1/projects`

List projects.

**Query Parameters:**
- `limit` (optional): Max results
- `offset` (optional): Pagination offset

**Response:**
```json
{
  "projects": [
    {
      "id": "proj-001",
      "title": "Análise Macro 2025",
      "created_at": "2026-04-01T00:00:00Z",
      "task_count": 12
    }
  ],
  "total": 1
}
```

---

### `POST /v1/projects`

Create project.

**Request:**
```json
{
  "title": "Análise Macro 2025",
  "description": "Estudo macroeconômico completo"
}
```

**Response:**
```json
{
  "id": "proj-001",
  "title": "Análise Macro 2025",
  "created_at": "2026-05-03T10:00:00Z"
}
```

---

### `GET /v1/projects/{id}/tasks`

List tasks for a project.

**Response:**
```json
{
  "tasks": [
    {
      "id": "task-001",
      "title": "Coletar IPCA",
      "status": "completed",
      "completed_at": "2026-05-01T00:00:00Z"
    }
  ]
}
```

---

### `GET /v1/network/pair`

Get P2P pairing token (loopback only).

**Response:**
```json
{
  "alias": "sovereign-node-01",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2026-05-03T11:00:00Z"
}
```

**Note:** Token is only returned for loopback requests (127.0.0.1). LAN requests receive alias only.

---

### `POST /v1/network/connect`

Connect to remote node.

**Request:**
```json
{
  "host": "192.168.1.100",
  "port": 8080,
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Response:**
```json
{
  "status": "connected",
  "remote_alias": "sovereign-node-02"
}
```

---

## 🔐 Authentication

Most endpoints do not require authentication for local loopback (127.0.0.1).

For LAN/P2P connections:
- Use `/v1/network/pair` to get a JWT token
- Include token in `Authorization: Bearer <token>` header

## 📦 Rate Limiting

| Endpoint | Limit |
|----------|-------|
| `/health`, `/ready` | No limit |
| `/v1/chat/completions` | 100 req/min |
| `/v1/research/deep` | 10 req/min |
| `/v1/tools/*` | 60 req/min |

## 🚨 Error Codes

| Code | Description |
|------|-------------|
| `400 Bad Request` | Invalid payload or parameters |
| `401 Unauthorized` | Missing or invalid token |
| `404 Not Found` | Resource not found |
| `413 Payload Too Large` | Body exceeds limit |
| `503 Service Unavailable` | Model or service unavailable |

---

**Generated:** 2026-05-03  
**Contact:** jefersonlopes@proton.me
