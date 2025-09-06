//! Tests for build functionality

#[cfg(test)]
mod tests {
    use crate::tests::*;
    use std::fs;

    #[test]
    fn test_build_executor_creation() {
        run_in_temp_dir(|temp_path| {
            create_test_project(temp_path).expect("Failed to create test project");
            
            let result = create_build_executor_in_dir(temp_path);
            
            assert!(result.is_ok(), "BuildExecutor creation should succeed: {:?}", result);
            let executor = result.unwrap();
            assert_eq!(executor.num_envs, 2);
            assert_eq!(executor.num_extensions, 1);
        });
    }

    #[test]
    fn test_build_executor_missing_config() {
        run_in_temp_dir(|temp_path| {
            // Don't create any project files - should fail
            let result = create_build_executor_in_dir(temp_path);
            
            assert!(result.is_err(), "BuildExecutor creation should fail when config is missing");
        });
    }

    #[test]
    fn test_execute_build_integration() {
        run_in_temp_dir(|temp_path| {
            create_test_project(temp_path).expect("Failed to create test project");
            
            let result = execute_build_in_dir(temp_path);
            
            assert!(result.is_ok(), "Build execution should succeed: {:?}", result);
            
            // Check that build directory was created
            let build_dir = temp_path.join("build");
            assert!(build_dir.exists(), "Build directory should exist at: {:?}", build_dir);
            
            // Should have created at least one test combination
            let generated_files: Vec<_> = fs::read_dir(&build_dir)
                .expect("Failed to read build directory")
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .collect();
            
            assert!(!generated_files.is_empty(), "Should have generated at least one build combination");
        });
    }
}