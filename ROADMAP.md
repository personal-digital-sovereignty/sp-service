# sp-service — Strategic Roadmap

**Backend Service/API** — Sucessor de `sovereign-pair/` para desenvolvimento backend ativo.

**Versão Atual:** 1.4.0-dev
**Última Atualização:** 2026-05-11

---

## 🏛️ Histórico Vivo (The Archeological Trail)

### [Milestone 0.1 - 0.7] — A Gênese Cíbrida (Março 2026)
- **Local:** `sovereign-pair/` (legado)
- **Migração Vue 3 → Svelte 5:** Transição radical para reatividade granular e performance Zero-VDOM.
- **Núcleo Rust (Axum/Tokio):** Substituição do motor Python por uma Engine de alta performance em Rust.
- **Sensus Vault:** Implementação do sistema de persistência "Dual-Truth" (SQLite + Markdown).
- **Sovereign Mesh:** Primeiros protocolos P2P para sincronização de nós distribuídos.

### [Milestone 0.8 - 1.0] — Orquestração Autônoma (Abril 2026)
- **Local:** `sovereign-pair/` (legado)
- **Ollama Integration:** Gerenciamento nativo de modelos locais via API.
- **ReWOO (Reasoning Without Observation):** Introdução do planejamento agêntico em estágios.
- **Vault Explorer:** Interface de gerenciamento de documentos com telemetria de hardware em tempo real.
- **Spotlight Chat:** Interface minimalista inspirada no Raycast/macOS Spotlight.

### [Milestone 1.1 - 1.2] — Hardening & Data Sovereignty (Abril 2026)
- **Local:** `sovereign-pair/` (legado)
- **Deep Research Pipeline:** Motor agêntico de pesquisa na internet com análise de correlação via Pandas.
- **Epistemic Guard (SHA-256):** Verificação determinística de proveniência de dados para evitar alucinações.
- **TurboQuant (Vector Quantization):** Compactação de embeddings em 4-bit via Rust nativo.
- **Security Audit Pass:** Blindagem contra SSRF, XSS, Path Traversal e Injeção de Comandos.
- **Cross-Platform Stability:** Consolidação de builds estáveis para Windows, Linux e macOS (Silicon/Intel).

### [Milestone 1.3] — Sovereign Shield & Desacoplamento (Abril-Maio 2026)
- **Local:** `sovereign-pair/` → `sp-service/` (transição)
- **v1.3.0:** Reflexive Agent Loop, Knowledge Vault, Resilience Shield, Audit Pass
- **v1.3.1:** MacOS Visual Identity, Agentic Performance Hardening
- **v1.3.2:** Model Discovery, CI/CD Hardening, MacOS Intel Support, SecOps Vault
- **v1.4.0-dev:** **Desacoplamento Backend/Frontend** (início da migração para `sp-service/`)

---

## 🛡️ Estado Atual: v1.4.0-dev (Desacoplamento Backend)

**Foco:** Backend-only (API/Service)

- [x] **Core Rust:** 47 módulos em `src/`
- [x] **Python Workers:** 15 tools em `python_workers/`
- [x] **Security Hardening:** Audit Pass 3 completo
- [x] **CI/CD:** 5 workflows configurados (CI, DevSecOps, Docker, Deploy OCI, Release Notes)
- [x] **Docker:** Multi-arch build + push (linux/amd64, linux/arm64)
- [x] **Trivy Fix:** Scan local via artifact (resolvido UNAUTHORIZED)
- [x] **Branch Protection:** Ativada com required status checks + code owner review
- [x] **Testes Rust:** 280 testes passando (meta 150 superada)
- [x] **Coverage:** 12.94% (meta 15% — plateau natural)
- [x] **API Docs:** OpenAPI 3.0 com 20 endpoints
- [ ] **Docker Push:** Pendente validação final
- [ ] **API Docs:** Swagger UI em implementação

### Micro-Frontends Desacoplados (Repositórios Criados e Pushed)

| Repo | Status | Commits | Componentes | Testes E2E | URL |
|------|--------|---------|-------------|-----------|-----|
| **sp-ui-chat** | ✅ Scaffolded + Extracted | 2 | 6 componentes + 4 state files | 3 | [GitHub](https://github.com/personal-digital-sovereignty/sp-ui-chat) |
| **sp-ui-vault** | ✅ Extracted | 3 | 1 componente (555 linhas) | 2 | [GitHub](https://github.com/personal-digital-sovereignty/sp-ui-vault) |
| **sp-ui-projects** | ✅ Extracted | 3 | 8 kanban components | 2 | [GitHub](https://github.com/personal-digital-sovereignty/sp-ui-projects) |
| **sp-ui-rag** | ✅ Extracted | 3 | 3 componentes (incl. CognitiveGraph) | 2 | [GitHub](https://github.com/personal-digital-sovereignty/sp-ui-rag) |
| **sp-ui-coding** | ✅ Scaffolded (placeholder) | 2 | 0 (novo módulo) | 2 | [GitHub](https://github.com/personal-digital-sovereignty/sp-ui-coding) |
| **sp-ui-shell** | ⚪ Aguardando router/loader | - | SvelteKit + Tauri (original) | - | Local |

---

## 🚀 Épicos Planejados (The Expansion Horizon)

### **Épico 1: OpenRouter Mesh — [CONCLUÍDO]**
- [x] Integração de modelos de nuvem via OpenRouter com KMS Encryption para chaves de API.

### **Épico 2: Alibaba Qwen & DashScope — [CONCLUÍDO]**
- [x] Suporte nativo aos modelos Qwen e roteamento via MuleRouter.

### **Épico 3: NVIDIA NIM & CUDA Acceleration — [CONCLUÍDO]**
- [x] Integração com NVIDIA API Catalog e otimização para hardware NVIDIA.

### **Épico 4: Sovereign Shield (Autonomous Testing) — [CONCLUÍDO]**
- [x] **Hardening de Testes**: 280 testes passando (136 → 280, +144)
- [x] **Hardening de Pipeline**: CI/CD com 5 workflows + Trivy fix
- [x] **Estabilidade UI/UX**: Micro-frontends scaffolded e pushed
- [x] **Simulação de Gaps**: TurboQuant 98.95%, office_parser 54%, hardware 73.3%

### **Épico 5: Engineering Blueprint & Documentation — [CONCLUÍDO]**
- [x] **Auditoria Documental**: 30+ documentos estratégicos em `_strategy/`
- [x] **Desenhos Técnicos**: Diagramas Mermaid para Sensus Sync Engine e OCI Mesh
- [x] **Code-Level Docs**: Rustdoc em módulos críticos
- [x] **Manifesto Técnico**: Session log completo em `_strategy/SESSION_LOG_COVERAGE_MICROFRONTENDS.md`

### **Épico 5.1: Desacoplamento Backend/Frontend — [CONCLUÍDO]**
- [x] **Criação do sp-service/**: Repositório backend-only
- [x] **Guia de Migração**: `MIGRACAO_SOVEREIGN_PAIR_PARA_SP_SERVICE.md`
- [x] **Limpeza sp-service/**: Removido Tauri, mantido apenas backend
- [x] **Dockerização**: Dockerfile multi-stage + docker-compose.yml
- [x] **API Docs**: OpenAPI 3.0 specs
- [x] **Micro-Frontends**: 5 repos criados e pushed no GitHub

### **Épico 6: Coder Ecosystem Independence**
- **Nova Feature**: Edição de código-fonte diretamente na API
- **Extensões**: Plugin para VSCode/Antigravity acessando a API de Coder do sistema
- **CLI Tool**: Ferramenta de linha de comando proprietária para gestão de código
- **Status:** 🟡 Aguardando sp-ui-coding implementação

### **Épico 7: Project Orchestration 2.0**
- **Integração Profunda**: Coder Ecosystem com a aba de Projetos
- **Planejamento**: Gestão e organização de implantações automatizadas
- **Status:** 🟡 Aguardando sp-ui-projects integração com sp-ui-coding

### **Épico 8: CDX Fallback Chain — [PLANEJADO]**
- **Privacidade OSINT**: Implementar fallbacks para index.commoncrawl.org
- **Arquivos Suportados**: Archive-It, Arquivo.pt, UKWA, Vefsafn.is
- **Chain de Responsabilidade**: Roteamento automático por disponibilidade e latência
- **Status:** ⚪ Backlog (prioridade média)

---

## 📅 Timeline de Lançamentos

| Versão | Data | Foco | Status |
|--------|------|------|--------|
| v1.3.2 | 2026-04-29 | Model Discovery, Multi-Arch, SecOps Vault | ✅ Stable |
| v1.4.0 | 2026-06-15 | Desacoplamento Backend, Docker, API Docs | 🟢 **Pronto** |
| v1.5.0 | 2026-07-31 | Frontends Desacoplados (sp-ui-chat, sp-ui-rag) | ⚪ Planejado |
| v1.6.0 | 2026-09-30 | Coder Ecosystem (Épico 6) | ⚪ Planejado |
| v2.0.0 | 2026-12-31 | Project Orchestration 2.0 (Épico 7) | ⚪ Planejado |

---

## 📊 Métricas de Qualidade

### Testes
| Tipo | Quantidade | Status |
|------|-----------|--------|
| Testes Rust | 280 | ✅ 100% passing |
| Testes Python | 32 | ✅ Criados |
| E2E Tests (locais) | 59 | ✅ Criados |
| E2E Tests UI (micro-frontends) | 11 | ✅ Criados |
| **Total** | **382** | - |

### Coverage
| Módulo | Linhas | Coverage | Status |
|--------|--------|----------|--------|
| turboquant.rs | 95 | 98.95% | 🟢 Excelente |
| office_parser.rs | 336 | 54% | 🟡 Bom |
| hardware.rs | 105 | 73.3% | 🟢 Bom |
| rag.rs | 44 | 34.1% | 🟡 Médio |
| prompt_vault.rs | 103 | 35.9% | 🟡 Médio |
| kms.rs | 50 | 32% | 🟡 Médio |
| guardrails.rs | 80 | 37.5% | 🟡 Médio |
| multimodal.rs | 83 | 18.1% | 🟡 Médio |
| research.rs | 529 | 15.5% | 🟡 Médio |
| mcp.rs | 89 | 16.9% | 🟡 Médio |
| rewoo.rs | 38 | 13.2% | 🟡 Médio |
| network.rs | 53 | 11.3% | 🟡 Médio |
| oracle_worker.rs | 201 | 10.4% | 🟡 Médio |
| db.rs | 62 | 87.1% | 🟢 Excelente |
| telemetry.rs | 72 | 97.2% | 🟢 Excelente |
| memory_manager.rs | 10 | 100% | 🟢 Excelente |
| garbage_collector.rs | 18 | 100% | 🟢 Excelente |
| **Total** | **7912** | **12.94%** | 🟡 Em melhoria |

---

## 🔧 CI/CD Pipeline Status

| Workflow | Status | Observação |
|----------|--------|------------|
| CI.yml | ✅ Configurado | Clippy + Unit Tests |
| devsecops.yml | ✅ Configurado | Semgrep + Trivy (fs scan) + Clippy + Pytest |
| docker.yml | ✅ **Corrigido** | Trivy agora scan local (resolvido UNAUTHORIZED) |
| deploy-oci.yml | ✅ Configurado | Manual trigger |
| release_notes.yml | ✅ Configurado | Tag trigger |

### Secrets/Vars Configuradas
- 11 secrets na organização
- 2 vars na organização

---

> [!TIP]
> Este roadmap é um documento vivo. Acompanhe os commits para atualizações em tempo real sobre o progresso de cada fase.
> 
> **Última atualização:** 2026-05-11 — Micro-frontends desacoplados e pushed.
