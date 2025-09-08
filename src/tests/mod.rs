//! Integration and unit tests for stackbuilder

pub mod config_tests;
pub mod build_tests;
pub mod merger_tests;
pub mod init_tests;
pub mod error_tests;

#[cfg(test)]
mod test_utils {
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    
    /// Create a temporary directory with test files
    pub fn create_test_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }
    
    /// Create a basic stackbuilder.toml file in the given directory
    pub fn create_test_config(dir: &Path) -> std::io::Result<()> {
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
        fs::write(dir.join("stackbuilder.toml"), config_content)
    }
    
    /// Test helper that runs code in isolated temp directory without changing global current_dir
    pub fn run_in_temp_dir<F, R>(test_fn: F) -> R
    where
        F: FnOnce(&Path) -> R + std::panic::UnwindSafe,
    {
        let temp_dir = create_test_dir();
        let temp_path = temp_dir.path();
        
        // Run test function in temp directory without changing global current_dir
        let result = std::panic::catch_unwind(|| test_fn(temp_path));
        
        // temp_dir will be cleaned up automatically when dropped
        match result {
            Ok(r) => r,
            Err(panic_info) => std::panic::resume_unwind(panic_info),
        }
    }
    
    /// Test version of run_init that works in specified directory
    #[cfg(test)]
    pub fn run_init_in_dir(args: &crate::init::InitArgs, working_dir: &Path) -> crate::error::Result<()> {
        use crate::config;
        use crate::error::{ConfigError, FileSystemError};
        use std::fs;
        
        let config_file = working_dir.join("stackbuilder.toml");
        let config_exists = config_file.exists();

        if !config_exists {
            // Create default config
            let default_config = config::Config::default();
            let toml_content = toml::to_string(&default_config)
                .map_err(ConfigError::toml_serialize_error)?;
            fs::write(&config_file, toml_content)
                .map_err(|e| FileSystemError::FileWriteFailed {
                    path: config_file.clone(),
                    source: e,
                })?;
            println!("Created default configuration file: {}", config_file.display());
        } else if !args.force {
            println!("Configuration file already exists: {}", config_file.display());
        } else {
            println!("Overwriting existing configuration file: {}", config_file.display());
            let default_config = config::Config::default();
            let toml_content = toml::to_string(&default_config)
                .map_err(ConfigError::toml_serialize_error)?;
            fs::write(&config_file, toml_content)
                .map_err(|e| FileSystemError::FileWriteFailed {
                    path: config_file.clone(),
                    source: e,
                })?;
            println!("Overwrote configuration file: {}", config_file.display());
        }

        // Step 2: Read the config
        let config_content = fs::read_to_string(&config_file)
            .map_err(|e| FileSystemError::FileReadFailed {
                path: config_file.clone(),
                source: e,
            })?;
        let config: config::Config = toml::from_str(&config_content)
            .map_err(|e| ConfigError::toml_parse_error(&config_file.display().to_string(), e))?;
        println!("Loaded configuration from: {}", config_file.display());

        // Step 3: Create folders if not skipping
        if !args.skip_folders {
            create_folders_in_dir(&config, working_dir)?;
            // Step 4: Create example docker-compose.yml in base/
            create_example_compose_in_dir(&config, working_dir)?;
        } else {
            println!("Skipping folder creation due to --skip-folders flag");
        }

        Ok(())
    }
    
    #[cfg(test)]
    fn create_folders_in_dir(config: &crate::config::Config, working_dir: &Path) -> crate::error::Result<()> {
        use crate::error::InitError;
        use std::fs;
        
        // Always create components_dir + base_dir relative to working_dir
        let components_dir_path = working_dir.join(&config.paths.components_dir);
        let base_dir_path = components_dir_path.join(&config.paths.base_dir);
        if !base_dir_path.exists() {
            fs::create_dir_all(&base_dir_path)
                .map_err(|e| InitError::ProjectStructureCreationFailed { source: e })?;
            println!("Created folder: {}", base_dir_path.display());
        } else {
            println!("Folder already exists: {}", base_dir_path.display());
        }

        // If build has environments, create components_dir + environments_dir
        if let Some(ref envs) = config.build.environments {
            if !envs.is_empty() {
                let env_dir_path = components_dir_path.join(&config.paths.environments_dir);
                if !env_dir_path.exists() {
                    fs::create_dir_all(&env_dir_path)
                        .map_err(|e| InitError::ProjectStructureCreationFailed { source: e })?;
                    println!("Created folder: {}", env_dir_path.display());
                } else {
                    println!("Folder already exists: {}", env_dir_path.display());
                }
            }
        }

        // Create folders for each extensions_dirs
        for ext_dir in &config.paths.extensions_dirs {
            let ext_dir_path = components_dir_path.join(ext_dir);
            if !ext_dir_path.exists() {
                fs::create_dir_all(&ext_dir_path)
                    .map_err(|e| InitError::ProjectStructureCreationFailed { source: e })?;
                println!("Created folder: {}", ext_dir_path.display());
            } else {
                println!("Folder already exists: {}", ext_dir_path.display());
            }
        }

        Ok(())
    }
    
    #[cfg(test)]
    fn create_example_compose_in_dir(config: &crate::config::Config, working_dir: &Path) -> crate::error::Result<()> {
        use crate::error::InitError;
        use std::fs;
        
        let components_dir_path = working_dir.join(&config.paths.components_dir);
        let base_dir_path = components_dir_path.join(&config.paths.base_dir);
        let compose_file = base_dir_path.join("docker-compose.yml");

        if compose_file.exists() {
            println!("docker-compose.yml already exists in: {}", compose_file.display());
            return Ok(());
        }

        let example_content = r#"version: '3.8'
services:
  example-service:
    image: nginx:latest
    ports:
      - "8080:80"
    environment:
      - EXAMPLE_VAR=hello
"#;

        fs::create_dir_all(&base_dir_path)
            .map_err(|e| InitError::ProjectStructureCreationFailed { source: e })?;
        fs::write(&compose_file, example_content)
            .map_err(|e| InitError::ExampleFileCreationFailed {
                details: format!("Failed to write docker-compose.yml to {}: {}", compose_file.display(), e),
            })?;
        println!("Created example docker-compose.yml in: {}", compose_file.display());

        Ok(())
    }
    
    /// Test version of load_config that works in specified directory
    #[cfg(test)]
    pub fn load_config_from_dir(working_dir: &Path) -> crate::error::Result<crate::config::Config> {
        use crate::error::ConfigError;
        use std::fs;
        
        let config_path = working_dir.join("stackbuilder.toml");
        
        let content = fs::read_to_string(&config_path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => ConfigError::config_not_found(&config_path.display().to_string()),
                _ => ConfigError::ConfigFileReadError {
                    file: config_path.display().to_string(),
                    source: e,
                }
            })?;

        let config: crate::config::Config = toml::from_str(&content)
            .map_err(|e| ConfigError::toml_parse_error(&config_path.display().to_string(), e))?;

        Ok(config)
    }
    
    /// Test version of validate_config that works with specified working directory
    #[cfg(test)]
    pub fn validate_config_in_dir(config: &crate::config::Config, working_dir: &Path) -> crate::error::Result<()> {
        use crate::error::ValidationError;
        
        println!("Validating configuration...");

        // Check required directories relative to working_dir
        let components_path = working_dir.join(&config.paths.components_dir);
        if !components_path.exists() {
            return Err(ValidationError::ComponentsDirectoryNotFound {
                path: components_path,
            }.into());
        }

        let base_path = components_path.join(&config.paths.base_dir);
        if !base_path.exists() {
            return Err(ValidationError::BaseDirectoryNotFound {
                path: base_path,
            }.into());
        }

        // Check if build configuration has valid targets
        let has_legacy_environments = config.build.environments.as_ref().is_some_and(|e| !e.is_empty());
        let has_legacy_extensions = config.build.extensions.as_ref().is_some_and(|e| !e.is_empty());
        let has_combos = !config.build.combos.is_empty();
        let has_targets = config.build.targets.is_some();

        if !has_legacy_environments && !has_legacy_extensions && !has_combos && !has_targets {
            println!("ℹ No specific targets configured - will build base configuration only");
        }

        // Validate combo definitions
        validate_combo_definitions_in_dir(config, working_dir)?;

        // Check environments_dir if specified and not empty (optional - environments can exist without specific folders)
        if let Some(ref envs) = config.build.environments {
            if !envs.is_empty() {
                let envs_path = components_path.join(&config.paths.environments_dir);
                // Environments directory is optional - it may not exist if environments are just logical names
                if envs_path.exists() {
                    for env in envs {
                        let env_path = envs_path.join(env);
                        // Individual environment directories are also optional
                        if env_path.exists() {
                            println!("✓ Found environment directory: {}", env);
                        } else {
                            println!("ℹ Environment '{}' has no specific directory (using base only)", env);
                        }
                    }
                } else {
                    println!("ℹ No environments directory found - environments will use base configuration only");
                }
            }
        }

        // Check extensions_dirs if extensions are specified (optional - extensions directories may not exist)
        if has_legacy_extensions || has_combos || has_targets {
            for ext_dir in &config.paths.extensions_dirs {
                let ext_path = components_path.join(ext_dir);
                if ext_path.exists() {
                    println!("✓ Found extensions directory: {}", ext_dir);
                } else {
                    println!("ℹ Extensions directory '{}' not found - no extensions will be available", ext_dir);
                }
            }
        }

        println!("Configuration validation passed");
        Ok(())
    }
    
    #[cfg(test)]
    fn validate_combo_definitions_in_dir(config: &crate::config::Config, working_dir: &Path) -> crate::error::Result<()> {
        use crate::error::ValidationError;
        
        let available_extensions = discover_extensions_in_dir(config, working_dir)?;
        
        for (combo_name, extensions) in &config.build.combos {
            if extensions.is_empty() {
                return Err(ValidationError::InvalidComboDefinition {
                    combo_name: combo_name.clone(),
                    details: "Combo must contain at least one extension".to_string(),
                }.into());
            }
            
            for ext in extensions {
                if !available_extensions.contains(ext) {
                    return Err(ValidationError::ExtensionNotFound {
                        name: ext.clone(),
                        available_dirs: config.paths.extensions_dirs.clone(),
                    }.into());
                }
            }
            
            println!("✓ Validated combo '{}': {:?}", combo_name, extensions);
        }
        
        Ok(())
    }
    
    /// Test version of discover_extensions that works in specified directory
    #[cfg(test)]
    pub fn discover_extensions_in_dir(config: &crate::config::Config, working_dir: &Path) -> crate::error::Result<Vec<String>> {
        use crate::error::FileSystemError;
        use std::fs;
        
        let mut extensions = Vec::new();

        for ext_dir in &config.paths.extensions_dirs {
            // Build full path: working_dir + components_dir + ext_dir
            let ext_path = working_dir.join(&config.paths.components_dir).join(ext_dir);
            
            if ext_path.exists() {
                for entry in fs::read_dir(&ext_path)
                    .map_err(|e| FileSystemError::DirectoryReadFailed {
                        path: ext_path.to_path_buf(),
                        source: e,
                    })? {
                    let entry = entry.map_err(|e| FileSystemError::DirectoryReadFailed {
                        path: ext_path.to_path_buf(),
                        source: e,
                    })?;
                    
                    if entry.path().is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            extensions.push(name.to_string());
                        }
                    }
                }
            }
        }

        println!("Discovered extensions: {:?}", extensions);
        Ok(extensions)
    }
    
    /// Test version of discover_environments that works in specified directory
    #[cfg(test)]
    pub fn discover_environments_in_dir(config: &crate::config::Config, working_dir: &Path) -> crate::error::Result<Vec<String>> {
        use crate::error::FileSystemError;
        use std::fs;
        
        let mut environments = Vec::new();
        
        let env_path = working_dir.join(&config.paths.components_dir)
            .join(&config.paths.environments_dir);
        
        if env_path.exists() {
            for entry in fs::read_dir(&env_path)
                .map_err(|e| FileSystemError::DirectoryReadFailed {
                    path: env_path.clone(),
                    source: e,
                })? {
                let entry = entry.map_err(|e| FileSystemError::DirectoryReadFailed {
                    path: env_path.clone(),
                    source: e,
                })?;
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        environments.push(name.to_string());
                    }
                }
            }
        }
        
        println!("Discovered environments: {:?}", environments);
        Ok(environments)
    }
    
    /// Test version of BuildExecutor::new that works in specified directory
    #[cfg(test)]
    pub fn create_build_executor_in_dir(working_dir: &Path) -> crate::error::Result<crate::build::BuildExecutor> {
        use crate::merger::ComposeMerger;
        use crate::yq_merger::YqMerger;
        use crate::env_merger::EnvMerger;
        
        let config = load_config_from_dir(working_dir)?;
        validate_config_in_dir(&config, working_dir)?;
        
        let _available_environments = discover_environments_in_dir(&config, working_dir)?;
        let _available_extensions = discover_extensions_in_dir(&config, working_dir)?;
        
        // Create mergers with relative paths from working_dir
        let rust_merger = ComposeMerger::new(
            format!("{}/{}", working_dir.display(), config.paths.base_dir),
            format!("{}/{}", working_dir.display(), config.paths.environments_dir),
            config.paths.extensions_dirs.iter()
                .map(|ext_dir| format!("{}/{}", working_dir.display(), ext_dir))
                .collect(),
        );
        
        let yq_merger = YqMerger::new(
            format!("{}/{}", working_dir.display(), config.paths.base_dir),
            format!("{}/{}", working_dir.display(), config.paths.environments_dir),
            config.paths.extensions_dirs.iter()
                .map(|ext_dir| format!("{}/{}", working_dir.display(), ext_dir))
                .collect(),
        );
        
        let env_merger = EnvMerger::new(
            format!("{}/{}", working_dir.display(), config.paths.base_dir),
            format!("{}/{}", working_dir.display(), config.paths.environments_dir),
            config.paths.extensions_dirs.iter()
                .map(|ext_dir| format!("{}/{}", working_dir.display(), ext_dir))
                .collect(),
        );
        
        let num_envs = config.build.environments.as_ref().map_or(0, |e| e.len());
        let num_extensions = config.build.extensions.as_ref().map_or(0, |e| e.len());
        let num_combos = config.build.combos.len();
        
        Ok(crate::build::BuildExecutor {
            config,
            rust_merger,
            yq_merger,
            env_merger,
            num_envs,
            num_extensions,
            num_combos,
        })
    }
    
    /// Test version of execute_build that works in specified directory
    #[cfg(test)]
    pub fn execute_build_in_dir(working_dir: &Path) -> crate::error::Result<()> {
        let executor = create_build_executor_in_dir(working_dir)?;
        // For testing, we just validate that the executor was created successfully
        // The actual build execution would create files, which we can test separately
        println!("Build executor created successfully with {} environments and {} extensions",
                executor.num_envs, executor.num_extensions);
        
        // Create a simple build directory to simulate build execution
        let build_dir = working_dir.join(&executor.config.paths.build_dir);
        std::fs::create_dir_all(&build_dir).map_err(|e| crate::error::FileSystemError::DirectoryCreationFailed {
            path: build_dir.clone(),
            source: e,
        })?;
        
        // Create a test build subdirectory to simulate build output
        let test_build_subdir = build_dir.join("test-combination");
        std::fs::create_dir_all(&test_build_subdir).map_err(|e| crate::error::FileSystemError::DirectoryCreationFailed {
            path: test_build_subdir,
            source: e,
        })?;
        
        Ok(())
    }

    /// Test version that performs real build execution in specified directory
    #[cfg(test)]
    pub fn execute_real_build_in_dir(working_dir: &Path) -> crate::error::Result<()> {
        use crate::build;
        
        // Save current directory
        let original_dir = std::env::current_dir().unwrap();
        
        // Change to working directory for build
        std::env::set_current_dir(working_dir).map_err(|e| crate::error::FileSystemError::DirectoryReadFailed {
            path: working_dir.to_path_buf(),
            source: e,
        })?;
        
        // Execute real build
        let result = build::execute_build();
        
        // Restore original directory
        std::env::set_current_dir(&original_dir).map_err(|e| crate::error::FileSystemError::DirectoryReadFailed {
            path: original_dir,
            source: e,
        })?;
        
        result
    }
    
    /// Create a basic docker-compose.yml file
    pub fn create_test_compose(path: &Path) -> std::io::Result<()> {
        let compose_content = r#"
version: '3.8'
services:
  test-service:
    image: nginx:alpine
    ports:
      - "8080:80"
"#;
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, compose_content)
    }
    
    /// Create a complete test project structure
    pub fn create_test_project(dir: &Path) -> std::io::Result<()> {
        create_test_config(dir)?;
        
        // Create base component
        create_test_compose(&dir.join("components/base/docker-compose.yml"))?;
        
        // Create environments
        let dev_compose = r#"
version: '3.8'
services:
  test-service:
    environment:
      - ENV=development
"#;
        fs::create_dir_all(dir.join("components/environments/dev"))?;
        fs::write(dir.join("components/environments/dev/docker-compose.yml"), dev_compose)?;
        
        let prod_compose = r#"
version: '3.8'
services:
  test-service:
    environment:
      - ENV=production
    deploy:
      replicas: 2
"#;
        fs::create_dir_all(dir.join("components/environments/prod"))?;
        fs::write(dir.join("components/environments/prod/docker-compose.yml"), prod_compose)?;
        
        // Create extension
        let monitoring_compose = r#"
version: '3.8'
services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
"#;
        fs::create_dir_all(dir.join("components/extensions/monitoring"))?;
        fs::write(dir.join("components/extensions/monitoring/docker-compose.yml"), monitoring_compose)?;
        
        Ok(())
    }
}

pub use test_utils::*;