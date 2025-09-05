//! Tests for configuration loading and validation

#[cfg(test)]
mod tests {
    use crate::config::*;
    use crate::tests::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        
        assert_eq!(config.paths.components_dir, "./components");
        assert_eq!(config.paths.base_dir, "base");
        assert_eq!(config.paths.environments_dir, "environments");
        assert_eq!(config.paths.extensions_dirs, vec!["extensions"]);
        assert_eq!(config.paths.build_dir, "./build");
    }

    #[test]
    fn test_config_load_valid() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("stackbuilder.toml");
        
        let config_content = r#"
[paths]
components_dir = "./test-components"
base_dir = "core"
environments_dir = "envs"
extensions_dirs = ["addons", "plugins"]
build_dir = "./output"

[build]
environments = ["development", "production"]
extensions = ["monitoring", "auth"]
"#;
        
        fs::write(&config_path, config_content).expect("Failed to write config");
        
        // Change to temp directory to test relative path loading
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let result = load_config();
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok());
        let config = result.unwrap();
        
        assert_eq!(config.paths.components_dir, "./test-components");
        assert_eq!(config.paths.base_dir, "core");
        assert_eq!(config.paths.environments_dir, "envs");
        assert_eq!(config.paths.extensions_dirs, vec!["addons", "plugins"]);
        assert_eq!(config.paths.build_dir, "./output");
        
        assert_eq!(config.build.environments, Some(vec!["development".to_string(), "production".to_string()]));
        assert_eq!(config.build.extensions, Some(vec!["monitoring".to_string(), "auth".to_string()]));
    }

    #[test]
    fn test_config_load_missing() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let result = load_config();
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Configuration file 'stackbuilder.toml' not found"));
    }

    #[test]
    fn test_config_load_invalid_toml() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("stackbuilder.toml");
        
        let invalid_config = r#"
[paths
components_dir = "./components"
"#;
        
        fs::write(&config_path, invalid_config).expect("Failed to write config");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let result = load_config();
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid TOML syntax"));
    }

    #[test]
    fn test_config_validation_success() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        create_test_project(temp_dir.path()).expect("Failed to create test project");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let config = load_config().expect("Failed to load config");
        let result = validate_config(&config);
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_validation_missing_components() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        create_test_config(temp_dir.path()).expect("Failed to create config");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let config = load_config().expect("Failed to load config");
        let result = validate_config(&config);
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Components directory"));
    }

    #[test]
    fn test_config_validation_missing_base() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        create_test_config(temp_dir.path()).expect("Failed to create config");
        
        // Create components dir but not base
        fs::create_dir_all(temp_dir.path().join("components")).expect("Failed to create components dir");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let config = load_config().expect("Failed to load config");
        let result = validate_config(&config);
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Base directory"));
    }

    #[test]
    fn test_config_validation_no_targets() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("stackbuilder.toml");
        
        let config_content = r#"
[paths]
components_dir = "./components"
base_dir = "base"
environments_dir = "environments"
extensions_dirs = ["extensions"]
build_dir = "./build"

[build]
environments = []
extensions = []
"#;
        
        fs::write(&config_path, config_content).expect("Failed to write config");
        create_test_compose(&temp_dir.path().join("components/base/docker-compose.yml")).expect("Failed to create base compose");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let config = load_config().expect("Failed to load config");
        let result = validate_config(&config);
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("must specify at least one environment or extension"));
    }

    #[test]
    fn test_discover_environments() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        create_test_project(temp_dir.path()).expect("Failed to create test project");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let mut config = load_config().expect("Failed to load config");
        resolve_paths(&mut config).expect("Failed to resolve paths");
        
        let environments = discover_environments(&config).expect("Failed to discover environments");
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert_eq!(environments.len(), 2);
        assert!(environments.contains(&"dev".to_string()));
        assert!(environments.contains(&"prod".to_string()));
    }

    #[test]
    fn test_discover_extensions() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        create_test_project(temp_dir.path()).expect("Failed to create test project");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let mut config = load_config().expect("Failed to load config");
        resolve_paths(&mut config).expect("Failed to resolve paths");
        
        let extensions = discover_extensions(&config).expect("Failed to discover extensions");
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert_eq!(extensions.len(), 1);
        assert!(extensions.contains(&"monitoring".to_string()));
    }
}