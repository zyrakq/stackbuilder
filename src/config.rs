use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::{Result, ConfigError, ValidationError, FileSystemError};

/// YAML merger type configuration
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum YamlMergerType {
    /// Use external yq command (default, recommended)
    #[default]
    Yq,
    /// Use built-in Rust libraries (yaml-rust2 + serde_yaml_ng)
    Rust,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub paths: Paths,
    #[serde(default)]
    pub build: BuildConfig,
}

// Use custom deserializer to handle both APIs
#[derive(Serialize, Debug, Clone)]
pub struct BuildConfig {
    // All fields unified
    pub environments: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    pub combos: HashMap<String, Vec<String>>,
    pub targets: Option<BuildTargets>,
    pub environments_config: Option<BuildEnvironments>,
    pub yaml_merger: YamlMergerType,
    pub copy_env_example: bool,
    pub copy_additional_files: bool,
    pub exclude_patterns: Vec<String>,
    pub preserve_env_files: bool,
    pub env_file_patterns: Vec<String>,
    pub backup_dir: String,
    pub skip_base_generation: bool,
}

impl<'de> Deserialize<'de> for BuildConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct BuildConfigVisitor;

        impl<'de> Visitor<'de> for BuildConfigVisitor {
            type Value = BuildConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a build configuration")
            }

            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut environments: Option<serde_json::Value> = None;
                let mut extensions: Option<Vec<String>> = None;
                let mut combos: HashMap<String, Vec<String>> = HashMap::new();
                let mut targets: Option<BuildTargets> = None;
                let mut yaml_merger: Option<YamlMergerType> = None;
                let mut copy_env_example: Option<bool> = None;
                let mut copy_additional_files: Option<bool> = None;
                let mut exclude_patterns: Option<Vec<String>> = None;
                let mut preserve_env_files: Option<bool> = None;
                let mut env_file_patterns: Option<Vec<String>> = None;
                let mut backup_dir: Option<String> = None;
                let mut skip_base_generation: Option<bool> = None;

                while let Some(key) = map.next_key::<String>().map_err(serde::de::Error::custom)? {
                    match key.as_str() {
                        "environments" => {
                            environments = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "extensions" => {
                            extensions = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "combos" => {
                            combos = map.next_value().map_err(serde::de::Error::custom)?;
                        }
                        "targets" => {
                            targets = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "yaml_merger" => {
                            yaml_merger = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "copy_env_example" => {
                            copy_env_example = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "copy_additional_files" => {
                            copy_additional_files = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "exclude_patterns" => {
                            exclude_patterns = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "preserve_env_files" => {
                            preserve_env_files = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "env_file_patterns" => {
                            env_file_patterns = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "backup_dir" => {
                            backup_dir = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        "skip_base_generation" => {
                            skip_base_generation = Some(map.next_value().map_err(serde::de::Error::custom)?);
                        }
                        _ => {
                            // Skip unknown fields
                            let _: serde_json::Value = map.next_value().map_err(serde::de::Error::custom)?;
                        }
                    }
                }

                // Determine API type and parse environments accordingly
                let (final_environments, environments_config) = if let Some(env_value) = environments {
                    if env_value.is_object() {
                        // New API: environments is an object with "available" and environment configs
                        let env_config: BuildEnvironments = serde_json::from_value(env_value)
                            .map_err(serde::de::Error::custom)?;
                        (None, Some(env_config))
                    } else if env_value.is_array() {
                        // Legacy API: environments is an array
                        let env_list: Vec<String> = serde_json::from_value(env_value)
                            .map_err(serde::de::Error::custom)?;
                        (Some(env_list), None)
                    } else {
                        return Err(serde::de::Error::custom("environments must be either an array or an object"));
                    }
                } else {
                    (None, None)
                };

                Ok(BuildConfig {
                    environments: final_environments,
                    extensions,
                    combos,
                    targets,
                    environments_config,
                    yaml_merger: yaml_merger.unwrap_or_default(),
                    copy_env_example: copy_env_example.unwrap_or_else(default_copy_env_example),
                    copy_additional_files: copy_additional_files.unwrap_or_else(default_copy_additional_files),
                    exclude_patterns: exclude_patterns.unwrap_or_else(default_exclude_patterns),
                    preserve_env_files: preserve_env_files.unwrap_or_else(default_preserve_env_files),
                    env_file_patterns: env_file_patterns.unwrap_or_else(default_env_file_patterns),
                    backup_dir: backup_dir.unwrap_or_else(default_backup_dir),
                    skip_base_generation: skip_base_generation.unwrap_or_else(default_skip_base_generation),
                })
            }
        }

        deserializer.deserialize_map(BuildConfigVisitor)
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        BuildConfig {
            environments: None,
            extensions: None,
            combos: HashMap::new(),
            targets: None,
            environments_config: None,
            yaml_merger: YamlMergerType::default(),
            copy_env_example: default_copy_env_example(),
            copy_additional_files: default_copy_additional_files(),
            exclude_patterns: default_exclude_patterns(),
            preserve_env_files: default_preserve_env_files(),
            env_file_patterns: default_env_file_patterns(),
            backup_dir: default_backup_dir(),
            skip_base_generation: default_skip_base_generation(),
        }
    }
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
    // Legacy field for backwards compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environments: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    #[serde(default)]
    pub combos: HashMap<String, Vec<String>>,
    // Legacy field for backwards compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(default = "default_backup_dir")]
    pub backup_dir: String,
    #[serde(default = "default_skip_base_generation")]
    pub skip_base_generation: bool,
}

// New environments structure
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BuildEnvironments {
    pub available: Option<Vec<String>>,
    #[serde(flatten)]
    pub environment_configs: HashMap<String, EnvironmentConfig>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EnvironmentConfig {
    pub extensions: Option<Vec<String>>,
    pub combos: Option<Vec<String>>,
    pub skip_base_generation: Option<bool>,
}

// Legacy structure for backwards compatibility
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BuildTargets {
    #[serde(flatten)]
    pub environment_configs: HashMap<String, EnvironmentTarget>,
}

// Legacy structure for backwards compatibility
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EnvironmentTarget {
    pub extensions: Option<Vec<String>>,
    pub combos: Option<Vec<String>>,
    pub skip_base_generation: Option<bool>,
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
            backup_dir: default_backup_dir(),
            skip_base_generation: default_skip_base_generation(),
        }
    }
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

fn default_backup_dir() -> String {
    "./.stackbuilder/backup".to_string()
}

fn default_skip_base_generation() -> bool {
    false
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
    let environments_list = get_environments_list(config);
    let has_environments = !environments_list.is_empty();
    let has_legacy_extensions = config.build.extensions.as_ref().is_some_and(|e| !e.is_empty());
    let has_combos = !config.build.combos.is_empty();
    let has_targets = config.build.targets.is_some() || config.build.environments_config.is_some();

    if !has_environments && !has_legacy_extensions && !has_combos && !has_targets {
        println!("ℹ No specific targets configured - will build base configuration only");
    }

    // Validate combo definitions
    validate_combo_definitions(config)?;

    // Check environments_dir if specified and not empty (optional - environments can exist without specific folders)
    let environments_list = get_environments_list(config);
    if !environments_list.is_empty() {
        let envs_path = components_path.join(&config.paths.environments_dir);
        // Environments directory is optional - it may not exist if environments are just logical names
        if envs_path.exists() {
            for env in &environments_list {
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

    // Validate targets section if present (legacy API)
    if let Some(ref targets) = config.build.targets {
        validate_build_targets(config, targets)?;
    }
    
    // Validate new environments configuration if present
    if let Some(ref env_config) = config.build.environments_config {
        validate_build_environments(config, env_config)?;
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

// Validate build targets section (legacy)
fn validate_build_targets(config: &Config, targets: &BuildTargets) -> Result<()> {
    let available_extensions = discover_extensions(config)?;
    
    // Validate target environments from global config (targets no longer have environments field)
    let environments_list = get_environments_list(config);
    if !environments_list.is_empty() {
        let envs_path = std::path::Path::new(&config.paths.components_dir)
            .join(&config.paths.environments_dir);
        
        // Environments directory and individual environment folders are optional
        if envs_path.exists() {
            for env in &environments_list {
                let env_path = envs_path.join(env);
                if env_path.exists() {
                    println!("✓ Found target environment directory: {}", env);
                } else {
                    println!("ℹ Target environment '{}' has no specific directory (using base only)", env);
                }
            }
        } else {
            println!("ℹ No environments directory found for targets - environments will use base configuration only");
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

// Validate new build environments section
fn validate_build_environments(config: &Config, env_config: &BuildEnvironments) -> Result<()> {
    let available_extensions = discover_extensions(config)?;
    
    // Validate environments from available list
    if let Some(ref available_envs) = env_config.available {
        let envs_path = std::path::Path::new(&config.paths.components_dir)
            .join(&config.paths.environments_dir);
        
        // Environments directory and individual environment folders are optional
        if envs_path.exists() {
            for env in available_envs {
                let env_path = envs_path.join(env);
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
    
    // Validate each environment configuration
    for (env_name, env_cfg) in &env_config.environment_configs {
        // Validate extensions
        if let Some(ref extensions) = env_cfg.extensions {
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
        if let Some(ref combos) = env_cfg.combos {
            for combo_name in combos {
                if !config.build.combos.contains_key(combo_name) {
                    return Err(ValidationError::ComboNotFound {
                        combo_name: combo_name.clone(),
                        available_combos: config.build.combos.keys().cloned().collect(),
                    }.into());
                }
            }
        }
        
        println!("✓ Validated environment '{}' configuration", env_name);
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

    // Only resolve environments_dir if environments are specified and directory exists
    let environments_list = get_environments_list(config);
    if !environments_list.is_empty() {
        let env_path = components_path.join(&config.paths.environments_dir);
        if env_path.exists() {
            let resolved_env_path = env_path.canonicalize()
                .map_err(|e| ValidationError::PathResolutionError {
                    path: config.paths.environments_dir.clone(),
                    details: e.to_string(),
                })?;
            config.paths.environments_dir = resolved_env_path.to_string_lossy().to_string();
        } else {
            // Keep relative path if directory doesn't exist - it's optional
            config.paths.environments_dir = env_path.to_string_lossy().to_string();
        }
    }

    // Only resolve extensions_dirs if extensions are specified in build configuration
    if config.build.extensions.is_some() || !config.build.combos.is_empty() ||
       config.build.targets.is_some() || config.build.environments_config.is_some() {
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

/// Get environments list from configuration (new API first, then legacy fallback)
pub fn get_environments_list(config: &Config) -> Vec<String> {
    // Try new API first
    if let Some(ref env_config) = config.build.environments_config {
        if let Some(ref available) = env_config.available {
            return available.clone();
        }
    }
    
    // Fallback to legacy API
    config.build.environments.as_ref().map_or_else(Vec::new, |v| v.clone())
}

/// Get environment-specific configuration (new API first, then legacy fallback)
pub fn get_environment_config(config: &Config, env_name: &str) -> Option<EnvironmentConfig> {
    // Try new API first
    if let Some(ref env_config) = config.build.environments_config {
        if let Some(env_cfg) = env_config.environment_configs.get(env_name) {
            return Some(env_cfg.clone());
        }
    }
    
    // Fallback to legacy API
    if let Some(ref targets) = config.build.targets {
        if let Some(legacy_target) = targets.environment_configs.get(env_name) {
            return Some(EnvironmentConfig {
                extensions: legacy_target.extensions.clone(),
                combos: legacy_target.combos.clone(),
                skip_base_generation: legacy_target.skip_base_generation,
            });
        }
    }
    
    None
}

/// Check if new environments API is being used
pub fn is_using_new_environments_api(config: &Config) -> bool {
    config.build.environments_config.is_some()
}
