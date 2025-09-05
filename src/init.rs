use std::fs;
use std::path::Path;
use clap::Parser;
use crate::config;
use crate::error::{Result, InitError, ConfigError, FileSystemError};

/// Runs the init command logic
pub fn run_init(args: &InitArgs) -> Result<()> {
    const CONFIG_FILE: &str = "stackbuilder.toml";

    // Step 1: Check if config exists
    let config_path = Path::new(CONFIG_FILE);
    let config_exists = config_path.exists();

    if !config_exists {
        // Create default config
        let default_config = config::Config::default();
        let toml_content = toml::to_string(&default_config)
            .map_err(|e| ConfigError::toml_serialize_error(e))?;
        fs::write(CONFIG_FILE, toml_content)
            .map_err(|e| FileSystemError::FileWriteFailed {
                path: config_path.to_path_buf(),
                source: e,
            })?;
        println!("Created default configuration file: {}", CONFIG_FILE);
    } else {
        if !args.force {
            println!("Configuration file already exists: {}", CONFIG_FILE);
        } else {
            println!("Overwriting existing configuration file: {}", CONFIG_FILE);
            let default_config = config::Config::default();
            let toml_content = toml::to_string(&default_config)
                .map_err(|e| ConfigError::toml_serialize_error(e))?;
            fs::write(CONFIG_FILE, toml_content)
                .map_err(|e| FileSystemError::FileWriteFailed {
                    path: config_path.to_path_buf(),
                    source: e,
                })?;
            println!("Overwrote configuration file: {}", CONFIG_FILE);
        }
    }

    // Step 2: Read the config
    let config_content = fs::read_to_string(CONFIG_FILE)
        .map_err(|e| FileSystemError::FileReadFailed {
            path: config_path.to_path_buf(),
            source: e,
        })?;
    let config: config::Config = toml::from_str(&config_content)
        .map_err(|e| ConfigError::toml_parse_error(CONFIG_FILE, e))?;
    println!("Loaded configuration from: {}", CONFIG_FILE);

    // Step 3: Create folders if not skipping
    if !args.skip_folders {
        create_folders(&config)?;
        // Step 4: Create example docker-compose.yml in base/
        create_example_compose(&config)?;
    } else {
        println!("Skipping folder creation due to --skip-folders flag");
    }

    Ok(())
}

fn create_folders(config: &config::Config) -> Result<()> {
    // Always create components_dir + base_dir
    let components_dir_path = Path::new(&config.paths.components_dir);
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

fn create_example_compose(config: &config::Config) -> Result<()> {
    let base_dir_path = Path::new(&config.paths.components_dir).join(&config.paths.base_dir);
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

#[derive(Parser)]
pub struct InitArgs {
    /// Skip creating folders, only create config
    #[arg(long)]
    pub skip_folders: bool,
    
    /// Force overwrite existing configuration file
    #[arg(long)]
    pub force: bool,
}