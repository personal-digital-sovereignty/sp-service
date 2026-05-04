# sp-service Scripts

Scripts de automação e otimização para o sp-service.

---

## Scripts Disponíveis

### 1. optimize_ollama.sh

**Finalidade:** Otimizar o Ollama para hardware AMD Ryzen 7 5800H (32GB RAM)

**Compatibilidade:**
- ✅ Arch Linux (kernel 6.x)
- ✅ Ubuntu 20.04+ / 22.04 / 24.04 (kernel 5.x+)
- ✅ Debian 11+ / 12+ (kernel 5.x+)
- ✅ Fedora 38+ (kernel 6.x+)

**Requisitos:**
- systemd como init system
- Ollama instalado e rodando como serviço
- Acesso root (sudo)

**Execução:**
```bash
cd scripts
sudo ./optimize_ollama.sh
```

**O que o script faz:**
1. Configura variáveis de ambiente do llama.cpp via systemd override
2. Define CPU Governor para modo performance
3. Habilita HugePages (1024 páginas)
4. Aplica Memory Fence de 24GB via cgroups v2
5. Reinicia o serviço do Ollama

**Configurações aplicadas:**
- `OLLAMA_NUM_THREADS=12` - 12 threads para o 5800H
- `OLLAMA_NUM_PARALLEL=3` - Paralelismo de requests
- `OLLAMA_KV_CACHE_TYPE=q8_0` - KV cache otimizado
- `HSA_OVERRIDE_GFX_VERSION=9.0.0` - AMD ROCm spoofing
- `MemoryMax=24G` - Hard cap de memória
- `MemorySwapMax=0` - Sem swap (fail-fast)

**Para reverter:**
```bash
sudo rm -rf /etc/systemd/system/ollama.service.d
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

---

## Estrutura de Diretórios

```
scripts/
├── optimize_ollama.sh    # Otimização de hardware Ollama
└── README.md             # Este arquivo
```

---

## Troubleshooting

### Script falha com "systemd não encontrado"

**Causa:** Sistema usa init diferente (SysV, OpenRC, etc.)

**Solução:** Este script requer systemd. Use uma distro com systemd.

### MemoryMax não aparece após execução

**Causa:** cgroups v1 ativo

**Solução (Ubuntu 20.04+):**
```bash
# Editar GRUB
sudo nano /etc/default/grub

# Adicionar ao GRUB_CMDLINE_LINUX
GRUB_CMDLINE_LINUX="... systemd.unified_cgroup_hierarchy=1"

# Atualizar GRUB e reboot
sudo update-grub
sudo reboot
```

### CPU Governor não disponível

**Causa:** VM ou hardware sem suporte a frequency scaling

**Solução:** Nenhuma necessária - script continua com warning. Ollama funcionará normalmente.

### HugePages falha ao aplicar

**Causa:** Limite do kernel ou memória insuficiente

**Solução:**
```bash
# Verificar hugepages disponíveis
cat /proc/sys/vm/nr_hugepages

# Tentar valor menor
echo "vm.nr_hugepages = 512" | sudo tee /etc/sysctl.d/99-hugepages-ollama.conf
sudo sysctl -p
```

---

## Validação

### Verificar configurações aplicadas

```bash
# Verificar override do systemd
systemctl cat ollama

# Verificar MemoryMax (cgroups v2)
systemctl show ollama --property=MemoryMax

# Verificar uso atual de memória
systemctl show ollama --property=MemoryCurrent

# Verificar CPU Governor
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Verificar HugePages
cat /proc/sys/vm/nr_hugepages
```

### Testar performance

```bash
# Rodar modelo de teste
ollama run qwen3:8b "Conte de 1 a 100"

# Verificar tokens por segundo
# Deve observar melhoria de 20-40% após otimização
```

---

## Contato

- Repositorio: https://github.com/Personal-Digital-Sovereignty/sp-service
- Email: jefersonlopes@proton.me

---

**Última Atualização:** 2026-05-04
**Versão:** 1.0.0
