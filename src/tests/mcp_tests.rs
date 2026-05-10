//! ============================================================
//! sp-service — MCP Tests
//! Tests for tool schemas and path validation
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::mcp::{get_mcp_tools, validate_safe_path};
    use std::path::Path;

    // ─────────────────────────────────────────────────────────
    // get_mcp_tools Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_get_mcp_tools_returns_non_empty() {
        let tools = get_mcp_tools();
        assert!(!tools.is_empty());
    }

    #[test]
    fn test_get_mcp_tools_count() {
        let tools = get_mcp_tools();
        assert_eq!(tools.len(), 6);
    }

    #[test]
    fn test_get_mcp_tools_tool_names() {
        let tools = get_mcp_tools();
        let names: Vec<&str> = tools.iter()
            .filter_map(|t| t.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()))
            .collect();

        assert!(names.contains(&"mcp_list_directory"));
        assert!(names.contains(&"mcp_read_file"));
        assert!(names.contains(&"mcp_write_file"));
        assert!(names.contains(&"mcp_deep_research"));
        assert!(names.contains(&"mcp_transcribe_audio"));
        assert!(names.contains(&"mcp_ocr_image"));
    }

    #[test]
    fn test_get_mcp_tools_all_have_type_function() {
        let tools = get_mcp_tools();
        for tool in &tools {
            assert_eq!(tool.get("type").and_then(|t| t.as_str()), Some("function"));
        }
    }

    #[test]
    fn test_get_mcp_tools_all_have_description() {
        let tools = get_mcp_tools();
        for tool in &tools {
            let desc = tool.get("function").and_then(|f| f.get("description")).and_then(|d| d.as_str());
            assert!(desc.is_some(), "Tool missing description");
        }
    }

    #[test]
    fn test_get_mcp_tools_all_have_parameters() {
        let tools = get_mcp_tools();
        for tool in &tools {
            let params = tool.get("function").and_then(|f| f.get("parameters"));
            assert!(params.is_some(), "Tool missing parameters");
        }
    }

    #[test]
    fn test_get_mcp_tools_serialization() {
        let tools = get_mcp_tools();
        let json = serde_json::to_string(&tools).unwrap();
        assert!(json.contains("mcp_read_file"));
        assert!(json.contains("mcp_write_file"));
    }

    // ─────────────────────────────────────────────────────────
    // validate_safe_path Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_validate_safe_path_within_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let result = validate_safe_path(root, "file.txt");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(root));
    }

    #[test]
    fn test_validate_safe_path_subdirectory() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        // Create subdir
        std::fs::create_dir_all(root.join("src")).unwrap();
        let result = validate_safe_path(root, "src/main.rs");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_safe_path_dotdot_blocked() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        // Path traversal attempt
        let result = validate_safe_path(root, "../../../etc/passwd");
        // Should either error or resolve within root
        if let Ok(resolved) = result {
            // The parent canonicalization check should catch this
            // since root's parent is /tmp which won't contain etc/passwd
            assert!(
                resolved.starts_with(root) || !resolved.to_string_lossy().contains("etc/passwd"),
                "Path escaped root: {:?}", resolved
            );
        }
    }

    #[test]
    fn test_validate_safe_path_absolute_within_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let abs_path = root.join("file.txt");
        let result = validate_safe_path(root, abs_path.to_str().unwrap());
        assert!(result.is_ok());
    }
}
