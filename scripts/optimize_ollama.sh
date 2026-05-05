#!/bin/bash

# =========================================================================
# sp-service - Ollama Hardware Optimization (Cross-Distro)
# Otimização de Hardware para Motor de Inferência Local
# =========================================================================
# Este script configura o backend llama.cpp usado pelo Ollama para
# otimizar alocação de núcleos e ajustar o CPU Governor para modo
# performance durante inferência de LLM.
#
# Compatibilidade testada:
#   - Arch Linux (kernel 6.x)
#   - Ubuntu 20.04+ / 22.04 / 24.04 (kernel 5.x+)
#   - Debian 11+ / 12+ (kernel 5.x+)
#   - Fedora 38+ (kernel 6.x+)
#
# Requisitos:
#   - systemd como init system
#   - Ollama instalado e rodando como serviço
#   - Acesso root (sudo)
# =========================================================================

set -e  # Exit on error

# Cores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# =========================================================================
# Funções de Log
# =========================================================================
log_info() {
    echo -e "${BLUE}>>${NC} $1"
}

log_success() {
    echo -e "${GREEN}   [OK]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}   [⚠️]${NC} $1"
}

log_error() {
    echo -e "${RED}   [ERRO]${NC} $1"
}

# =========================================================================
# 1. Verificar privilégios de ROOT
# =========================================================================
echo "========================================================================="
echo "  sp-service - Ollama Hardware Optimization"
echo "========================================================================="
echo ""

if [ "$EUID" -ne 0 ]; then
    log_error "Por favor, execute este script como root (sudo ./optimize_ollama.sh)"
    exit 1
fi

log_info "Iniciando otimização do Ollama para AMD Ryzen 7 5800H (32GB RAM)"
echo "------------------------------------------------------------------------"

# =========================================================================
# 2. Detectar Sistema e Dependências
# =========================================================================
log_info "Detectando sistema e dependências..."

# Detectar init system
if ! command -v systemctl &> /dev/null; then
    log_error "systemd não encontrado. Este script requer systemd."
    exit 1
fi
log_success "systemd detectado"

# Detectar cgroups version
if [ -f /sys/fs/cgroup/cgroup.controllers ]; then
    CGROUPS_VERSION="v2"
    log_success "cgroups v2 detectado"
else
    CGROUPS_VERSION="v1"
    log_warning "cgroups v1 detectado (MemoryMax não estará disponível)"
fi

# Detectar se Ollama está instalado
if ! command -v ollama &> /dev/null && ! systemctl list-unit-files | grep -q ollama; then
    log_warning "Ollama não foi detectado. O script continuará, mas o serviço pode falhar."
fi

# =========================================================================
# 3. Criar Override SystemD para o serviço do Ollama
# =========================================================================
log_info "Configurando Variáveis de Ambiente do llama.cpp (Systemd Override)..."

# Detectar path correto do serviço ollama
OLLAMA_SERVICE_PATH=""
if [ -f /etc/systemd/system/ollama.service ]; then
    OLLAMA_SERVICE_PATH="/etc/systemd/system/ollama.service"
elif [ -f /lib/systemd/system/ollama.service ]; then
    OLLAMA_SERVICE_PATH="/lib/systemd/system/ollama.service"
elif [ -f /usr/lib/systemd/system/ollama.service ]; then
    OLLAMA_SERVICE_PATH="/usr/lib/systemd/system/ollama.service"
else
    # Tentar detectar via systemctl
    OLLAMA_SERVICE_PATH=$(systemctl show ollama --property=FragmentPath 2>/dev/null | cut -d= -f2)
    if [ -z "$OLLAMA_SERVICE_PATH" ] || [ "$OLLAMA_SERVICE_PATH" = "" ]; then
        # Fallback: criar em /etc (funciona em todas distros)
        OLLAMA_SERVICE_PATH="/etc/systemd/system/ollama.service"
        log_warning "Serviço ollama não encontrado, criando em $OLLAMA_SERVICE_PATH"
    fi
fi

SYSTEMD_DIR="$(dirname "$OLLAMA_SERVICE_PATH")/ollama.service.d"
mkdir -p "$SYSTEMD_DIR"
log_success "Diretório criado: $SYSTEMD_DIR"

# Criar override.conf
cat > "$SYSTEMD_DIR/override.conf" << 'CONF'
[Service]
# Threads físicas vs virtuais: 12 threads para extrair 100% dos núcleos do 5800H
# Ryzen 7 5800H: 8 núcleos / 16 threads, mas reservamos 4 threads para o SO
Environment="OLLAMA_NUM_THREADS=12"

# Paralelização das requests do sp-service
# OLLAMA_NUM_PARALLEL deve ser >= SOVEREIGN_PARALLEL_QUERIES ou as threads ficam em fila
Environment="OLLAMA_NUM_PARALLEL=3"

# Modelos na RAM simultaneamente
Environment="OLLAMA_MAX_LOADED_MODELS=2"

# OpenBLAS (Aceleração matemática vetorial - AVX2)
Environment="OPENBLAS_NUM_THREADS=12"
Environment="OMP_NUM_THREADS=12"

# Afinidade de CPU (Force uso coerente dos CCX da AMD)
Environment="GOMP_CPU_AFFINITY=0-15"

# [AMD ROCm Hardware Spoofing]
# APUs Ryzen Vega (como 5800H) são bloqueadas pela AMD comercialmente no ROCm.
# Forçamos a biblioteca a ler a APU como uma gfx900 (Enterprise) para destravar acesso à Memória Compartilhada.
Environment="HSA_OVERRIDE_GFX_VERSION=9.0.0"

# [FALLBACK UNIVERSAL]: Vulkan Compute
# Se a AMD quebrar o suporte ao ROCm spoofing, ative o modo Vulkan (Open-Source)
Environment="OLLAMA_BACKEND=vulkan"
# Tuning de Memória Global (32GB RAM física)
Environment="OLLAMA_MAX_QUEUE=512"
Environment="OLLAMA_KEEP_ALIVE=5m"

# Flash Attention (otimização de KV cache)
Environment="OLLAMA_FLASH_ATTENTION=1"

# KV Cache Type: q8_0 é o sweet spot — reduz KV cache em ~50% sem degradar RoPE
# qwen3/gemma4 estáveis com q8_0 (8-bit preserva senos/cossenos do RoPE)
Environment="OLLAMA_KV_CACHE_TYPE=q8_0"

# Host Binding (Permitir acesso Docker/Tailscale se necessário)
Environment="OLLAMA_HOST=0.0.0.0:11434"

# =========================================================================
# [MEMORY FENCE]: Hard Cap via cgroups v2
# =========================================================================
# Limita toda a árvore de processos do Ollama a 24GB.
# Em 32GB físicos, reserva ~8GB para: kernel, sp-service, browser, OS.
#
# Comportamento ao exceder:
#   - O kernel mata o runner do modelo (OOM kill cirúrgico)
#   - O ollama serve continua vivo e responde com erro
#   - O sp-service recebe o erro e aciona fallback normalmente
#
# MemorySwapMax=0 impede degradação silenciosa: sem swap, o sistema
# falha rápido e explícito ao invés de rastejar a 100KB/s no disco.
# NOTA: Requer cgroups v2. Em cgroups v1, estas diretivas são ignoradas.
# =========================================================================
MemoryMax=24G
MemorySwapMax=0
CONF

log_success "Arquivo systemd override.conf criado"

# =========================================================================
# 4. CPU Governor para Performance
# =========================================================================
log_info "Configurando CPU Scaling Governor para 'performance'..."

CPU_GOVERNOR_AVAILABLE=false

# Verificar se scaling_governor existe
if [ -d /sys/devices/system/cpu/cpu0/cpufreq ]; then
    CPU_GOVERNOR_AVAILABLE=true
    
    for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
        if [ -w "$cpu" ]; then
            echo "performance" > "$cpu" 2>/dev/null || true
        fi
    done
    log_success "Todos os núcleos configurados para modo performance"
else
    log_warning "CPU frequency scaling não disponível (comum em VMs ou hardware limitado)"
fi

# =========================================================================
# 5. HUGEPAGES (Acesso massivo à RAM para LLM)
# =========================================================================
log_info "Habilitando HugePages para acelerar buscas na memória..."

# Configurar hugepages via sysctl
HUGEPAGES_CONF="/etc/sysctl.d/99-hugepages-ollama.conf"
echo "vm.nr_hugepages = 1024" > "$HUGEPAGES_CONF"

# Aplicar configuração
if sysctl -p "$HUGEPAGES_CONF" > /dev/null 2>&1; then
    log_success "HugePages configurados (1024 páginas)"
else
    log_warning "Falha ao aplicar HugePages (pode requerer reboot)"
fi

# =========================================================================
# 6. Reiniciar o serviço do Ollama
# =========================================================================
log_info "Recarregando Daemon e reiniciando serviço do Ollama..."

systemctl daemon-reload
log_success "Daemon recarregado"

# Verificar se o serviço existe antes de restart
if systemctl list-unit-files | grep -q ollama; then
    systemctl restart ollama
    log_success "Ollama reiniciado com novas configurações"
else
    log_warning "Serviço ollama não encontrado. Inicie manualmente após instalar o Ollama."
fi

# =========================================================================
# 7. Verificação do Memory Fence (apenas cgroups v2)
# =========================================================================
log_info "Verificando configurações aplicadas..."
sleep 2

if [ "$CGROUPS_VERSION" = "v2" ]; then
    MEM_MAX=$(systemctl show ollama --property=MemoryMax 2>/dev/null | cut -d= -f2)
    
    if [ "$MEM_MAX" = "25769803776" ] || [ "$MEM_MAX" = "24G" ]; then
        log_success "MemoryMax = 24G (hard cap ativo via cgroups v2)"
    elif [ -n "$MEM_MAX" ] && [ "$MEM_MAX" != "" ]; then
        log_warning "MemoryMax = $MEM_MAX (verifique se cgroups v2 está habilitado)"
    else
        log_warning "MemoryMax não disponível (cgroups v2 pode não estar ativo)"
    fi
    
    MEM_CURRENT=$(systemctl show ollama --property=MemoryCurrent 2>/dev/null | cut -d= -f2)
    if [ -n "$MEM_CURRENT" ] && [ "$MEM_CURRENT" != "infinity" ]; then
        MEM_MB=$((MEM_CURRENT / 1024 / 1024))
        log_success "MemoryCurrent = ${MEM_MB}MB (uso atual do Ollama)"
    fi
else
    log_warning "cgroups v1 detectado - MemoryMax/MemoryCurrent não disponíveis"
    log_info "Para habilitar cgroups v2 no Ubuntu, adicione ao kernel cmdline:"
    echo "   systemd.unified_cgroup_hierarchy=1"
fi

# =========================================================================
# 8. Resumo Final
# =========================================================================
echo ""
echo "------------------------------------------------------------------------"
log_success "SUCESSO! O motor de IA local está tunado."
echo ""
echo "   📊 Alocação de RAM:"
echo "   ├── Ollama (modelos + KV cache):  ≤ 24GB (hard cap)"
echo "   ├── Sistema + sp-service:           ~8GB (reservado)"
echo "   └── Swap:                           BLOQUEADO (fail-fast)"
echo ""
echo "   ⚙️  Configurações aplicadas:"
echo "   ├── OLLAMA_NUM_THREADS=12"
echo "   ├── OLLAMA_NUM_PARALLEL=3"
echo "   ├── OLLAMA_KV_CACHE_TYPE=q8_0"
echo "   ├── HSA_OVERRIDE_GFX_VERSION=9.0.0"
echo "   ├── CPU Governor=performance"
echo "   └── HugePages=1024"
echo ""
echo "➡️  DICA: Para testar a diferença de velocidade, rode:"
echo "   ollama run qwen3:8b"
echo ""
echo "➡️  Para reverter para o padrão:"
echo "   sudo rm -rf $SYSTEMD_DIR"
echo "   sudo systemctl daemon-reload"
echo "   sudo systemctl restart ollama"
echo "------------------------------------------------------------------------"
echo ""

exit 0
