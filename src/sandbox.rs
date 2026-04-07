use std::env;
use std::path::PathBuf;
use std::process::Command;
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;

/// Retorna o caminho base do ecossistema Sovereign
fn get_base_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| {
        env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string())
    });
    PathBuf::from(home).join(".local/share/sovereign-pair/sandbox")
}

/// Retorna o caminho do executável Python dentro da bolha (Venv)
pub fn get_hermetic_python_bin() -> PathBuf {
    let base = get_base_path().join("venv");
    if cfg!(target_os = "windows") {
        base.join("Scripts").join("python.exe")
    } else {
        base.join("bin").join("python3")
    }
}

/// Retorna o caminho do Pip dentro da bolha
pub fn get_hermetic_pip_bin() -> PathBuf {
    let base = get_base_path().join("venv");
    if cfg!(target_os = "windows") {
        base.join("Scripts").join("pip.exe")
    } else {
        base.join("bin").join("pip")
    }
}

/// Inicializa e provisiona a sandbox na inicialização do sistema
pub async fn setup_python_sandbox() -> bool {
    let sandbox_dir = get_base_path();
    let venv_dir = sandbox_dir.join("venv");

    if venv_dir.exists() {
        return true; // Já está provisionado
    }

    info!("📦 [Sovereign Sandbox] Provisionando ambiente Python Hermético na raiz do usuário...");
    if !sandbox_dir.exists() {
        let _ = fs::create_dir_all(&sandbox_dir).await;
    }

    // 1. Criar o Venv usando o Python do Host
    let python_cmds = if cfg!(target_os = "windows") {
        vec!["python", "py", "python3"]
    } else {
        vec!["python3", "python"]
    };

    let mut venv_created = false;
    for cmd in python_cmds {
        let status = Command::new(cmd)
            .arg("-m")
            .arg("venv")
            .arg(&venv_dir)
            .status();

        if status.is_ok_and(|st| st.success()) {
            venv_created = true;
            break;
        }
    }

    if !venv_created {
        warn!("❌ [Sovereign Sandbox] Falha ao criar a Sandbox! O host O.S possui módulo 'python3-venv' instalado?");
        return false;
    }

    info!("🐍 [Sovereign Sandbox] Venv criado. Instalando pacotes analíticos universais (Numpy, Pandas, etc)...");
    
    // 2. Instalar Pacotes Críticos via pip hermético
    let pip_bin = get_hermetic_pip_bin();
    let install_status = Command::new(&pip_bin)
        .arg("install")
        .arg("--disable-pip-version-check")
        .arg("-q") // Modo silencioso
        .arg("pandas")
        .arg("numpy")
        .arg("yfinance")
        .arg("requests")
        .arg("duckduckgo-search")
        .arg("duckdb")
        .status();

    if install_status.is_ok_and(|st| st.success()) {
        info!("✅ [Sovereign Sandbox] Ambiente Matemático e Analítico isolado com sucesso!");
        return true;
    }
    
    warn!("⚠️ [Sovereign Sandbox] Venv criado, mas a instalação de pacotes via pip falhou.");
    false
}

/// Executa um script Python puramente dentro da bolha hermética.
/// Retorna Stdout puro ou Err(Stderr).
pub async fn execute_python_code(code: &str) -> Result<String, String> {
    let python_bin = get_hermetic_python_bin();
    if !python_bin.exists() {
        return Err("A Sovereign Sandbox não foi inicializada neste S.O.".to_string());
    }

    let script_name = format!("sovereign_execute_{}.py", Uuid::new_v4());
    let temp_dir = env::temp_dir();
    let script_path = temp_dir.join(&script_name);

    if let Err(e) = fs::write(&script_path, code).await {
        return Err(format!("Falha ao injetar script no sistema de arquivos: {}", e));
    }

    let _ = tracing_subscriber::fmt::format(); // Silencia logs de formatação
    info!("⚙️ [Plan & Execute] Disparando script na Sandbox Hermética: {}", script_name);

    let output = Command::new(&python_bin)
        .arg(&script_path)
        .current_dir(&temp_dir)
        .output();

    // Tentar apagar o script de forma silenciosa para não poluir /tmp
    let _ = fs::remove_file(&script_path).await;

    match output {
        Ok(out) => {
            let stdout_str = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr_str = String::from_utf8_lossy(&out.stderr).to_string();

            if out.status.success() {
                Ok(stdout_str)
            } else {
                Err(format!("{}\n{}", stdout_str, stderr_str))
            }
        },
        Err(e) => {
            Err(format!("Falha Crítica ao invocar Sandbox Core: {}", e))
        }
    }
}
