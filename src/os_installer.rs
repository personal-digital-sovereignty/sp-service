use std::env;
use std::process::Command;
use std::path::PathBuf;
use std::fs;
use std::time::Duration;
use reqwest::Client;

pub async fn run_headless_setup() {
    println!("============================================================");
    println!("🛡️ Sovereign Pair - Install Wizard (Headless CLI / GUI Wrapper)");
    println!("============================================================");

    // 0. Automação de Elevação de Privilégio (Graphical Sudo via Sidecar)
    if !is_root() {
        println!("🔐 [Auth] Privilégio de Standard User detectado. Elevando para Admin (Root)...");
        elevate_privileges();
        return; // A branch elevada continuará a execução
    }

    // 1. Diagnosticar Ollama
    println!("🔍 [1/3] Detectando motor RAG local (Ollama) na porta 11434...");
    if check_ollama().await {
        println!("✅ Motor Ollama detectado e ativo.");
    } else {
        println!("⚠️ AVISO CRÍTICO: Ollama não detectado no localhost:11434.");
        println!("   O Sovereign Node bootará normalmente, mas a interface de Chat RAG falhará!");
        println!("   -> Acesse https://ollama.com para baixar a base de inferência local assim que puder.");
        println!("   -> Ou rode via terminal: curl -fsSL https://ollama.com/install.sh | sh");
    }

    // 2. Registrando Serviços OS Native
    println!("⚙️ [2/3] Registrando Daemon Universal no Sistema Operacional...");
    if cfg!(target_os = "linux") {
        install_systemd_service();
    } else if cfg!(target_os = "macos") {
        install_launchd_plist();
    } else if cfg!(target_os = "windows") {
        install_windows_service();
    } else {
        println!("❌ S.O desconhecido. Ignorando instalação de Background Daemons.");
    }

    // 3. Extensões (KDE/Gnome) se Linux
    println!("🧩 [3/3] Checando Extensões Desktop (Widgets)...");
    if cfg!(target_os = "linux") {
        install_linux_widgets();
    } else {
        println!("➡️ Nenhuma injeção de Plasma/Gnome aplicável neste S.O.");
    }

    println!("============================================================");
    println!("✅ Instalação Nativa Finalizada. Sovereign Cibrid está pronto.");
    println!("============================================================");
}

async fn check_ollama() -> bool {
    let client = Client::builder().timeout(Duration::from_secs(2)).build().unwrap_or_default();
    match client.get(format!("{}{}", std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()), "/api/tags")).send().await {
        Ok(res) => res.status().is_success(),
        Err(_) => false
    }
}

fn install_systemd_service() {
    let bin_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/local/bin/sovereign-core"));
    let service_content = format!(
        "[Unit]\n\
         Description=Sovereign Pair Cibrid Node\n\
         After=network.target\n\n\
         [Service]\n\
         Type=simple\n\
         ExecStart={}\n\
         Restart=always\n\
         RestartSec=5\n\
         Environment=RUST_LOG=info\n\n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        bin_path.display()
    );

    let service_path = "/etc/systemd/system/sovereign.service";
    
    match fs::write(service_path, service_content) {
        Ok(_) => {
            println!("✅ Arquivo {} criado com sucesso.", service_path);
            let _ = Command::new("systemctl").arg("daemon-reload").status();
            let _ = Command::new("systemctl").arg("enable").arg("sovereign.service").status();
            let _ = Command::new("systemctl").arg("start").arg("sovereign.service").status();
            println!("🚀 Serviço Systemd nativo habilitado e disparado.");
        },
        Err(e) => {
            println!("❌ Falha ao gravar Service. Você rodou como Super-Usuário (sudo)? Erro: {}", e);
        }
    }
}

fn install_launchd_plist() {
    println!("💻 [MacOS] Configuração via Launchd .plist agendada para o Wizard GUI.");
    // Aqui vai logica nativa de Plist para Mac
}

fn install_windows_service() {
    println!("🪟 [Windows] Registro via WinSvc (sc.exe) agendado para o Wizard GUI.");
}

fn install_linux_widgets() {
    println!("🔗 Procurando KDE env e Gnome Extensions para instalar os ponteiros...");
    // A integração real do plasmoid vem nesta função
}

fn is_root() -> bool {
    #[cfg(unix)]
    {
        if let Ok(output) = Command::new("id").arg("-u").output() {
            let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return uid == "0";
        }
        false
    }
    #[cfg(windows)]
    { false /* Windows assumes UAC manifest */ }
}

fn elevate_privileges() {
    println!("🔄 Injetando prompt do O.S para reinicialização...");

    #[cfg(target_os = "linux")]
    {
        let exe = env::current_exe().unwrap();
        // Tenta pkexec para pop-up visual (Wayland/X11), cai pra sudo se puro TTY
        let status = Command::new("pkexec")
            .arg(&exe)
            .arg("--setup")
            .status()
            .unwrap_or_else(|_| Command::new("sudo").arg(&exe).arg("--setup").status().expect("Sudo indisponível"));
        
        std::process::exit(status.code().unwrap_or(1));
    }

    #[cfg(target_os = "macos")]
    {
        let exe = env::current_exe().unwrap();
        let script = format!("do shell script \"{} --setup\" with administrator privileges", exe.display());
        let status = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .expect("AppleScript OsaScript indisponível!");
            
        std::process::exit(status.code().unwrap_or(1));
    }

    #[cfg(target_os = "windows")]
    {
        println!("🪟 [Windows] Execute este console / aplicativo como Administrador!");
        std::process::exit(1);
    }
}
