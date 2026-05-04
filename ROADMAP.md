# sp-service — Strategic Roadmap

**Backend Service/API** — Sucessor de `sovereign-pair/` para desenvolvimento backend ativo.

**Versão Atual:** 1.4.0-dev  
**Última Atualização:** 2026-05-03

---

## 🏛️ Histórico Vivo (The Archeological Trail)

Abaixo, as Milestones retroativas validadas via histórico de commits e documentação técnica, mapeando a evolução do projeto desde sua concepção.

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

- [x] **Core Rust:** 47 módulos em `core/src/`
- [x] **Python Workers:** 15 tools em `python_workers/`
- [x] **Security Hardening:** Audit Pass 3 completo
- [x] **CI/CD:** Builds cross-platform (Linux, macOS, Windows)
- [ ] **Docker:** Containerização em implementação
- [ ] **API Docs:** OpenAPI/Swagger em implementação

### Frontends Desacoplados (Repositórios Separados)

- [ ] **sp-ui-chat:** Frontend Chat (Épico 1)
- [ ] **sp-ui-rag:** Frontend RAG Pipeline (Épico 2)
- [ ] **sp-ui-coding:** Frontend Coder Ecosystem (Épico 6)
- [ ] **sp-ui-projects:** Frontend Projects (Épico 7)
- [ ] **sp-ui-vault:** Frontend Vault Explorer (Épico 3)
- [ ] **sp-ui-shell:** Shell/Tray Icon (Épico 4)

---

## 🚀 Épicos Planejados (The Expansion Horizon)

### **Épico 1: OpenRouter Mesh — [CONCLUÍDO]**
- [x] Integração de modelos de nuvem via OpenRouter com KMS Encryption para chaves de API.

### **Épico 2: Alibaba Qwen & DashScope — [CONCLUÍDO]**
- [x] Suporte nativo aos modelos Qwen e roteamento via MuleRouter.

### **Épico 3: NVIDIA NIM & CUDA Acceleration — [CONCLUÍDO]**
- [x] Integração com NVIDIA API Catalog e otimização para hardware NVIDIA.

### **Épico 4: Sovereign Shield (Autonomous Testing) — [CONCLUÍDO]**
- [x] **Hardening de Testes**: Cobertura de 100% dos fluxos agênticos sem intervenção humana.
- [x] **Hardening de Pipeline**: Implementação de cache inteligente, targets explícitos, SQLite bundled e fixação de versões de actions (SHA).
- [x] **Estabilidade UI/UX**: Correção de lints de acessibilidade e validação de configurações via Playwright.
- [x] **Simulação de Gaps**: Automação de testes de regressão massivos e estresse de tokens.

### **Épico 5: Engineering Blueprint & Documentation — [CONCLUÍDO]**
- [x] **Auditoria Documental**: Purga de relatórios legados e centralização em `docs/engineering/`.
- [x] **Desenhos Técnicos**: Criação de diagramas Mermaid para Sensus Sync Engine e OCI Mesh.
- [x] **Code-Level Docs**: Auditoria minuciosa e Rustdoc em 100% do core e frontend crítico.
- [x] **Manifesto Técnico**: Finalização do `BLUEPRINT.md` e do `Unified SecOps Vault`.

### **Épico 5.1: Desacoplamento Backend/Frontend — [EM ANDAMENTO]**
- [x] **Criação do sp-service/**: Novo repositório backend-only
- [x] **Guia de Migração**: `MIGRACAO_SOVEREIGN_PAIR_PARA_SP_SERVICE.md`
- [ ] **Limpeza sp-service/**: Remover referências a UI/Svelte/Tauri
- [ ] **Dockerização**: Criar Dockerfile e docker-compose.yml
- [ ] **API Docs**: Gerar OpenAPI/Swagger specs

### **Épico 6: Coder Ecosystem Independence**
- **Nova Feature**: Edição de código-fonte diretamente na API
- **Extensões**: Plugin para VSCode/Antigravity acessando a API de Coder do sistema
- **CLI Tool**: Ferramenta de linha de comando proprietária para gestão de código
- **Status:** Aguardando frontends desacoplados (sp-ui-coding)

### **Épico 7: Project Orchestration 2.0**
- **Integração Profunda**: Coder Ecosystem com a aba de Projetos
- **Planejamento**: Gestão e organização de implantações automatizadas
- **Status:** Aguardando frontends desacoplados (sp-ui-projects)

### **Épico 8: CDX Fallback Chain — [PLANEJADO]**
- **Privacidade OSINT**: Implementar fallbacks para index.commoncrawl.org
- **Arquivos Suportados**: Archive-It, Arquivo.pt, UKWA, Vefsafn.is
- **Chain de Responsabilidade**: Roteamento automático por disponibilidade e latência
- **Status:** Backlog (prioridade média)

---

## 📅 Timeline de Lançamentos

| Versão | Data | Foco | Status |
|--------|------|------|--------|
| v1.3.2 | 2026-04-29 | Model Discovery, Multi-Arch, SecOps Vault | ✅ Stable |
| v1.4.0 | 2026-06-15 | Desacoplamento Backend, Docker, API Docs | 🟡 Dev |
| v1.5.0 | 2026-07-31 | Frontends Desacoplados (sp-ui-chat, sp-ui-rag) | ⚪ Planejado |
| v1.6.0 | 2026-09-30 | Coder Ecosystem (Épico 6) | ⚪ Planejado |
| v2.0.0 | 2026-12-31 | Project Orchestration 2.0 (Épico 7) | ⚪ Planejado |

---

> [!TIP]
> Este roadmap é um documento vivo. Acompanhe os commits para atualizações em tempo real sobre o progresso de cada fase.
