//! Tests for init functionality

#[cfg(test)]
mod tests {
    use crate::init::*;
    use crate::tests::*;
    use std::fs;

    #[test]
    fn test_init_in_empty_directory() {
        run_in_temp_dir(|temp_path| {
            let args = InitArgs {
                skip_folders: false,
                force: false,
            };
            
            let result = run_init_in_dir(&args, temp_path);
            
            assert!(result.is_ok(), "Init should succeed in empty directory: {:?}", result);
            
            // Check that config file was created
            let config_file = temp_path.join("stackbuilder.toml");
            assert!(config_file.exists(), "Config file should exist at: {:?}", config_file);
            
            // Check that folder structure was created
            let components_dir = temp_path.join("components");
            assert!(components_dir.exists(), "Components directory should exist at: {:?}", components_dir);
            
            let base_dir = temp_path.join("components/base");
            assert!(base_dir.exists(), "Base directory should exist at: {:?}", base_dir);
            
            let extensions_dir = temp_path.join("components/extensions");
            assert!(extensions_dir.exists(), "Extensions directory should exist at: {:?}", extensions_dir);
            
            // Check that example compose file was created
            let compose_file = temp_path.join("components/base/docker-compose.yml");
            assert!(compose_file.exists(), "Compose file should exist at: {:?}", compose_file);
        });
    }

    #[test]
    fn test_init_skip_folders() {
        run_in_temp_dir(|temp_path| {
            let args = InitArgs {
                skip_folders: true,
                force: false,
            };
            
            let result = run_init_in_dir(&args, temp_path);
            
            assert!(result.is_ok(), "Init should succeed with skip_folders: {:?}", result);
            
            // Check that config file was created
            let config_file = temp_path.join("stackbuilder.toml");
            assert!(config_file.exists(), "Config file should exist at: {:?}", config_file);
            
            // Check that folders were NOT created
            let components_dir = temp_path.join("components");
            assert!(!components_dir.exists(), "Components directory should NOT exist when skip_folders=true at: {:?}", components_dir);
        });
    }

    #[test]
    fn test_init_existing_config_no_force() {
        run_in_temp_dir(|temp_path| {
            // Create existing valid config
            let existing_config = r#"
[paths]
components_dir = "./existing-components"
base_dir = "existing-base"
environments_dir = "existing-envs"
extensions_dirs = ["existing-ext"]
build_dir = "./existing-build"

[build]
environments = ["existing-env"]
"#;
            let config_file = temp_path.join("stackbuilder.toml");
            fs::write(&config_file, existing_config).expect("Failed to write existing config");
            let original_content = fs::read_to_string(&config_file).expect("Failed to read config");
            
            let args = InitArgs {
                skip_folders: false,
                force: false,
            };
            
            let result = run_init_in_dir(&args, temp_path);
            
            // Init should succeed even when config exists and force=false (it just skips overwriting)
            assert!(result.is_ok(), "Init should succeed when config exists and force=false: {:?}", result);
            
            // Config should NOT be overwritten when force=false
            let current_content = fs::read_to_string(&config_file).expect("Failed to read updated config");
            assert_eq!(current_content, original_content);
            
            // But folders should be created according to the existing config
            let existing_components_dir = temp_path.join("existing-components");
            assert!(existing_components_dir.exists(), "Existing components directory should be created: {:?}", existing_components_dir);
        });
    }

    #[test]
    fn test_init_existing_config_with_force() {
        run_in_temp_dir(|temp_path| {
            // Create existing valid config
            let existing_config = r#"
[paths]
components_dir = "./existing-components"
base_dir = "existing-base"
environments_dir = "existing-envs"
extensions_dirs = ["existing-ext"]
build_dir = "./existing-build"

[build]
environments = ["existing-env"]
"#;
            let config_file = temp_path.join("stackbuilder.toml");
            fs::write(&config_file, existing_config).expect("Failed to write existing config");
            let original_content = fs::read_to_string(&config_file).expect("Failed to read config");
            
            let args = InitArgs {
                skip_folders: false,
                force: true,
            };
            
            let result = run_init_in_dir(&args, temp_path);
            
            assert!(result.is_ok(), "Init should succeed with force=true: {:?}", result);
            
            // Config should be overwritten
            let current_content = fs::read_to_string(&config_file).expect("Failed to read updated config");
            assert_ne!(current_content, original_content);
            assert!(current_content.contains("[paths]"));
        });
    }
}