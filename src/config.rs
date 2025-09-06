use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::{Result, ConfigError, ValidationError, FileSystemError};

/// YAML merger type configuration
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum YamlMergerType {
    /// Use external yq command (default, recommended)
    Yq,
    /// Use built-in Rust libraries (yaml-rust2 + serde_yaml)
    Rust,
}

impl Default for YamlMergerType {
    fn default() -> Self {
        YamlMergerType::Yq
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    pub paths: Paths,
    pub build: Build,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Build {
    pub environments: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    #[serde(default)]
    pub combos: HashMap<String, Vec<String>>,
    pub targets: Option<BuildTargets>,
    #[serde(default)]
    pub yaml_merger: YamlMergerType,
    #[serde(default = "default_copy_env_example")]
    pub copy_env_example: bool,
    #[serde(default = "default_copy_additional_files")]
    pub copy_additional_files: bool,
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    #[serde(default = "default_preserve_env_files")]
    pub preserve_env_files: bool,
    #[serde(default = "default_env_file_patterns")]
    pub env_file_patterns: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BuildTargets {
    pub environments: Option<Vec<String>>,
    #[serde(flatten)]
    pub environment_configs: HashMap<String, EnvironmentTarget>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EnvironmentTarget {
    pub extensions: Option<Vec<String>>,
    pub combos: Option<Vec<String>>,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            environments: None,
            extensions: None,
            combos: HashMap::new(),
            targets: None,
            yaml_merger: YamlMergerType::default(),
            copy_env_example: default_copy_env_example(),
            copy_additional_files: default_copy_additional_files(),
            exclude_patterns: default_exclude_patterns(),
            preserve_env_files: default_preserve_env_files(),
            env_file_patterns: default_env_file_patterns(),
        }
    }
}

// Legacy support for old configuration format
#[derive(Deserialize, Serialize, Debug, Clone)]
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

fn default_copy_additional_files() -> bool {
    true
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "docker-compose.yml".to_string(),
        ".env.example".to_string(),
        "*.tmp".to_string(),
        ".git*".to_string(),
        "node_modules".to_string(),
        "*.log".to_string(),
    ]
}

fn default_preserve_env_files() -> bool {
    true
}

fn default_env_file_patterns() -> Vec<String> {
    vec![
        ".env".to_string(),
        ".env.local".to_string(),
        ".env.production".to_string(),
    ]
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

    // Check if build configuration has valid targets
    let has_legacy_environments = config.build.environments.as_ref().is_some_and(|e| !e.is_empty());
    let has_legacy_extensions = config.build.extensions.as_ref().is_some_and(|e| !e.is_empty());
    let has_combos = !config.build.combos.is_empty();
    let has_targets = config.build.targets.is_some();

    if !has_legacy_environments && !has_legacy_extensions && !has_combos && !has_targets {
        return Err(ValidationError::NoTargetsSpecified.into());
    }

    // Validate combo definitions
    validate_combo_definitions(config)?;

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

    // Validate targets section if present
    if let Some(ref targets) = config.build.targets {
        validate_build_targets(config, targets)?;
    }

    // Check extensions_dirs if extensions are specified
    if has_legacy_extensions || has_combos || has_targets {
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

// Validate combo definitions
fn validate_combo_definitions(config: &Config) -> Result<()> {
    let available_extensions = discover_extensions(config)?;
    
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

// Validate build targets section
fn validate_build_targets(config: &Config, targets: &BuildTargets) -> Result<()> {
    let available_extensions = discover_extensions(config)?;
    
    // Validate target environments exist
    if let Some(ref envs) = targets.environments {
        let envs_path = std::path::Path::new(&config.paths.components_dir)
            .join(&config.paths.environments_dir);
        
        for env in envs {
            let env_path = envs_path.join(env);
            if !env_path.exists() {
                return Err(ValidationError::environment_not_found(env, envs_path.clone()).into());
            }
        }
    }
    
    // Validate each environment target configuration
    for (env_name, env_target) in &targets.environment_configs {
        // Validate extensions
        if let Some(ref extensions) = env_target.extensions {
            for ext in extensions {
                if !available_extensions.contains(ext) {
                    return Err(ValidationError::ExtensionNotFound {
                        name: ext.clone(),
                        available_dirs: config.paths.extensions_dirs.clone(),
                    }.into());
                }
            }
        }
        
        // Validate combo references
        if let Some(ref combos) = env_target.combos {
            for combo_name in combos {
                if !config.build.combos.contains_key(combo_name) {
                    return Err(ValidationError::ComboNotFound {
                        combo_name: combo_name.clone(),
                        available_combos: config.build.combos.keys().cloned().collect(),
                    }.into());
                }
            }
        }
        
        println!("✓ Validated target environment '{}' configuration", env_name);
    }
    
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
    if config.build.environments.as_ref().is_some_and(|e| !e.is_empty()) {
        let env_path = components_path.join(&config.paths.environments_dir).canonicalize()
            .map_err(|e| ValidationError::PathResolutionError {
                path: config.paths.environments_dir.clone(),
                details: e.to_string(),
            })?;
        config.paths.environments_dir = env_path.to_string_lossy().to_string();
    }

    // Only resolve extensions_dirs if extensions are specified in build.targets
    if config.build.extensions.is_some() || !config.build.combos.is_empty() || config.build.targets.is_some() {
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

// Discover available extensions from extensions_dirs
pub fn discover_extensions(config: &Config) -> Result<Vec<String>> {
    let mut extensions = Vec::new();

    for ext_dir in &config.paths.extensions_dirs {
        // Build full path: components_dir + ext_dir
        let ext_path = std::path::Path::new(&config.paths.components_dir).join(ext_dir);
        
        if ext_path.exists() {
            for entry in std::fs::read_dir(&ext_path)
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

// Resolve combo extensions into a flat list of extension names
pub fn resolve_combo_extensions(config: &Config, combo_names: &[String]) -> Result<Vec<String>> {
    let mut resolved_extensions = Vec::new();
    
    for combo_name in combo_names {
        if let Some(extensions) = config.build.combos.get(combo_name) {
            for ext in extensions {
                if !resolved_extensions.contains(ext) {
                    resolved_extensions.push(ext.clone());
                }
            }
            println!("✓ Resolved combo '{}' to extensions: {:?}", combo_name, extensions);
        } else {
            return Err(ValidationError::ComboNotFound {
                combo_name: combo_name.clone(),
                available_combos: config.build.combos.keys().cloned().collect(),
            }.into());
        }
    }
    
    Ok(resolved_extensions)
}

// Get combo names for an environment target
pub fn get_target_combo_names(env_target: &EnvironmentTarget) -> Vec<String> {
    env_target.combos.as_ref().map_or_else(Vec::new, |combos| combos.clone())
}