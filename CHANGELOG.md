# Changelog

All notable changes to `sp-service` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.7.0] - 2026-08-06 — Sovereign Fast-Router, Cold-Boot Guard & Empacotamento Nativo

*Tag `v1.5.0-rc.1` cortada em 2026-08-02 (`fa9677d`) sobre este mesmo trabalho, depois renomeada para `v1.7.0-dev` em 2026-08-04 para alinhar com a numeração real da suite (ver `sp-ui-shell/CHANGELOG.md`), e lançada como `v1.7.0` de verdade em 2026-08-06 — o rótulo `[1.5.0-dev]` original também não refletia a versão do `Cargo.toml` após o bump.*

### Security (2026-08-04)
- **Semgrep (SAST)**: `Gate 1: Semgrep (SAST)` bloqueava toda a pipeline (39 findings da regra `github-actions-mutable-action-tag`) nos 5 workflows (`ci.yml`, `deploy-oci.yml`, `devsecops.yml`, `docker.yml`, `release_notes.yml`) — nenhuma action estava pinada em SHA neste repo. Corrigido: `actions/checkout`, `actions/setup-python`, `actions/{upload,download}-artifact`, `Swatinem/rust-cache`, `docker/{setup-qemu,setup-buildx,login,metadata,build-push}-action` e `softprops/action-gh-release` agora pinados em SHA de commit real (verificado via GitHub API). `dtolnay/rust-toolchain@stable` também pinado — a resolução da versão do Rust é dinâmica em runtime, independente do SHA da action, então pinar o wrapper não congela o compilador. Validado localmente: `semgrep scan` 0 findings (era 39).

### Added
- **LLM Judge & Observability**: Rota de telemetria agora expõe os 15 relatórios mais recentes de auditoria do LLM Judge (scores de fidelidade e precisão) vindos da tabela `evaluations` (Item 6 do Roadmap).
- **Background Cron-Agents (Autonomia)**: Implementado o módulo `cron_agents.rs` com agentes independentes rodando em background (Market Pricing 12h, Gap Solver 24h, SQLite Backup 24h, RAG Reindex 7d) via `tokio::time::interval` (Item 8 do Roadmap).
- **TTFT Metric & Observability**: Adicionado rastreamento de TTFT (Time To First Token) no backend de telemetria para maior observabilidade do LLMOps (Item 6 do Roadmap).
- **Sovereign Fast-Router (`qwen3:0.6b`)**: Roteador dinâmico SLM que analisa a complexidade (P, M, G) e roteia para modelos maiores localmente baseando-se na memória disponível.
- **Cold-Boot Guard**: Sistema de retries (3 tentativas de 5s) no `reqwest::post` quando o Ollama retorna 500 ou Connection Reset, protegendo contra OOM/Timeout no carregamento da VRAM.
- **Auto-Eviction (10-Minute Eviction)**: `schedule_model_unload` implementado no `memory_manager.rs` para liberar modelos pesados da memória após o uso.

### Fixed — Distinção Financeira PTAX vs CAMBIO/USD (Item 3 do Roadmap)
- **Investigação**: o bug descrito no roadmap (PTAX fundido com BRL=X na dedup) **não existia mais em código** — o rename `DOLAR`→`DOLAR_SPOT` antes do passo de dedup por correlação Pearson (FIX-13) já impedia a colisão, mas sem nenhum teste provando isso.
- **Bug real encontrado no mesmo território**: `CAMBIO` e o indicador macro `USD` resolvem, por padrão, para a mesma série BCB SGS 10813 que `DOLAR_PTAX` (`FALLBACK_CHAINS` em `sovereign_matrix.py`), mas o `SEMANTIC_MAP` de `analyze_and_join_time_series.py` dava a cada um sua própria chave — duplicando a mesma série sob três nomes de coluna diferentes em vez de fundir.
- **Fix**: `CAMBIO` e `"INDICADOR USD"` (match específico, não `"USD"` solto — colide com o aviso `(USD/BRL)` de tickers convertidos) agora são aliases de `DOLAR_PTAX` no `SEMANTIC_MAP`, mergeando via o `combine_first` do FIX-13 em vez de duplicar.
- **Testes**: 6 testes novos em `test_time_series.py` — trava o cenário raiz (`PTAX` + `BRL=X` nunca fundem, mesmo com alta correlação real) e o bug do `CAMBIO`/`USD` (mergeiam com `DOLAR_PTAX`, sem colidir com o aviso de conversão). 114/114 testes passando (`pytest tests/`).

### Fixed — Resilience Shield: GAP-RS-02 (UNREACHABLE vs DEAD)
- **`health_check_apis.py` (`check_yahoo`):** falhas de rede (conexão recusada, timeout, DNS) agora emitem `UNREACHABLE` em vez de serem sempre reportadas como `DEAD`. Também passou a propagar o `timeout` configurado para `yfinance.Ticker.history()`, que antes era ignorado — sem isso, uma falha de rede nunca gerava exceção a tempo de ser classificada.
- **`health_gate.rs` (`ApiHealthSummary`/`build_summary`):** o agregado antes colapsava `DEAD`, `UNREACHABLE` e `EMPTY` em um único contador `degraded`, perdendo a distinção já presente nos `entries` brutos. Adicionados contadores `dead`, `unreachable`, `empty` preservados até o handler `GET /v1/analytics/api_health`.
- **`telemetry.svelte.ts` + `engineer/analytics/+page.svelte`:** o dado de saúde por API era buscado e armazenado em `telemetryState.apiEntries`, mas nenhum componente da UI o renderizava. Adicionado painel "Resilience Shield" com badge e ícone distintos por status (`HEALTHY`/`UNREACHABLE`/`DEAD`/`EMPTY`/`SKIP`).

### Fixed — Seguranca (CVEs Trivy HIGH)
- **CVE-2026-42327** (`openssl 0.10.78`): Undefined Behavior em `X509Ref::ocsp_responders` para certificados com OCSP URLs nao-UTF-8. Bumped para `0.10.79` no `Cargo.toml` + `cargo update` atualizou o `Cargo.lock`.
- **GHSA-82j2-j2ch-gfr8** (`rustls-webpki 0.103.10`): Denial of Service via panic em CRL BIT STRING malformado. Bumped para `0.103.13` via `cargo update rustls-webpki`.
- **GHSA-4w2j-m93h-cj5j** (`quinn-proto 0.11.14`, dependencia transitiva de `reqwest 0.12` via `quinn`): exaustao remota de memoria por reassembly ilimitado de streams fora de ordem. Corrigido via `cargo update -p quinn-proto --precise 0.11.15` (compativel com o range `^0.11` exigido por `quinn 0.11.9`, sem bump de dependencia direta). Achado no run `30962743616` do `ci.yml` (Gate 2: Trivy).

### Fixed — Clippy Gate (`-D warnings`)
- **`clippy::manual_flatten`** (`api.rs:1069`): Substituiu `for res in join_all(...).await { if let Ok((link, md)) = res { ... } }` por `for (link, md) in join_all(...).await.into_iter().flatten() { ... }` conforme sugerido pelo Clippy.
- **`clippy::op_ref`** (`fast_router.rs:56`): `name == &base_name` comparava `&str` contra uma referencia desnecessaria de `String` (`&base_name`); Clippy exige o valor direto (`name == base_name`), ja coberto por `PartialEq<String> for str`. Introduzido junto com o Sovereign Fast-Router (`v1.7.0-dev`) e nunca pego antes porque nenhum `cargo clippy -D warnings` rodou sobre o arquivo desde entao. Achado no mesmo run `30962743616` (Gate 3: Rust Clippy + Tests).

### Added — Empacotamento Nativo (.deb/.rpm/.pkg)
- **Gap**: `build-core`/`publish-nightly`/`publish-stable` só anexavam o binário `sovereign-daemon` solto por plataforma na release — sem `.deb`/`.rpm` no Linux nem `.pkg` no macOS, mesmo com o `ci.yml` já buildando os 4 targets.
- **Linux (`cargo-deb` + `cargo-generate-rpm`)**: novos steps no job `build-core` (matriz `ubuntu-latest`/`ubuntu-24.04-arm`) geram `.deb` e `.rpm` a partir do binário já compilado (`--no-build`/`--target`, sem rebuild). Metadata (`description`, `maintainer`, `license-file`, `depends = "$auto"`) adicionada em `[package.metadata.deb]`/`[package.metadata.generate-rpm]` no `Cargo.toml`. `cargo-deb` resolveu `Depends: libc6, libssl3t64, libstdc++6` automaticamente. RPM não aceita `-` no campo `version` (só `[A-Za-z0-9._+%{}~^]`) — `1.7.0-dev` é sanitizado em runtime para `1.7.0~dev` via `--set-metadata` (tilde ordena como pre-release, igual à convenção `dpkg`).
- **macOS (`pkgbuild`)**: novo step gera `.pkg` (instala em `/usr/local/bin/sovereign-daemon`) usando a ferramenta nativa do runner `macos-latest`, sem dependência externa.
- **Sem assinatura/notarização**: o projeto não tem certificado Apple Developer nem chave de assinatura Authenticode/GPG ainda. Pacotes publicados sem assinar — release notes avisam que o usuário precisa liberar manualmente (Gatekeeper no macOS, `dpkg -i`/`rpm -i` direto no Linux, sem repo APT/YUM próprio). Decisão registrada aqui para não ser confundida com omissão.
- **Docker (`docker.yml`)**: permanece manual (`workflow_dispatch`), sem mudança — decisão de desacoplamento já registrada anteriormente neste changelog.
- Validado localmente (ambiente Linux amd64): `cargo deb --no-build` gerou `.deb` instalável com deps corretas; `cargo generate-rpm` gerou `.rpm` válido (`file` reconhece como `RPM v3.0 bin`).

### Fixed — `publish-stable` nunca rodava de verdade (achado cortando a tag `v1.7.0`)
- **Problema 1**: `ci.yml` só escutava `push:branches:[main]` e `release:types:[created]` — nunca `push:tags` diretamente. O `release_notes.yml` cria a release via `softprops/action-gh-release` usando o `GITHUB_TOKEN` padrão, e por proteção do GitHub contra recursão, eventos gerados por uma Action usando esse token não disparam outros workflows. Resultado: `publish-stable` nunca tinha uma chance real de rodar, mesmo com uma tag de verdade cortada. Fix: `push:tags:['v*.*.*']` adicionado.
- **Problema 2**: na primeira tentativa real, `build-core` falhou só no runner ARM64 — `cargo-binstall` achou (errado) que `cargo-generate-rpm` já estava instalado ("already installed, use --force to override") e pulou a instalação sem erro, quebrando só no step seguinte ("no such command: generate-rpm"). Sem `fail-fast: false`, isso cancelou as outras 3 plataformas que estavam saudáveis. Fix: `--force` no `cargo binstall` + `fail-fast: false` na matriz.
- **Resultado real, confirmado em CI**: tag `v1.7.0` recortada sobre o commit corrigido — pipeline completo verde nas 4 plataformas, `publish-stable` publicou a release `v1.7.0` com binário + `.deb`/`.rpm`/`.pkg` de verdade pela primeira vez na história do repositório.

---

## [1.4.0-rc1] - 2026-05-07 — Estabilização de Pipeline e Docker
   
### Fixed — CI/CD e Docker (Forensic Fixes)
- **GLIBC Noble Migration:** Migrada imagem base do Docker de Debian Bookworm para Ubuntu 24.04 para compatibilidade com símbolos `__isoc23_strtoll` exigidos pelo `ort-sys`.
- **UID 1000 Collision:** Implementada remoção automática do usuário padrão `ubuntu` nas imagens Ubuntu 24.04 para garantir que o usuário `sovereign` possa assumir o UID 1000.
- **Binary Name Sync:** Corrigido erro de `COPY` no Dockerfile sincronizando o nome do artefato com o binário `sovereign-daemon` gerado pelo Cargo.
- **GHCR Permissions:** Adicionada permissão explícita `packages:write` ao job `build-docker` para resolver falhas de push no GitHub Container Registry.
- **Semgrep Security:** Corrigida vulnerabilidade de `run-shell-injection` no workflow Docker, migrando interpolações diretas `${{ }}` para variáveis de ambiente intermediárias.

### Changed — Arquitetura de Build
- **Docker Runtime-Only:** Refatorado Dockerfile para eliminar o stage de compilação redundante (~115min economizados no ARM64). A imagem agora consome binários pré-compilados injetados via build context.
- **Decoupling CI/Docker:** O build de imagens Docker foi desacoplado do pipeline principal `ci.yml`. Agora é executado on-demand via `docker.yml`, consumindo artefatos de releases estáveis ou nightly.

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
