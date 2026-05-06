# Changelog

All notable changes to `sp-service` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.4.0-dev] - 2026-05-06 — Estabilizacao Estrutural Critica

### Fixed — AST e Delimitadores (api.rs)
- **WAG Reranker lock scope:** O bloco `if let Ok(mut rlock) = RERANKER.lock()` nao fechava corretamente seu delimitador `}`, arrastando a logica subsequente para dentro do escopo do lock. Brace inserida na linha correta.
- **`futures_util::join_all` loop:** O iterador assincrono sobre `scrape_handles` nao possuia brace de fechamento, causando colapso da arvore sintatica (AST) a partir da linha ~916.
- **`tokio::spawn` SSE streaming leak:** Chave de fechamento `});` prematura no corpo do `res.bytes_stream().map(move |result| {...})` deixava o `tokio::spawn` principal do chat handler sem fechamento ate o fim do arquivo, corrompendo todo o escopo do Axum stream handler.

### Refactored — Let Chains (Rust 2024 -> Rust 2021)
- **`api.rs`:** `if use_openrouter && let Some(settings) = openrouter_settings` convertido para blocos `if` aninhados compativeis com Rust 2021 stable.
- **`research.rs`:** Removidas 10 instancias de `&& let` em:
  - `scrape_ghost_fallbacks`: pattern matching regex `__NEXT_DATA__`
  - `scrape_via_google_cache` / `scrape_via_archive_today`: checagens sequenciais HTTP + body text
  - Ghost Protocol race: 6 fallbacks (`arquivo_pt`, `ukwa`, `vefsafn`, `wayback`, `gcache`, `archive_ph`) refatorados de `if let Ok(md) = X && md.len() > 200` para blocos aninhados
  - `search_brave_api`: parse JSON `json_data.get("web").and_then(...)`
  - `search_searxng_public`: fetch de nodes customizados do banco SQLite (multi-chain `db_pool && query_scalar && from_str`)
  - `search_searxng_public`: parse de resultados da instancia SearXNG

### Fixed — CI/CD Pipeline
- Removidos caracteres nao-ASCII (emojis, acentuacoes em comentarios YAML) dos workflows `ci.yml`, `docker.yml`, `deploy-oci.yml`, `release_notes.yml` para conformidade com `actionlint`.
- Migrada logica de nomeacao de imagens Docker de `$GITHUB_ENV` para Step Outputs (`id: prep -> outputs.image_name`), seguindo melhores praticas do `actionlint`.
- Corrigida sanitizacao de tags GHCR (lowercase enforcement).

### Added — Documentacao
- `_strategy/CONSTRUCTION_LOG_COMPLETE.md`: Secao "Correcoes Estruturais Criticas (2026-05-06)" documentando toda a investigacao da AST e refatoracao final de Let Chains.

### Validated
- `cargo check`: Exit 0, 0 erros, 0 warnings fatais.
- `cargo test`: **136 testes Rust passando** (0 failed).
- `pytest`: **108 testes Python passando** (0 failed).
- `grep "&& let"`: **0 instancias** de Let Chains em toda a codebase.

---

## [1.4.0-alpha] - 2026-05-03 — Desacoplamento e Modularizacao

### Added
- Repositorio `sp-service` extraido do monolito `sovereign-pair` via `git subtree`.
- 5 workflows CI/CD configurados: `ci.yml`, `devsecops.yml`, `docker.yml`, `deploy-oci.yml`, `release_notes.yml`.
- Branch protection ativada na `main` com required status checks.
- 32 testes Python criados para workers de dados.
- 59 testes E2E criados (execucao local apenas).
- Docker multi-stage build + `docker-compose.yml`.
- OpenAPI 3.0 spec (`docs/api/`).

### Changed
- `Cargo.toml`: versao atualizada para `1.4.0-dev`, edition `2021`.
- `src/api.rs`: Triagem agêntica refatorada (Trivial / Web / Sys) para suporte a `is_trivial`, `is_web`, `is_sys`.
- Todos os 138 erros de Let Chains (Rust 2024) refatorados para Rust 2021 nested-ifs (14 arquivos).

### Fixed
- Suporte dinamico a modelo via `discover_best_model_from_matrix` (eliminacao de hardcodes).
- SecOps Vault: override de API keys de providers (NVIDIA, Qwen, OpenRouter) via vault criptografado.
- SSH Mesh Connector: Master Resolver para conectividade OCI/Oracle Cloud.

---

## [1.3.2] - 2026-04-29 — Dynamic Model Discovery

### Added
- Helper `discover_best_model_from_matrix` para descoberta dinamica de modelos via SQLite `model_capabilities`.
- Sincronizacao de versao `v1.3.2` em todos os arquivos de configuracao.

### Fixed
- Eliminacao de referencias hardcoded a modelos em `api.rs`, `api_trainer.rs`, `sync_engine.rs`, `auto_evaluator.rs`.
- Pipeline CI/CD para macOS x64 validado.

---

## [1.3.0] - 2026-04-23 — Sovereign RAG & Resilience Shield

### Added
- RAG Pipeline com geracao de tabelas Markdown e indicadores de fonte.
- OOM Guard / Hardware Telemetry (monitoramento VRAM cross-platform: Linux sysfs, nvidia-smi, Apple Silicon).
- Health Gate: `health_check_apis.py` no startup + a cada 4h, persiste em `api_health_log`, expoe `GET /v1/analytics/api_health`.

### Fixed
- 6 gaps auditados no Resilience Shield (GAP-RS-01 a RS-06): 4 corrigidos, 1 deferido, 1 confirmado nao-gap.

---

[1.4.0-dev]: https://github.com/Personal-Digital-Sovereignty/sp-service/compare/v1.4.0-alpha...HEAD
[1.4.0-alpha]: https://github.com/Personal-Digital-Sovereignty/sp-service/compare/v1.3.2...v1.4.0-alpha
[1.3.2]: https://github.com/Personal-Digital-Sovereignty/sp-service/compare/v1.3.0...v1.3.2
[1.3.0]: https://github.com/Personal-Digital-Sovereignty/sp-service/releases/tag/v1.3.0
