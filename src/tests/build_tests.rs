//! Tests for build functionality

#[cfg(test)]
mod tests {
    use crate::build::*;
    use crate::tests::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_build_executor_creation() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        create_test_project(temp_dir.path()).expect("Failed to create test project");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let result = BuildExecutor::new();
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok());
        let executor = result.unwrap();
        assert_eq!(executor.num_envs, 2);
        assert_eq!(executor.num_extensions, 1);
    }

    #[test]
    fn test_build_executor_missing_config() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let result = BuildExecutor::new();
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_build_integration() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        create_test_project(temp_dir.path()).expect("Failed to create test project");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let result = execute_build();
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok());
        
        // Check that build directory was created with expected files
        let build_dir = temp_dir.path().join("build");
        assert!(build_dir.exists());
        
        // Should have created multiple combinations
        let generated_files: Vec<_> = fs::read_dir(&build_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .collect();
        
        assert!(!generated_files.is_empty());
    }
}