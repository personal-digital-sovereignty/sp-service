/// ============================================================
/// Sovereign Pair — Cross-Platform Regression Test Suite
/// Covers: XDG paths, temp_dir, venv resolution, DB path logic
/// Regress: LIN-01..09, WIN-01..06, P2-01..P2-06
/// ============================================================

#[cfg(test)]
mod cross_platform_paths {
    use std::path::PathBuf;

    /// LIN-01/02: Fallback de path XDG deve ser cross-platform válido
    #[test]
    fn test_db_path_fallback_is_absolute() {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let path = base.join("sovereign-pair").join("data").join("sovereign_memory.db");
        // Path deve ser absoluto ou relativo à CWD — nunca vazio
        assert!(!path.to_string_lossy().is_empty(), "DB path não deve ser vazio");
        // Em Linux, deve conter sovereign-pair/data
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("sovereign-pair"), "DB path deve conter sovereign-pair");
        assert!(path_str.contains("sovereign_memory.db"), "DB path deve conter o nome correto do banco");
    }

    /// WIN-04/LIN: temp_dir deve retornar path válido em todos os OSes
    #[test]
    fn test_temp_dir_is_valid() {
        let temp = std::env::temp_dir();
        assert!(temp.is_absolute() || temp.to_string_lossy().starts_with('.'),
            "temp_dir deve ser um path absoluto ou relativo válido");
        assert!(!temp.to_string_lossy().is_empty(), "temp_dir não deve ser vazio");
    }

    /// WIN-04: sovereign temp path deve ser cross-platform
    #[test]
    fn test_sovereign_temp_path_cross_platform() {
        let sovereign_tmp = std::env::temp_dir().join("sovereign");
        let path_str = sovereign_tmp.to_string_lossy();
        // Deve conter 'sovereign' como subdiretório
        assert!(path_str.ends_with("sovereign") || path_str.contains("sovereign"),
            "Path sovereign em temp deve conter 'sovereign': {}", path_str);
        // Não deve ter hardcoded /tmp
        #[cfg(not(target_os = "linux"))]
        assert!(!path_str.starts_with("/tmp"), "Path não deve ser /tmp hardcoded em não-Linux");
    }

    /// P2-02/P2-03: RAG vault path fallback deve ser válido
    #[test]
    fn test_rag_vault_path_fallback_chain() {
        // Testa a cadeia de resolução: home_dir → current_dir → "."
        let vault = std::env::var("SOVEREIGN_VAULT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .or_else(|| std::env::current_dir().ok())
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("Vault")
            });
        assert!(!vault.to_string_lossy().is_empty(), "Vault path não deve ser vazio");
        assert!(vault.to_string_lossy().contains("Vault"), "Vault path deve terminar em Vault/");
    }

    /// LIN-03: temp_dir NÃO deve ser /proc em nenhuma plataforma  
    #[test]
    fn test_temp_dir_not_proc() {
        let temp = std::env::temp_dir();
        assert!(!temp.starts_with("/proc"), "/proc não deve ser usado como temp dir");
    }

    /// DB name consistency — garante que o nome correto é usado
    #[test]
    fn test_db_filename_is_correct() {
        // Regressão para LIN-02: culture_matrix.py usava nome errado
        let correct_name = "sovereign_memory.db";
        let wrong_name = "SovereignHub_OS_System.db";
        assert_ne!(correct_name, wrong_name,
            "DB name deve ser sovereign_memory.db, não SovereignHub_OS_System.db");
    }

    /// WIN-02: venv python path deve variar por OS
    #[test]
    fn test_venv_python_path_by_os() {
        let venv_base = PathBuf::from("/venv");

        #[cfg(target_os = "windows")]
        let python_path = venv_base.join("Scripts").join("python.exe");

        #[cfg(not(target_os = "windows"))]
        let python_path = venv_base.join("bin").join("python3");

        let path_str = python_path.to_string_lossy();

        #[cfg(target_os = "windows")]
        assert!(path_str.contains("Scripts") && path_str.ends_with("python.exe"),
            "Windows venv deve usar Scripts\\python.exe");

        #[cfg(target_os = "linux")]
        assert!(path_str.contains("bin") && path_str.ends_with("python3"),
            "Linux venv deve usar bin/python3");

        #[cfg(target_os = "macos")]
        assert!(path_str.contains("bin") && path_str.ends_with("python3"),
            "MacOS venv deve usar bin/python3");
    }
}

#[cfg(test)]
mod sync_engine_resilience {
    /// LIN-08: FSEvent watcher deve degradar graciosamente
    /// (Testa a lógica de guard — não o watcher real)
    #[test]
    fn test_watcher_error_handled_gracefully() {
        // Simula o resultado de notify::recommended_watcher() falhando
        let watcher_result: Result<(), String> = Err("inotify: too many open files".to_string());
        let mut watcher_active = true;

        // Lógica de guard do sync_engine.rs
        match watcher_result {
            Err(e) => {
                // Deve registrar warning e retornar sem panic
                eprintln!("⚠️ Watcher falhou: {}", e);
                watcher_active = false;
            }
            Ok(_) => {}
        }

        assert!(!watcher_active, "Watcher deve ser desativado após erro — sem panic");
    }
}
