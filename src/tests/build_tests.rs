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

    #[test]
    fn test_execute_build_minimal_config() {
        run_in_temp_dir(|temp_path| {
            // Create minimal config with just comment
            let config_content = "# Minimal configuration";
            fs::write(temp_path.join("stackbuilder.toml"), config_content).expect("Failed to write config");
            create_test_compose(&temp_path.join("components/base/docker-compose.yml")).expect("Failed to create base compose");
            
            let result = create_build_executor_in_dir(temp_path);
            
            assert!(result.is_ok(), "BuildExecutor creation should succeed with minimal config: {:?}", result);
            
            let executor = result.unwrap();
            assert_eq!(executor.num_envs, 0);
            assert_eq!(executor.num_extensions, 0);
        });
    }

    #[test]
    fn test_execute_build_env_no_folder() {
        run_in_temp_dir(|temp_path| {
            let config_content = r#"
[build]
environments = ["prod"]
"#;
            fs::write(temp_path.join("stackbuilder.toml"), config_content).expect("Failed to write config");
            create_test_compose(&temp_path.join("components/base/docker-compose.yml")).expect("Failed to create base compose");
            // Don't create environments directory
            
            let result = create_build_executor_in_dir(temp_path);
            
            assert!(result.is_ok(), "BuildExecutor creation should succeed even without environments directory: {:?}", result);
            
            let executor = result.unwrap();
            assert_eq!(executor.num_envs, 1);
            assert_eq!(executor.num_extensions, 0);
        });
    }

    #[test]
    fn test_execute_build_ext_only() {
        run_in_temp_dir(|temp_path| {
            let config_content = r#"
[build]
extensions = ["monitoring"]
"#;
            fs::write(temp_path.join("stackbuilder.toml"), config_content).expect("Failed to write config");
            create_test_compose(&temp_path.join("components/base/docker-compose.yml")).expect("Failed to create base compose");
            create_test_compose(&temp_path.join("components/extensions/monitoring/docker-compose.yml")).expect("Failed to create extension compose");
            
            let result = create_build_executor_in_dir(temp_path);
            
            assert!(result.is_ok(), "BuildExecutor creation should succeed for extension-only config: {:?}", result);
            
            let executor = result.unwrap();
            assert_eq!(executor.num_envs, 0);
            assert_eq!(executor.num_extensions, 1);
        });
    }

    #[test]
    fn test_build_skip_base_generation() {
        run_in_temp_dir(|temp_path| {
            let config_content = r#"
[build]
environments = ["dev"]
extensions = ["monitoring"]
skip_base_generation = true
"#;
            fs::write(temp_path.join("stackbuilder.toml"), config_content).expect("Failed to write config");
            create_test_compose(&temp_path.join("components/base/docker-compose.yml")).expect("Failed to create base compose");
            create_test_compose(&temp_path.join("components/extensions/monitoring/docker-compose.yml")).expect("Failed to create extension compose");
            
            let result = create_build_executor_in_dir(temp_path);
            
            assert!(result.is_ok(), "BuildExecutor creation should succeed with skip_base_generation: {:?}", result);
            
            let executor = result.unwrap();
            assert_eq!(executor.num_envs, 1);
            assert_eq!(executor.num_extensions, 1);
            assert!(executor.config.build.skip_base_generation);
        });
    }

    #[test]
    fn test_legacy_combo_support() {
        run_in_temp_dir(|temp_path| {
            // Create config with combos in legacy format
            let config_content = r#"
[build]
environments = ["dev"]
extensions = ["logging"]
combos = { security = ["oidc", "guard"] }
"#;
            fs::write(temp_path.join("stackbuilder.toml"), config_content).expect("Failed to write config");

            // Create component structure
            let base_compose = r#"
version: '3.8'
services:
  app:
    image: nginx:latest
"#;
            fs::create_dir_all(temp_path.join("components/base")).expect("Failed to create base dir");
            fs::write(temp_path.join("components/base/docker-compose.yml"), base_compose).expect("Failed to write base compose");

            // Create extensions
            fs::create_dir_all(temp_path.join("components/extensions/logging")).expect("Failed to create logging dir");
            fs::write(temp_path.join("components/extensions/logging/docker-compose.yml"), base_compose).expect("Failed to write logging compose");
            
            fs::create_dir_all(temp_path.join("components/extensions/oidc")).expect("Failed to create oidc dir");
            fs::write(temp_path.join("components/extensions/oidc/docker-compose.yml"), base_compose).expect("Failed to write oidc compose");
            
            fs::create_dir_all(temp_path.join("components/extensions/guard")).expect("Failed to create guard dir");
            fs::write(temp_path.join("components/extensions/guard/docker-compose.yml"), base_compose).expect("Failed to write guard compose");

            // Test that BuildExecutor can be created and combo is processed
            let executor = create_build_executor_in_dir(temp_path).expect("Failed to create build executor");
            
            // Should have 1 env, 1 extension, 1 combo
            assert_eq!(executor.num_envs, 1);
            assert_eq!(executor.num_extensions, 1);
            assert_eq!(executor.num_combos, 1);
        });
    }

    #[test]
    fn test_combo_only_with_skip_base() {
        run_in_temp_dir(|temp_path| {
            // Create config with combo only + skip_base_generation
            let config_content = r#"
[build]
environments = ["prod"]
skip_base_generation = true
combos = { fullstack = ["frontend", "backend"] }
"#;
            fs::write(temp_path.join("stackbuilder.toml"), config_content).expect("Failed to write config");

            // Create component structure
            let base_compose = r#"
version: '3.8'
services:
  app:
    image: nginx:latest
"#;
            fs::create_dir_all(temp_path.join("components/base")).expect("Failed to create base dir");
            fs::write(temp_path.join("components/base/docker-compose.yml"), base_compose).expect("Failed to write base compose");

            // Create extensions for combo
            fs::create_dir_all(temp_path.join("components/extensions/frontend")).expect("Failed to create frontend dir");
            fs::write(temp_path.join("components/extensions/frontend/docker-compose.yml"), base_compose).expect("Failed to write frontend compose");
            
            fs::create_dir_all(temp_path.join("components/extensions/backend")).expect("Failed to create backend dir");
            fs::write(temp_path.join("components/extensions/backend/docker-compose.yml"), base_compose).expect("Failed to write backend compose");

            // Test that BuildExecutor can be created with skip_base_generation
            let executor = create_build_executor_in_dir(temp_path).expect("Failed to create build executor");
            
            // Should have 1 env, 0 direct extensions, 1 combo
            assert_eq!(executor.num_envs, 1);
            assert_eq!(executor.num_extensions, 0);
            assert_eq!(executor.num_combos, 1);
            assert!(executor.config.build.skip_base_generation);
        });
    }
}