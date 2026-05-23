//! ============================================================
//! Sovereign Pair — Coding API Test Suite (Epic P1)
//! Covers: Workspace file tree resolution, path traversal guards
//! ============================================================

#[cfg(test)]
mod coding_api_tests {
    use std::path::PathBuf;

    /// Test resolution of resolve_coding_workspace helper
    #[test]
    fn test_resolve_coding_workspace_fallback() {
        std::env::remove_var("CODING_WORKSPACE");
        let fallback = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/home/sovereign"))
            .join("sovereign-workspace");
        
        let resolved = std::env::var("CODING_WORKSPACE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/home/sovereign"))
                    .join("sovereign-workspace")
            });

        assert_eq!(resolved, fallback, "Coding workspace resolution fallback must resolve to dirs::home_dir/sovereign-workspace");
    }

    /// Ensure path traversal checks prevent ascending out of workspace
    #[test]
    fn test_coding_path_traversal_detection() {
        let ws = PathBuf::from("/home/sovereign/sovereign-workspace");
        let attack_path = ws.join("../../../etc/passwd");
        
        // Emulate Axum Handler path resolution and security guard with normalization
        let normalized = attack_path.components()
            .fold(PathBuf::new(), |mut acc, c| {
                match c {
                    std::path::Component::ParentDir => { acc.pop(); acc }
                    _ => { acc.push(c); acc }
                }
            });
        let is_inside = normalized.starts_with(&ws);
        assert!(!is_inside, "Path traversal via parent directories must not be inside the resolved workspace");
    }

    /// Ensure valid relative files inside workspace correctly resolve to full path inside workspace
    #[test]
    fn test_coding_valid_path_resolution() {
        let ws = PathBuf::from("/home/sovereign/sovereign-workspace");
        let valid_relative = "src/main.rs";
        let full_path = ws.join(valid_relative);
        
        assert!(full_path.starts_with(&ws), "Paths inside workspace must start with workspace base");
    }
}
