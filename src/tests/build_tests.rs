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

            assert!(
                result.is_ok(),
                "BuildExecutor creation should succeed: {:?}",
                result
            );
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

            assert!(
                result.is_err(),
                "BuildExecutor creation should fail when config is missing"
            );
        });
    }

    #[test]
    fn test_execute_build_integration() {
        run_in_temp_dir(|temp_path| {
            create_test_project(temp_path).expect("Failed to create test project");

            let result = execute_build_in_dir(temp_path);

            assert!(
                result.is_ok(),
                "Build execution should succeed: {:?}",
                result
            );

            // Check that build directory was created
            let build_dir = temp_path.join("build");
            assert!(
                build_dir.exists(),
                "Build directory should exist at: {:?}",
                build_dir
            );

            // Should have created at least one test combination
            let generated_files: Vec<_> = fs::read_dir(&build_dir)
                .expect("Failed to read build directory")
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .collect();

            assert!(
                !generated_files.is_empty(),
                "Should have generated at least one build combination"
            );
        });
    }

    #[test]
    fn test_execute_build_minimal_config() {
        run_in_temp_dir(|temp_path| {
            // Create minimal config with just comment
            let config_content = "# Minimal configuration";
            fs::write(temp_path.join("stackbuilder.toml"), config_content)
                .expect("Failed to write config");
            create_test_compose(&temp_path.join("components/base/docker-compose.yml"))
                .expect("Failed to create base compose");

            let result = create_build_executor_in_dir(temp_path);

            assert!(
                result.is_ok(),
                "BuildExecutor creation should succeed with minimal config: {:?}",
                result
            );

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
            fs::write(temp_path.join("stackbuilder.toml"), config_content)
                .expect("Failed to write config");
            create_test_compose(&temp_path.join("components/base/docker-compose.yml"))
                .expect("Failed to create base compose");
            // Don't create environments directory

            let result = create_build_executor_in_dir(temp_path);

            assert!(
                result.is_ok(),
                "BuildExecutor creation should succeed even without environments directory: {:?}",
                result
            );

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
            fs::write(temp_path.join("stackbuilder.toml"), config_content)
                .expect("Failed to write config");
            create_test_compose(&temp_path.join("components/base/docker-compose.yml"))
                .expect("Failed to create base compose");
            create_test_compose(
                &temp_path.join("components/extensions/monitoring/docker-compose.yml"),
            )
            .expect("Failed to create extension compose");

            let result = create_build_executor_in_dir(temp_path);

            assert!(
                result.is_ok(),
                "BuildExecutor creation should succeed for extension-only config: {:?}",
                result
            );

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
            fs::write(temp_path.join("stackbuilder.toml"), config_content)
                .expect("Failed to write config");
            create_test_compose(&temp_path.join("components/base/docker-compose.yml"))
                .expect("Failed to create base compose");
            create_test_compose(
                &temp_path.join("components/extensions/monitoring/docker-compose.yml"),
            )
            .expect("Failed to create extension compose");

            let result = create_build_executor_in_dir(temp_path);

            assert!(
                result.is_ok(),
                "BuildExecutor creation should succeed with skip_base_generation: {:?}",
                result
            );

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
            fs::write(temp_path.join("stackbuilder.toml"), config_content)
                .expect("Failed to write config");

            // Create component structure
            let base_compose = r#"
version: '3.8'
services:
  app:
    image: nginx:latest
"#;
            fs::create_dir_all(temp_path.join("components/base"))
                .expect("Failed to create base dir");
            fs::write(
                temp_path.join("components/base/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write base compose");

            // Create extensions
            fs::create_dir_all(temp_path.join("components/extensions/logging"))
                .expect("Failed to create logging dir");
            fs::write(
                temp_path.join("components/extensions/logging/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write logging compose");

            fs::create_dir_all(temp_path.join("components/extensions/oidc"))
                .expect("Failed to create oidc dir");
            fs::write(
                temp_path.join("components/extensions/oidc/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write oidc compose");

            fs::create_dir_all(temp_path.join("components/extensions/guard"))
                .expect("Failed to create guard dir");
            fs::write(
                temp_path.join("components/extensions/guard/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write guard compose");

            // Test that BuildExecutor can be created and combo is processed
            let executor =
                create_build_executor_in_dir(temp_path).expect("Failed to create build executor");

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
            fs::write(temp_path.join("stackbuilder.toml"), config_content)
                .expect("Failed to write config");

            // Create component structure
            let base_compose = r#"
version: '3.8'
services:
  app:
    image: nginx:latest
"#;
            fs::create_dir_all(temp_path.join("components/base"))
                .expect("Failed to create base dir");
            fs::write(
                temp_path.join("components/base/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write base compose");

            // Create extensions for combo
            fs::create_dir_all(temp_path.join("components/extensions/frontend"))
                .expect("Failed to create frontend dir");
            fs::write(
                temp_path.join("components/extensions/frontend/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write frontend compose");

            fs::create_dir_all(temp_path.join("components/extensions/backend"))
                .expect("Failed to create backend dir");
            fs::write(
                temp_path.join("components/extensions/backend/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write backend compose");

            // Test that BuildExecutor can be created with skip_base_generation
            let executor =
                create_build_executor_in_dir(temp_path).expect("Failed to create build executor");

            // Should have 1 env, 0 direct extensions, 1 combo
            assert_eq!(executor.num_envs, 1);
            assert_eq!(executor.num_extensions, 0);
            assert_eq!(executor.num_combos, 1);
            assert!(executor.config.build.skip_base_generation);
        });
    }

    #[test]
    fn test_per_environment_skip_base_generation() {
        run_in_temp_dir(|temp_path| {
            // Create config with per-environment skip_base_generation using new API
            let config_content = r#"
[build]
skip_base_generation = false  # Global default

[build.environments]
available = ["dev", "prod"]

[build.environments.dev]
extensions = ["monitoring"]
skip_base_generation = true   # dev skips base

[build.environments.prod]
extensions = ["monitoring"]
skip_base_generation = false  # prod includes base
"#;
            fs::write(temp_path.join("stackbuilder.toml"), config_content)
                .expect("Failed to write config");

            // Create component structure
            let base_compose = r#"
version: '3.8'
services:
  app:
    image: nginx:latest
"#;
            fs::create_dir_all(temp_path.join("components/base"))
                .expect("Failed to create base dir");
            fs::write(
                temp_path.join("components/base/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write base compose");

            // Create environments directories
            fs::create_dir_all(temp_path.join("components/environments/dev"))
                .expect("Failed to create dev env dir");
            fs::write(
                temp_path.join("components/environments/dev/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write dev compose");

            fs::create_dir_all(temp_path.join("components/environments/prod"))
                .expect("Failed to create prod env dir");
            fs::write(
                temp_path.join("components/environments/prod/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write prod compose");

            // Create monitoring extension
            fs::create_dir_all(temp_path.join("components/extensions/monitoring"))
                .expect("Failed to create monitoring dir");
            fs::write(
                temp_path.join("components/extensions/monitoring/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write monitoring compose");

            // Execute real build
            let result = execute_real_build_in_dir(temp_path);
            assert!(
                result.is_ok(),
                "Build execution should succeed: {:?}",
                result
            );

            let build_dir = temp_path.join("build");

            // dev should NOT have base subfolder (skip_base_generation = true)
            let dev_base = build_dir.join("dev/base");
            assert!(
                !dev_base.exists(),
                "dev should not have base subfolder when skip_base_generation = true"
            );

            // dev should have monitoring merged directly into environment (single variant)
            let dev_monitoring = build_dir.join("dev/docker-compose.yml");
            assert!(
                dev_monitoring.exists(),
                "dev should have monitoring merged directly into environment folder"
            );

            // prod should have base subfolder (skip_base_generation = false)
            let prod_base = build_dir.join("prod/base/docker-compose.yml");
            assert!(
                prod_base.exists(),
                "prod should have base subfolder when skip_base_generation = false"
            );

            // prod should have monitoring subfolder
            let prod_monitoring = build_dir.join("prod/monitoring/docker-compose.yml");
            assert!(
                prod_monitoring.exists(),
                "prod should have monitoring subfolder"
            );
        });
    }

    #[test]
    fn test_env_no_variants_no_base_subfolder() {
        run_in_temp_dir(|temp_path| {
            // Create config with environments that have no variants (no extensions, no combos)
            let config_content = r#"
[build]

[build.environments]
available = ["devcontainer", "forwarding", "letsencrypt"]

[build.environments.devcontainer]
combos = []

[build.environments.forwarding]
combos = []

[build.environments.letsencrypt]
combos = []
"#;
            fs::write(temp_path.join("stackbuilder.toml"), config_content)
                .expect("Failed to write config");

            // Create component structure
            let base_compose = r#"
version: '3.8'
services:
  app:
    image: nginx:latest
"#;
            fs::create_dir_all(temp_path.join("components/base"))
                .expect("Failed to create base dir");
            fs::write(
                temp_path.join("components/base/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write base compose");

            // Create environments directories
            fs::create_dir_all(temp_path.join("components/environments/devcontainer"))
                .expect("Failed to create devcontainer env dir");
            fs::write(
                temp_path.join("components/environments/devcontainer/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write devcontainer compose");

            fs::create_dir_all(temp_path.join("components/environments/forwarding"))
                .expect("Failed to create forwarding env dir");
            fs::write(
                temp_path.join("components/environments/forwarding/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write forwarding compose");

            fs::create_dir_all(temp_path.join("components/environments/letsencrypt"))
                .expect("Failed to create letsencrypt env dir");
            fs::write(
                temp_path.join("components/environments/letsencrypt/docker-compose.yml"),
                base_compose,
            )
            .expect("Failed to write letsencrypt compose");

            // Create empty extensions directory (required by the config loader)
            fs::create_dir_all(temp_path.join("components/extensions"))
                .expect("Failed to create extensions dir");

            // Execute real build
            let result = execute_real_build_in_dir(temp_path);
            assert!(
                result.is_ok(),
                "Build execution should succeed: {:?}",
                result
            );

            let build_dir = temp_path.join("build");

            // All environments should have docker-compose.yml directly in their folder, NOT in base/ subfolder
            let devcontainer_base = build_dir.join("devcontainer/base");
            assert!(
                !devcontainer_base.exists(),
                "devcontainer should NOT have base subfolder when no variants specified"
            );

            let devcontainer_direct = build_dir.join("devcontainer/docker-compose.yml");
            assert!(
                devcontainer_direct.exists(),
                "devcontainer should have docker-compose.yml directly in its folder"
            );

            let forwarding_base = build_dir.join("forwarding/base");
            assert!(
                !forwarding_base.exists(),
                "forwarding should NOT have base subfolder when no variants specified"
            );

            let forwarding_direct = build_dir.join("forwarding/docker-compose.yml");
            assert!(
                forwarding_direct.exists(),
                "forwarding should have docker-compose.yml directly in its folder"
            );

            let letsencrypt_base = build_dir.join("letsencrypt/base");
            assert!(
                !letsencrypt_base.exists(),
                "letsencrypt should NOT have base subfolder when no variants specified"
            );

            let letsencrypt_direct = build_dir.join("letsencrypt/docker-compose.yml");
            assert!(
                letsencrypt_direct.exists(),
                "letsencrypt should have docker-compose.yml directly in its folder"
            );
        });
    }
}
