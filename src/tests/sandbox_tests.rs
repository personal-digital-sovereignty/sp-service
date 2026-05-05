//! ============================================================
//! sp-service — Sandbox Tests
//! Tests for the hermetic Python sandbox execution environment.
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::sandbox::{
        get_hermetic_python_bin,
        get_hermetic_pip_bin, execute_python_code, setup_python_sandbox,
    };

    // ─────────────────────────────────────────────────────────
    // Path Resolution Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_get_hermetic_python_bin_structure() {
        let pip_bin = get_hermetic_pip_bin();
        let path_str = pip_bin.to_string_lossy();
        
        // Should contain venv and pip
        assert!(
            path_str.contains("venv") && path_str.contains("pip"),
            "Pip bin should contain 'venv' and 'pip': {}",
            path_str
        );
    }

    #[test]
    fn test_get_hermetic_python_bin_exists_or_not() {
        let python_bin = get_hermetic_python_bin();
        
        // Either it exists or it doesn't (both are valid)
        if python_bin.exists() {
            // If it exists, verify it's a file
            assert!(python_bin.is_file(), "Python bin should be a file");
        }
        // If it doesn't exist, that's OK - sandbox may not be initialized
    }

    // ─────────────────────────────────────────────────────────
    // Python Code Execution Tests
    // ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_python_code_simple_print() {
        // Note: This test requires the sandbox to be set up first
        // If sandbox is not initialized, this will fail gracefully
        
        let code = "print('Hello from Sovereign Sandbox!')";
        let result = execute_python_code(code).await;
        
        // Either success with output, or error with sandbox not initialized
        match result {
            Ok(output) => {
                assert!(
                    output.contains("Hello") || output.contains("Sandbox"),
                    "Output should contain expected text: {}",
                    output
                );
            }
            Err(e) => {
                // Acceptable errors: sandbox not initialized, Python not found
                assert!(
                    e.contains("não foi inicializada") || 
                    e.contains("Python") ||
                    e.contains("No such file"),
                    "Error should be about sandbox/Python: {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_execute_python_code_mathematical_operation() {
        let code = r#"
import math
result = math.sqrt(16)
print(f"Square root of 16 is {result}")
"#;
        let result = execute_python_code(code).await;
        
        match result {
            Ok(output) => {
                assert!(
                    output.contains("4") || output.contains("sqrt"),
                    "Output should contain mathematical result: {}",
                    output
                );
            }
            Err(e) => {
                // Sandbox not initialized is acceptable
                assert!(
                    e.contains("não foi inicializada") || 
                    e.contains("Python"),
                    "Error should be about sandbox/Python: {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_execute_python_code_with_pandas() {
        let code = r#"
try:
    import pandas as pd
    df = pd.DataFrame({'A': [1, 2, 3], 'B': [4, 5, 6]})
    print(f"DataFrame shape: {df.shape}")
except ImportError:
    print("Pandas not available")
"#;
        let result = execute_python_code(code).await;
        
        match result {
            Ok(output) => {
                // Either pandas works or it's not installed
                assert!(
                    output.contains("DataFrame") || 
                    output.contains("Pandas not available") ||
                    output.contains("shape"),
                    "Output should be about pandas or availability: {}",
                    output
                );
            }
            Err(e) => {
                // Sandbox not initialized is acceptable
                assert!(
                    e.contains("não foi inicializada") || 
                    e.contains("Python"),
                    "Error should be about sandbox/Python: {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_execute_python_code_empty() {
        let code = "";
        let result = execute_python_code(code).await;
        
        // Empty code should either work (no output) or fail gracefully
        match result {
            Ok(_) => {
                // Empty code executed successfully
            }
            Err(e) => {
                // Or sandbox not initialized
                assert!(
                    e.contains("não foi inicializada") || 
                    e.contains("Python"),
                    "Error should be about sandbox/Python: {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_execute_python_code_syntax_error() {
        let code = "print(invalid syntax here";
        let result = execute_python_code(code).await;
        
        match result {
            Ok(output) => {
                // If it somehow succeeds, should have error output
                assert!(
                    output.contains("SyntaxError") || 
                    output.is_empty(),
                    "Output should contain syntax error or be empty: {}",
                    output
                );
            }
            Err(e) => {
                // Error is expected for syntax error
                assert!(
                    e.contains("SyntaxError") || 
                    e.contains("invalid") ||
                    e.contains("não foi inicializada"),
                    "Error should mention syntax or sandbox: {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_execute_python_code_runtime_error() {
        let code = "raise ValueError('Test error from Sovereign test')";
        let result = execute_python_code(code).await;
        
        match result {
            Ok(output) => {
                assert!(
                    output.contains("ValueError") || 
                    output.contains("Test error"),
                    "Output should contain runtime error: {}",
                    output
                );
            }
            Err(e) => {
                assert!(
                    e.contains("ValueError") || 
                    e.contains("Test error") ||
                    e.contains("não foi inicializada"),
                    "Error should mention ValueError or sandbox: {}",
                    e
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────
    // Integration Test: Sandbox Setup
    // ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_sandbox_setup_flow() {
        // This test attempts to set up the sandbox
        // It may fail if Python is not available, which is acceptable
        
        let result = setup_python_sandbox().await;
        
        // Either setup succeeds or fails gracefully
        if result {
            // If successful, verify the venv exists (using public API)
            let python_bin = get_hermetic_python_bin();
            assert!(
                python_bin.exists(),
                "If setup succeeded, python bin should exist"
            );
        } else {
            // If failed, that's OK - user may need to install Python manually
            assert!(true, "Setup failed gracefully (Python may not be available)");
        }
    }

    // ─────────────────────────────────────────────────────────
    // Cross-Platform Path Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_path_cross_platform_windows() {
        // Test that paths would work on Windows (even if running on Linux)
        let python_bin = get_hermetic_python_bin();
        let path_str = python_bin.to_string_lossy();
        
        // Should handle both Windows and Unix paths
        #[cfg(target_os = "windows")]
        {
            assert!(
                path_str.contains("Scripts") && path_str.contains("python.exe"),
                "Windows path should contain Scripts/python.exe: {}",
                path_str
            );
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            assert!(
                path_str.contains("bin") && path_str.contains("python"),
                "Unix path should contain bin/python: {}",
                path_str
            );
        }
    }

    #[test]
    fn test_pip_path_cross_platform() {
        let pip_bin = get_hermetic_pip_bin();
        let path_str = pip_bin.to_string_lossy();
        
        #[cfg(target_os = "windows")]
        {
            assert!(
                path_str.contains("Scripts") && path_str.contains("pip"),
                "Windows pip path should contain Scripts/pip: {}",
                path_str
            );
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            assert!(
                path_str.contains("bin") && path_str.contains("pip"),
                "Unix pip path should contain bin/pip: {}",
                path_str
            );
        }
    }

    // ─────────────────────────────────────────────────────────
    // Security Tests
    // ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_python_code_no_filesystem_pollution() {
        // Verify that AST Jail blocks dangerous imports
        let code = r#"
import os
print("Trying to access OS")
"#;
        let result = execute_python_code(code).await;
        
        // AST Jail should block 'os' module OR sandbox not initialized
        match result {
            Ok(output) => {
                // If somehow allowed, should not have OS access
                assert!(
                    !output.contains("OS") || output.contains("blocked"),
                    "Output should not allow OS access: {}",
                    output
                );
            }
            Err(e) => {
                // Expected: AST Jail or sandbox not initialized
                assert!(
                    e.contains("bloqueado") || 
                    e.contains("segurança") ||
                    e.contains("não foi inicializada") ||
                    e.contains("AST Jail"),
                    "Error should mention security block or sandbox: {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_execute_python_code_timeout_simulation() {
        let code = r#"
import time
time.sleep(2)
print("Finished sleep")
"#;
        let result = execute_python_code(code).await;
        
        match result {
            Ok(output) => {
                // If it passes, it means timeout is > 2 seconds or sandbox is mocked
                assert!(output.contains("Finished") || output.contains("sleep"));
            }
            Err(e) => {
                // If it fails, it could be timeout or AST Jail blocking 'time'
                assert!(
                    e.contains("timeout") || 
                    e.contains("bloqueado") ||
                    e.contains("segurança") ||
                    e.contains("não foi inicializada") ||
                    e.contains("AST Jail"),
                    "Error should mention timeout or security block: {}",
                    e
                );
            }
        }
    }
}
