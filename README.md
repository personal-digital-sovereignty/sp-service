# sp-service

**Sovereign Pair — Backend Daemon**

The core inference engine and REST API for the Sovereign OS ecosystem. Built with Rust, Axum, and Tokio for high-throughput, low-latency AI orchestration.

[![Version](https://img.shields.io/badge/version-1.6.0-blue.svg)](https://github.com/Personal-Digital-Sovereignty/sp-service)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-PolyForm--Noncommercial-red.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-283%20passing-brightgreen.svg)]()

---

## Overview

`sp-service` is the standalone backend daemon for the Sovereign OS platform. It acts as a local AI proxy, routing inference requests to local Ollama models or cloud providers, while maintaining a SQLite-backed persistence layer for sessions, vault documents, and system configuration.

The service exposes a superset of the OpenAI Chat Completions API, making it compatible with any OpenAI-compatible client.

### Core Capabilities

- OpenAI-compatible REST API (POST /v1/chat/completions) and WebSocket streaming (/v1/chat/ws)
- Multi-provider LLM routing: Ollama (local), OpenRouter, NVIDIA, Qwen (DashScope)
- Retrieval-Augmented Generation (RAG) with FastEmbed vector search
- Deep Research with multi-hop web scraping and semantic reranking
- ReWOO (Reasoning Without Observation) agentic planning loop
- MLA (Multi-Latent Attention) context compression for long conversations
- Sensus Sync Engine: dual-truth persistence via SQLite and Markdown vault
- Sovereign KMS: AES-256-GCM encryption for all stored credentials
- WebSocket PTY terminal via portable-pty
- OpenAPI specification at /api-docs/openapi.json with Swagger UI at /swagger-ui

---

## Architecture

```
sp-platform/
├── sp-service/          <- This repository (Rust backend daemon)
├── sp-ui-shell/         <- Desktop host (SvelteKit + Tauri)
├── sp-ui-core/          <- Shared state and components
├── sp-ui-chat/          <- Chat micro-frontend
├── sp-ui-vault/         <- Vault explorer micro-frontend
├── sp-ui-projects/      <- Kanban projects micro-frontend
├── sp-ui-rag/           <- RAG pipeline micro-frontend
└── sp-ui-coding/        <- Coding studio micro-frontend
```

The daemon binds to `127.0.0.1:38001` by default with automatic port escalation (38001–38010) to avoid collisions in desktop environments running multiple instances.

---

## Requirements

- Rust 1.75 or newer
- Ollama (optional, for local LLM inference)
- SQLite (bundled via sqlx — no system install required)

---

## Quick Start

```bash
# Clone the monorepo
git clone https://github.com/Personal-Digital-Sovereignty/sp-platform.git
cd sp-platform/sp-service

# Development build and run
cargo run

# Production build
cargo build --release
./target/release/sovereign-daemon
```

---

## Configuration

Create a `.env` file in the `sp-service` directory:

```env
# Local inference
OLLAMA_BASE_URL=http://127.0.0.1:11434

# CORS — comma-separated list of allowed frontend origins
ALLOWED_ORIGINS=http://localhost:5173,http://localhost:1420

# Vault path (Markdown document storage)
VAULT_PATH=/home/user/.sovereign/vault

# Database path
DATABASE_URL=sqlite:/home/user/.sovereign/sensus_nexus.db

# Runtime environment (native or docker)
SOVEREIGN_RUN_ENV=native

# Cloud providers (optional — can also be set via the UI SecOps Vault)
OPENROUTER_API_KEY=sk-or-...
NVIDIA_API_KEY=nvapi-...
```

---

## API Reference

The full interactive API reference is available at `http://localhost:38001/swagger-ui` when the daemon is running.

### Chat Completions (HTTP SSE)

```http
POST /v1/chat/completions
Content-Type: application/json
Authorization: Bearer <token>

{
  "model": "qwen3:8b",
  "messages": [{"role": "user", "content": "Hello"}],
  "stream": true,
  "workspace_id": "default",
  "deep_research": false,
  "rewoo_enabled": false
}
```

### Chat Completions (WebSocket)

Connect to `ws://localhost:38001/v1/chat/ws` and send the same JSON payload as a text frame. The server streams back OpenAI-compatible chunk frames and closes the connection with a final `{"done": true, "id": "session_<id>"}` frame.

### Key Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | /v1/chat/completions | Streaming chat inference (SSE) |
| GET | /v1/chat/ws | Bidirectional chat streaming (WebSocket) |
| GET | /v1/sessions | List chat sessions |
| GET | /v1/sessions/:id | Load session history |
| GET/POST | /v1/vault/fs | Vault file system operations |
| GET | /v1/rag/graph | Cognitive graph nodes and edges |
| GET | /v1/models | Aggregated model list (Ollama + OpenRouter) |
| GET | /v1/analytics/api_health | Hardware telemetry and OOM guard status |
| GET | /v1/coding/workspace | Real workspace file tree |
| GET | /v1/coding/terminal/ws | PTY terminal WebSocket |
| GET | /swagger-ui | Interactive Swagger UI |
| GET | /api-docs/openapi.json | OpenAPI 3.0 specification |

---

## Security Model

- **Zero-Trust LAN Guard:** Every request passes through `lan_auth_guard` before reaching handlers.
- **JWT Authentication:** HS256 algorithm enforced; `none` and asymmetric algorithms rejected.
- **Media Authentication:** Vault media files accept JWT via `Authorization` header, `?token=` query parameter, or `sovereign_token` cookie.
- **KMS Encryption:** All API keys stored in `secops_vault` are encrypted with AES-256-GCM. Keys are decrypted in-memory only at request time and zeroed after use.
- **CORS Hardening:** Origins loaded dynamically from `ALLOWED_ORIGINS` environment variable; no wildcard policy in production.
- **Body Limit:** 50 MB global request body limit to prevent DoS.
- **SSRF Guard:** Outbound HTTP requests validated against private IP ranges (RFC 1918, loopback, link-local).
- **Path Traversal:** File system endpoints validate all paths against the configured workspace root.

---

## Testing

```bash
# Run all 283 tests
cargo test -p sp-service

# Run security tests only
cargo test security

# Run with output
cargo test -- --nocapture
```

Test coverage spans: JWT security, KMS encrypt/decrypt roundtrips, path traversal guards, SSRF guards, body limits, SQLite WAL contention stress tests, telemetry, ReWOO planning, sync engine debounce, sandbox execution, and quantization utilities.

---

## Systemd Service (Linux)

```ini
[Unit]
Description=Sovereign OS Backend Daemon
After=network.target

[Service]
Type=simple
WorkingDirectory=/opt/sovereign
ExecStart=/opt/sovereign/sovereign-daemon
EnvironmentFile=/opt/sovereign/.env
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

---

## License

PolyForm Noncommercial 1.0.0. See [LICENSE](LICENSE).  
For commercial deployments, contact: personal-digitalsovereignty@proton.me

---

**Version:** 1.6.0  
**Last updated:** 2026-05-24  
**Test status:** 283 passing, 0 failed
