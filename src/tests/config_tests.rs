//! Tests for configuration loading and validation

#[cfg(test)]
mod tests {
    use crate::config::*;
    use crate::tests::*;
    use std::fs;

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
        run_in_temp_dir(|temp_path| {
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
            
            fs::write(temp_path.join("stackbuilder.toml"), config_content).expect("Failed to write config");
            
            let result = load_config_from_dir(temp_path);
            
            assert!(result.is_ok());
            let config = result.unwrap();
            
            assert_eq!(config.paths.components_dir, "./test-components");
            assert_eq!(config.paths.base_dir, "core");
            assert_eq!(config.paths.environments_dir, "envs");
            assert_eq!(config.paths.extensions_dirs, vec!["addons", "plugins"]);
            assert_eq!(config.paths.build_dir, "./output");
            
            assert_eq!(config.build.environments, Some(vec!["development".to_string(), "production".to_string()]));
            assert_eq!(config.build.extensions, Some(vec!["monitoring".to_string(), "auth".to_string()]));
        });
    }

    #[test]
    fn test_config_load_missing() {
        run_in_temp_dir(|temp_path| {
            let result = load_config_from_dir(temp_path);
            
            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("Configuration file") && error.to_string().contains("not found"));
        });
    }

    #[test]
    fn test_config_load_invalid_toml() {
        run_in_temp_dir(|temp_path| {
            let invalid_config = r#"
[paths
components_dir = "./components"
"#;
            
            fs::write(temp_path.join("stackbuilder.toml"), invalid_config).expect("Failed to write config");
            
            let result = load_config_from_dir(temp_path);
            
            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("Invalid TOML syntax"));
        });
    }

    #[test]
    fn test_config_validation_success() {
        run_in_temp_dir(|temp_path| {
            create_test_project(temp_path).expect("Failed to create test project");
            
            let config = load_config_from_dir(temp_path).expect("Failed to load config");
            let result = validate_config_in_dir(&config, temp_path);
            
            assert!(result.is_ok(), "Validation should succeed: {:?}", result);
        });
    }

    #[test]
    fn test_config_validation_missing_components() {
        run_in_temp_dir(|temp_path| {
            // Only create config file, don't create any directories
            let config_content = r#"
[paths]
components_dir = "./components"
base_dir = "base"
environments_dir = "environments"
extensions_dirs = ["extensions"]
build_dir = "./build"

[build]
environments = ["dev", "prod"]
extensions = ["monitoring"]
"#;
            fs::write(temp_path.join("stackbuilder.toml"), config_content).expect("Failed to write config");
            
            let config = load_config_from_dir(temp_path).expect("Failed to load config");
            let result = validate_config_in_dir(&config, temp_path);
            
            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("Components directory") || error.to_string().contains("not found"));
        });
    }

    #[test]
    fn test_config_validation_missing_base() {
        run_in_temp_dir(|temp_path| {
            create_test_config(temp_path).expect("Failed to create config");
            
            // Create components dir and extensions (so discover_extensions works) but not base subdirectory
            fs::create_dir_all(temp_path.join("components")).expect("Failed to create components dir");
            fs::create_dir_all(temp_path.join("components/extensions/monitoring")).expect("Failed to create extensions dir");
            // Explicitly don't create components/base directory
            
            let config = load_config_from_dir(temp_path).expect("Failed to load config");
            let result = validate_config_in_dir(&config, temp_path);
            
            assert!(result.is_err(), "Validation should fail when base directory is missing");
            let error = result.unwrap_err();
            assert!(error.to_string().contains("Base directory") || error.to_string().contains("base"));
        });
    }

    #[test]
    fn test_config_validation_no_targets() {
        run_in_temp_dir(|temp_path| {
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
            
            fs::write(temp_path.join("stackbuilder.toml"), config_content).expect("Failed to write config");
            create_test_compose(&temp_path.join("components/base/docker-compose.yml")).expect("Failed to create base compose");
            
            let config = load_config_from_dir(temp_path).expect("Failed to load config");
            let result = validate_config_in_dir(&config, temp_path);
            
            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("must specify at least one environment or extension"));
        });
    }

    #[test]
    fn test_discover_environments() {
        run_in_temp_dir(|temp_path| {
            create_test_project(temp_path).expect("Failed to create test project");
            
            let config = load_config_from_dir(temp_path).expect("Failed to load config");
            let environments = discover_environments_in_dir(&config, temp_path).expect("Failed to discover environments");
            
            assert_eq!(environments.len(), 2);
            assert!(environments.contains(&"dev".to_string()));
            assert!(environments.contains(&"prod".to_string()));
        });
    }

    #[test]
    fn test_discover_extensions() {
        run_in_temp_dir(|temp_path| {
            create_test_project(temp_path).expect("Failed to create test project");
            
            let config = load_config_from_dir(temp_path).expect("Failed to load config");
            let extensions = discover_extensions_in_dir(&config, temp_path).expect("Failed to discover extensions");
            
            // Extensions should be found since create_test_project creates monitoring extension
            assert_eq!(extensions.len(), 1, "Expected 1 extension, found: {:?}", extensions);
            assert!(extensions.contains(&"monitoring".to_string()));
        });
    }
}