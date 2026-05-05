# sp-service 🦀

**Service/API/Backend** — O motor de Inferência e RAG Cíbrido para o ecossistema Sovereign Pair.

[![Version](https://img.shields.io/badge/version-1.4.0--dev-blue.svg)](https://github.com/Personal-Digital-Sovereignty/sp-platform)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-PolyForm--Noncommercial-red.svg)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/Personal-Digital-Sovereignty/sp-platform/ci.yml?branch=main)](https://github.com/Personal-Digital-Sovereignty/sp-platform/actions)
[![E2E Pipeline](https://img.shields.io/github/actions/workflow/status/Personal-Digital-Sovereignty/sp-platform/e2e.yml?label=E2E%20Tests)](https://github.com/Personal-Digital-Sovereignty/sp-platform/actions/workflows/e2e.yml)
[![Docker Build](https://img.shields.io/github/actions/workflow/status/Personal-Digital-Sovereignty/sp-platform/docker.yml?label=Docker%20Build)](https://github.com/Personal-Digital-Sovereignty/sp-platform/actions/workflows/docker.yml)

---

## 📋 Visão Geral

**sp-service** é o **novo core backend** do ecossistema Sovereign Pair, focado exclusivamente em:

- ✅ **API REST/GraphQL** de alta performance (Axum + Tokio)
- ✅ **RAG Pipeline** com Tool Calling nativo
- ✅ **Deep Research** com análise de correlação (Pandas)
- ✅ **Multi-Provider LLM** (Ollama, Qwen, NVIDIA, OpenRouter)
- ✅ **Sensus Sync Engine** (Dual-Truth: SQLite + Markdown)
- ✅ **Zero-Trust Security** (KMS, SecOps Vault, SSH Mesh)

### 🏗️ Arquitetura Desacoplada

Este repositório é **backend-only**. Frontends vivem em repositórios separados:

```
sp-platform/
├── sp-service/          ← ESTE REPOSITÓRIO (Backend Rust + Python)
├── sovereign-pair/      ← Legado (consulta histórica)
├── sp-ui-chat/          ← Frontend Chat (futuro)
├── sp-ui-rag/           ← Frontend RAG Pipeline (futuro)
├── sp-ui-coding/        ← Frontend Coder Ecosystem (futuro)
├── sp-ui-projects/      ← Frontend Projects (futuro)
└── sp-ui-vault/         ← Frontend Vault Explorer (futuro)
```

** Migração:** Este repositório substitui `sovereign-pair/` para todo desenvolvimento backend ativo. Consulte [`_strategy/MIGRACAO_SOVEREIGN_PAIR_PARA_SP_SERVICE.md`](../_strategy/MIGRACAO_SOVEREIGN_PAIR_PARA_SP_SERVICE.md) para detalhes.

---

## 🚀 Quick Start

### Pré-requisitos

- **Rust 1.75+** ([instalação](https://www.rust-lang.org/tools/install))
- **Python 3.11+** (para workers de dados)
- **Ollama** (opcional, para inferência local)
- **SQLite** (compilado com `sqlite-vec` support)

### Instalação Rápida

```bash
# 1. Clone o repositório
git clone https://github.com/Personal-Digital-Sovereignty/sp-platform.git
cd sp-platform/sp-service

# 2. Build de desenvolvimento
cargo build --dev

# 3. Rodar o servidor
cargo run

# 4. Build de produção (binário otimizado)
cargo build --release
./target/release/sp-service
```

### Docker (Em Implementação)

```bash
# Build da imagem
docker build -t sp-service:latest .

# Rodar container
docker run -d \
  -p 8080:8080 \
  -v ./data:/app/data \
  -v ./models:/app/models \
  --name sp-service \
  sp-service:latest
```

---

## 🔧 Configuração

### Variáveis de Ambiente

Crie um arquivo `.env` na raiz:

```bash
# Ollama Local
OLLAMA_HOST=127.0.0.1:11434

# Cloud Providers (opcionais)
QWEN_API_KEY=sk-...
NVIDIA_API_KEY=nvapi-...
OPENROUTER_API_KEY=sk-or-...

# KMS Master Key (gerada automaticamente se ausente)
KMS_MASTER_KEY=...

# Database Path
DATABASE_PATH=./data/sensus_nexus.db

# Workspace (Markdown Vault)
WORKSPACE_PATH=./data/vault
```

### Primeiros Passos

1. **Inicializar Database:**
   ```bash
   ./sp-service --init-db
   ```

2. **Configurar API Keys:**
   ```bash
   curl -X POST http://localhost:8080/v1/settings/kms \
     -H "Content-Type: application/json" \
     -d '{"provider": "qwen", "api_key": "sk-..."}'
   ```

3. **Health Check:**
   ```bash
   curl http://localhost:8080/health
   # {"status": "healthy", "version": "1.4.0-dev"}
   ```

---

## 📚 Documentação

### Guias Principais

- **[Guia de Instalação](docs/install_guide.md)** — Instalação detalhada por plataforma
- **[API Reference](docs/api/)** — OpenAPI/Swagger specs
- **[RAG Mechanics](docs/rag_mechanics.md)** — Como funciona o RAG pipeline
- **[Security](docs/security.md)** — KMS, SecOps Vault, Zero-Trust

### Estratégia e Arquitetura

- **[BLUEPRINT.md](docs/engineering/BLUEPRINT.md)** — Manifesto técnico completo
- **[MIGRACAO.md](../_strategy/MIGRACAO_SOVEREIGN_PAIR_PARA_SP_SERVICE.md)** — Guia de migração do legado
- **[ROADMAP.md](ROADMAP.md)** — Próximos passos e épicos

---

## 🧠 Componentes Principais

### Backend Rust (Core)

| Módulo | Descrição |
|--------|-----------|
| `api.rs` | Hub agêntico e roteamento de requests |
| `api_rag.rs` | Pipeline RAG e vector search |
| `api_trainer.rs` | Sandbox de Python Workers |
| `sync_engine.rs` | Sensus Dual-Truth Sync |
| `kms.rs` | Key Management Service (AES-256-GCM) |
| `memory_manager.rs` | Sovereign Swap (VRAM GC) |
| `ssh_mesh_connector.rs` | Oracle Cloud Mesh Tunneling |

### Python Workers

| Worker | Descrição |
|--------|-----------|
| `sovereign_matrix.py` | Buscador financeiro e macroeconomia |
| `analyze_and_join_time_series.py` | Pandas Joiner + Pearson Correlation |
| `academic_matrix.py` | PubMed, NASA, arXiv extraction |
| `culture_matrix.py` | TMDb, IGDB, MusicBrainz |
| `empirical_verifier.py` | Advogado do Diabo (anti-alucinação) |

---

## 🔌 API Endpoints (Principais)

### Chat & Inference

```bash
# Chat com streaming SSE
POST /v1/chat/completions
Content-Type: application/json

{
  "model": "qwen3:8b",
  "messages": [{"role": "user", "content": "Olá"}],
  "stream": true
}

# Deep Research (Web-Augmented)
POST /v1/research/deep
Content-Type: application/json

{
  "query": "Análise macroeconômica Brasil 2025",
  "agentic_mode": true
}
```

### RAG Pipeline

```bash
# Upload de documentos
POST /v1/rag/ingest
Content-Type: multipart/form-data

# Vector search
GET /v1/rag/search?q=inflação+IPCA&limit=5

# RAG chat
POST /v1/rag/chat
Content-Type: application/json

{
  "query": "Qual a inflação acumulada?",
  "use_rag": true
}
```

### Tools & Workers

```bash
# Executar tool financeira
POST /v1/tools/fetch_financial_ticker
Content-Type: application/json

{
  "ticker": "PETR4.SA",
  "metrics": ["close", "volume"]
}

# Executar tool macroeconômica
POST /v1/tools/fetch_macroeconomy
Content-Type: application/json

{
  "indicators": ["IPCA", "SELIC"],
  "period": "12M"
}
```

### System & Telemetry

```bash
# Health check
GET /health

# Telemetria (VRAM, RAM, CPU)
GET /v1/telemetry

# Modelos instalados (Ollama)
GET /v1/models/local

# Configurações
GET /v1/settings
PUT /v1/settings
```

---

## 🛡️ Security

### Zero-Trust Architecture

- **KMS Local:** Chaves mestras em `OnceLock` com `zeroize()`
- **SecOps Vault:** Credenciais criptografadas (AES-256-GCM)
- **SSH Mesh:** Túneis reversos sem portas expostas
- **Body Limits:** 50 MB global, 5 MB por endpoint crítico
- **JWT HS256:** Algoritmo fixo, sem `none` ou `RS256`
- **DOMPurify:** XSS prevention em todo conteúdo LLM

### Security Audit

Auditado em **Pass 3** (v1.2.9):
- ✅ JWT Algorithm Confusion (P3-02)
- ✅ Token Exposure LAN (P3-01)
- ✅ XSS via LLM Content (P3-04)
- ✅ DoS Body Limit (P3-03, P3-05)
- ✅ Command Injection SSH (CWE-78)

---

## 🧪 Testes

```bash
# Testes unitários
cargo test --lib

# Testes de integração
cargo test --test integration

# Testes de segurança
cargo test security

# Coverage (requer cargo-tarpaulin)
cargo tarpaulin --out Html
```

---

## 📦 Deploy

### Binário Standalone

```bash
# Linux
./target/release/sp-service

# macOS
./target/release/sp-service

# Windows
.\target\release\sp-service.exe
```

### Systemd Service (Linux)

```ini
[Unit]
Description=Sovereign Pair Service
After=network.target

[Service]
Type=simple
User=sovereign
WorkingDirectory=/opt/sp-service
ExecStart=/opt/sp-service/sp-service
Restart=always

[Install]
WantedBy=multi-user.target
```

### Kubernetes (Em Implementação)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sp-service
spec:
  replicas: 3
  selector:
    matchLabels:
      app: sp-service
  template:
    spec:
      containers:
      - name: sp-service
        image: sp-service:latest
        ports:
        - containerPort: 8080
```

---

## 🤝 Contribuindo

### Como Contribuir

1. **Fork** o repositório
2. **Crie uma branch** para sua feature (`git checkout -b feature/amazing-feature`)
3. **Commit** suas mudanças (`git commit -m 'Add amazing feature'`)
4. **Push** para a branch (`git push origin feature/amazing-feature`)
5. **Pull Request** no GitHub

### Padrões de Código

- **Rust:** `rustfmt` (auto-format), `clippy` (lints)
- **Python:** `black` (format), `flake8` (lints)
- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/)

### Code Review

Todo PR requer:
- ✅ Builds passing (Linux, macOS, Windows)
- ✅ Testes passando
- ✅ Zero warnings do Clippy
- ✅ Documentação atualizada

---

## 📄 Licenciamento

Este projeto está sob licença **PolyForm Noncommercial 1.0.0**. Veja [LICENSE](LICENSE) para detalhes.

**Uso Comercial:** Restrito. Para implantações empresariais, contate: [personal-digitalsovereignty@proton.me]

---

## 📞 Contato

- **Repositório:** [GitHub](https://github.com/Personal-Digital-Sovereignty/sp-platform)
- **Issues:** [GitHub Issues](https://github.com/Personal-Digital-Sovereignty/sp-platform/issues)
- **Email:** [personal-digitalsovereignty@proton.me]

---

**Histórico:** Este repositório (`sp-service/`) é o sucessor de `sovereign-pair/` para desenvolvimento backend ativo.  
**Versão Atual:** 1.4.0-dev  
**Última Atualização:** 2026-05-03
