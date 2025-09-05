use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use toml;
use crate::error::{Result, ConfigError, ValidationError, FileSystemError};

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub paths: Paths,
    pub build: Build,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            paths: Paths::default(),
            build: Build::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Paths {
    #[serde(default = "default_components_dir")]
    pub components_dir: String,
    #[serde(default = "default_base_dir")]
    pub base_dir: String,
    #[serde(default = "default_environments_dir")]
    pub environments_dir: String,
    #[serde(default = "default_extensions_dirs")]
    pub extensions_dirs: Vec<String>,
    #[serde(default = "default_build_dir")]
    pub build_dir: String,
}

impl Default for Paths {
    fn default() -> Self {
        Paths {
            components_dir: default_components_dir(),
            base_dir: default_base_dir(),
            environments_dir: default_environments_dir(),
            extensions_dirs: default_extensions_dirs(),
            build_dir: default_build_dir(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Build {
    pub environments: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    pub combos: Option<Vec<String>>,
    pub environment: Option<Vec<EnvironmentConfig>>,
    #[serde(default = "default_copy_env_example")]
    pub copy_env_example: bool,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            environments: None,
            extensions: None,
            combos: None,
            environment: None,
            copy_env_example: default_copy_env_example(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EnvironmentConfig {
    pub name: String,
    pub extensions: Option<Vec<String>>,
}

// Default functions
fn default_components_dir() -> String {
    "./components".to_string()
}

fn default_base_dir() -> String {
    "base".to_string()
}

fn default_environments_dir() -> String {
    "environments".to_string()
}

fn default_extensions_dirs() -> Vec<String> {
    vec!["extensions".to_string()]
}

fn default_build_dir() -> String {
    "./build".to_string()
}

fn default_copy_env_example() -> bool {
    true
}

// Load and parse stackbuilder.toml configuration file
pub fn load_config() -> Result<Config> {
    let config_path = "stackbuilder.toml";
    
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ConfigError::config_not_found(config_path),
            _ => ConfigError::ConfigFileReadError {
                file: config_path.to_string(),
                source: e,
            }
        })?;

    let config: Config = toml::from_str(&content)
        .map_err(|e| ConfigError::toml_parse_error(config_path, e))?;

    Ok(config)
}

// Validate configuration: check paths existence and requirements
pub fn validate_config(config: &Config) -> Result<()> {
    println!("Validating configuration...");

    // Check required directories
    let components_path = std::path::Path::new(&config.paths.components_dir);
    if !components_path.exists() {
        return Err(ValidationError::ComponentsDirectoryNotFound {
            path: components_path.to_path_buf(),
        }.into());
    }

    let base_path = components_path.join(&config.paths.base_dir);
    if !base_path.exists() {
        return Err(ValidationError::BaseDirectoryNotFound {
            path: base_path,
        }.into());
    }

    // Check if build.targets has content, then must have environments or extensions
    let has_environments = config.build.environments.as_ref().map_or(false, |e| !e.is_empty());
    let has_global_extensions = config.build.extensions.as_ref().map_or(false, |e| !e.is_empty());
    let has_per_env_extensions = config.build.environment.as_ref().map_or(false, |envs| {
        envs.iter().any(|env| env.extensions.as_ref().map_or(false, |ext| !ext.is_empty()))
    });

    let has_targets = has_environments || has_global_extensions || has_per_env_extensions;

    if !has_targets {
        return Err(ValidationError::NoTargetsSpecified.into());
    }

    // Check environments_dir if specified and not empty
    if let Some(ref envs) = config.build.environments {
        if !envs.is_empty() {
            let envs_path = components_path.join(&config.paths.environments_dir);
            if !envs_path.exists() {
                return Err(ValidationError::DirectoryNotFound {
                    path: envs_path,
                }.into());
            }
            for env in envs {
                let env_path = envs_path.join(env);
                if !env_path.exists() {
                    return Err(ValidationError::environment_not_found(env, envs_path.clone()).into());
                }
            }
        }
    }

    // Check extensions_dirs if extensions are specified
    if has_global_extensions || has_per_env_extensions {
        for ext_dir in &config.paths.extensions_dirs {
            let ext_path = components_path.join(ext_dir);
            if !ext_path.exists() {
                return Err(ValidationError::DirectoryNotFound {
                    path: ext_path,
                }.into());
            }
        }
    }

    println!("Configuration validation passed");
    Ok(())
}

// Resolve relative paths to absolute paths
pub fn resolve_paths(config: &mut Config) -> Result<()> {
    let components_path = std::path::Path::new(&config.paths.components_dir).canonicalize()
        .map_err(|e| ValidationError::PathResolutionError {
            path: config.paths.components_dir.clone(),
            details: e.to_string(),
        })?;

    config.paths.components_dir = components_path.to_string_lossy().to_string();

    // Resolve other paths relative to components_dir
    let base_path = components_path.join(&config.paths.base_dir).canonicalize()
        .map_err(|e| ValidationError::PathResolutionError {
            path: config.paths.base_dir.clone(),
            details: e.to_string(),
        })?;
    config.paths.base_dir = base_path.to_string_lossy().to_string();

    // Only resolve environments_dir if environments are specified in build.targets
    if config.build.environments.as_ref().map_or(false, |e| !e.is_empty()) {
        let env_path = components_path.join(&config.paths.environments_dir).canonicalize()
            .map_err(|e| ValidationError::PathResolutionError {
                path: config.paths.environments_dir.clone(),
                details: e.to_string(),
            })?;
        config.paths.environments_dir = env_path.to_string_lossy().to_string();
    }

    // Only resolve extensions_dirs if extensions are specified in build.targets
    if config.build.extensions.is_some() || config.build.environment.as_ref().map_or(false, |envs| {
        envs.iter().any(|env| env.extensions.is_some())
    }) {
        let mut resolved_ext_dirs = Vec::new();
        for ext_dir in &config.paths.extensions_dirs {
            let ext_path = components_path.join(ext_dir).canonicalize()
                .map_err(|e| ValidationError::PathResolutionError {
                    path: ext_dir.clone(),
                    details: e.to_string(),
                })?;
            resolved_ext_dirs.push(ext_path.to_string_lossy().to_string());
        }
        config.paths.extensions_dirs = resolved_ext_dirs;
    }

    // Build dir will be created during build process, resolve to absolute path without requiring existence
    let build_path = std::path::Path::new(&config.paths.build_dir);
    config.paths.build_dir = build_path.canonicalize().unwrap_or_else(|_| build_path.to_path_buf()).to_string_lossy().to_string();

    println!("Paths resolved successfully");
    Ok(())
}

// Discover available environments from environments_dir
pub fn discover_environments(config: &Config) -> Result<Vec<String>> {
    let envs_path = std::path::Path::new(&config.paths.environments_dir);
    let mut environments = Vec::new();

    if envs_path.exists() {
        for entry in std::fs::read_dir(envs_path)
            .map_err(|e| FileSystemError::DirectoryReadFailed {
                path: envs_path.to_path_buf(),
                source: e,
            })? {
            let entry = entry.map_err(|e| FileSystemError::DirectoryReadFailed {
                path: envs_path.to_path_buf(),
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

// Discover available extensions from extensions_dirs
pub fn discover_extensions(config: &Config) -> Result<Vec<String>> {
    let mut extensions = Vec::new();

    for ext_dir in &config.paths.extensions_dirs {
        let ext_path = std::path::Path::new(ext_dir);
        if ext_path.exists() {
            for entry in std::fs::read_dir(ext_path)
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