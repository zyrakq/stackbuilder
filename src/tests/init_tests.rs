//! Tests for init functionality

#[cfg(test)]
mod tests {
    use crate::init::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_init_in_empty_directory() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let args = InitArgs {
            skip_folders: false,
            force: false,
        };
        
        let result = run_init(&args);
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok());
        
        // Check that config file was created
        assert!(temp_dir.path().join("stackbuilder.toml").exists());
        
        // Check that folder structure was created
        assert!(temp_dir.path().join("components").exists());
        assert!(temp_dir.path().join("components/base").exists());
        assert!(temp_dir.path().join("components/extensions").exists());
        
        // Check that example compose file was created
        assert!(temp_dir.path().join("components/base/docker-compose.yml").exists());
    }

    #[test]
    fn test_init_skip_folders() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let args = InitArgs {
            skip_folders: true,
            force: false,
        };
        
        let result = run_init(&args);
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok());
        
        // Check that config file was created
        assert!(temp_dir.path().join("stackbuilder.toml").exists());
        
        // Check that folders were NOT created
        assert!(!temp_dir.path().join("components").exists());
    }

    #[test]
    fn test_init_existing_config_no_force() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("stackbuilder.toml");
        
        // Create existing config
        fs::write(&config_path, "# existing config").expect("Failed to write existing config");
        let original_content = fs::read_to_string(&config_path).unwrap();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let args = InitArgs {
            skip_folders: false,
            force: false,
        };
        
        let result = run_init(&args);
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok());
        
        // Config should not be overwritten
        let current_content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(current_content, original_content);
    }

    #[test]
    fn test_init_existing_config_with_force() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("stackbuilder.toml");
        
        // Create existing config
        fs::write(&config_path, "# existing config").expect("Failed to write existing config");
        let original_content = fs::read_to_string(&config_path).unwrap();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let args = InitArgs {
            skip_folders: false,
            force: true,
        };
        
        let result = run_init(&args);
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok());
        
        // Config should be overwritten
        let current_content = fs::read_to_string(&config_path).unwrap();
        assert_ne!(current_content, original_content);
        assert!(current_content.contains("[paths]"));
    }
}