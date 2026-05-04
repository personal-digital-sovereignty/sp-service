# sp-service - Branch Protection Configuration

Data: 2026-05-03
Status: Instrucoes de Configuracao

---

## Contexto

Secrets e Vars devem estar configurados a nivel de organizacao para compartilhamento entre repositorios.

### Secrets (Organization-level)

- OCI_TENANCY_OCID
- OCI_USER_OCID
- OCI_FINGERPRINT
- OCI_PRIVATE_KEY
- OCI_COMPARTMENT_OCID
- OCI_SSH_PUBLIC_KEY
- OCI_SSH_PRIVATE_KEY
- TAILSCALE_AUTH_KEY
- PAT_GHCR
- DOCKERHUB_USERNAME (opcional)
- DOCKERHUB_TOKEN (opcional)

### Vars (Organization-level)

- OCI_REGION (default: sa-saopaulo-1)
- OCI_BOOT_VOLUME_SIZE (default: 200)

---

## Configuracao de Branch Protection

### Passo 1: Acessar Settings

1. Navegar para: https://github.com/Personal-Digital-Sovereignty/sp-service
2. Clicar em "Settings"
3. Clicar em "Branches" no menu lateral
4. Clicar em "Add branch protection rule"

### Passo 2: Configurar Rule

**Branch name pattern:** `main`

**Marcar as seguintes opcoes:**

- [x] Require a pull request before merging
  - [x] Require approvals
    - Required number of approvals before merging: `1`
  - [ ] Dismiss stale pull request approvals when new commits are pushed
  - [x] Require review from Code Owners

- [x] Require status checks to pass before merging
  - [x] Require branches to be up to date before merging
  - Status checks that are required:
    - `FOSS DevSecOps Gate` (devsecops.yml)
    - `FOSS Enterprise DevSecOps` (ci.yml)

- [x] Require conversation resolution before merging

- [ ] Do not allow bypassing the above settings
  - (Opcional: marcar para incluir administrators)

### Passo 3: Salvar

- Clicar em "Create" ou "Save changes"

---

## Validacao

### Test 1: Criar PR sem testes passando

1. Criar branch feature/test
2. Fazer commit sem rodar testes
3. Criar Pull Request para main
4. **Esperado:** PR deve mostrar status checks como required e bloqueados

### Test 2: Criar PR com testes passando

1. Rodar `cargo test` localmente
2. Fazer commit com testes passando
3. Criar Pull Request para main
4. **Esperado:** Status checks devem passar, mas ainda requer 1 approval de Code Owner

### Test 3: Tentar merge sem approval

1. Com PR aprovado pelos status checks
2. Tentar merge sem review de Code Owner
3. **Esperado:** Merge button deve estar bloqueado com mensagem "Review required"

---

## Workflow de Merge Aprovado

1. **PR Criado** -> Status checks rodam automaticamente
2. **Status Checks Passam** -> devsecops.yml e ci.yml devem estar verdes
3. **Code Owner Review** -> @jefersonlopes deve aprovar o PR
4. **Merge** -> Botao de merge e liberado
5. **CI/CD Principal** -> ci.yml roda build matrix e deploy

---

## Troubleshooting

### Problema: Status checks nao aparecem

**Solucao:**
1. Verificar se workflows estao em `.github/workflows/`
2. Verificar se `devsecops.yml` tem trigger `pull_request`
3. Rodar workflow manualmente via `workflow_dispatch`

### Problema: Code Owners nao sao notificados

**Solucao:**
1. Verificar se `.github/CODEOWNERS` existe
2. Verificar se pattern `* @jefersonlopes` esta correto
3. Verificar se usuario tem permissao de write no repositorio

### Problema: Secrets nao estao disponiveis

**Solucao:**
1. Verificar se secrets estao em Organization Settings -> Secrets
2. Verificar se repositorio tem acesso aos secrets da organizacao
3. Recriar secrets se necessario

---

## Referencias

- GitHub Branch Protection Docs: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches
- CODEOWNERS Docs: https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners
- Organization Secrets: https://docs.github.com/en/actions/security-guides/encrypted-secrets#creating-encrypted-secrets-for-an-organization

---

Documento Gerado: 2026-05-03
Responsavel: @jefersonlopes
