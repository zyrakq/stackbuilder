use std::fs;
use std::path::Path;

use crate::config::{self, YamlMergerType};
use crate::merger::{ComposeMerger, merge_compose_files};
use crate::yq_merger::{YqMerger, yq_merge_compose_files, check_yq_availability};
use crate::env_merger::{EnvMerger, merge_env_files, write_merged_env};
use crate::file_copier::FileCopier;
use crate::build_cleaner::BuildCleaner;
use crate::error::{Result, BuildError, FileSystemError, YamlError, ValidationError};

/// Structure for managing build process execution
#[derive(Debug)]
pub struct BuildExecutor {
    pub config: config::Config,
    pub rust_merger: ComposeMerger,
    pub yq_merger: YqMerger,
    pub env_merger: EnvMerger,
    pub num_envs: usize,
    pub num_extensions: usize,
}

impl BuildExecutor {
    /// Create new BuildExecutor with loaded configuration
    pub fn new() -> Result<Self> {
        let mut config = config::load_config()?;
        config::resolve_paths(&mut config)?;
        config::validate_config(&config)?;

        // Check yq availability only if yq merger is configured
        if config.build.yaml_merger == YamlMergerType::Yq {
            check_yq_availability()
                .map_err(|_| BuildError::BuildProcessFailed {
                    details: "yq is required but not available. Please either:\n\
                        1. Install yq v4+ from https://github.com/mikefarah/yq\n\
                        2. Or set yaml_merger = \"rust\" in your stackbuilder.toml config file\n\n\
                        Installation options:\n\
                        - Ubuntu/Debian: sudo apt install yq\n\
                        - macOS: brew install yq\n\
                        - Binary: wget https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -O /usr/bin/yq && chmod +x /usr/bin/yq".to_string(),
                })?;
        }

        let rust_merger = ComposeMerger::new(
            config.paths.base_dir.clone(),
            config.paths.environments_dir.clone(),
            config.paths.extensions_dirs.clone(),
        );

        let yq_merger = YqMerger::new(
            config.paths.base_dir.clone(),
            config.paths.environments_dir.clone(),
            config.paths.extensions_dirs.clone(),
        );

        let env_merger = EnvMerger::new(
            config.paths.base_dir.clone(),
            config.paths.environments_dir.clone(),
            config.paths.extensions_dirs.clone(),
        );

        let num_envs = config.build.environments.as_ref().map_or(0, |e| e.len());
        let num_extensions = config.build.extensions.as_ref().map_or(0, |e| e.len());

        Ok(Self { config, rust_merger, yq_merger, env_merger, num_envs, num_extensions })
    }
}

/// Main build execution function
pub fn execute_build() -> Result<()> {
    println!("Starting build process...");

    let executor = BuildExecutor::new()
        .map_err(|e| BuildError::BuildProcessFailed {
            details: format!("Failed to initialize build executor: {}", e),
        })?;
    println!("Configuration loaded and validated");

    let combinations = determine_build_combinations(&executor.config)?;
    println!("Determined {} build combinations", combinations.len());

    if combinations.is_empty() {
        return Err(BuildError::BuildProcessFailed {
            details: "No valid build combinations found".to_string(),
        }.into());
    }

    create_build_structure(&executor, &combinations)?;

    println!("Build process completed successfully");
    Ok(())
}

/// Determine all build combinations based on configuration
fn determine_build_combinations(config: &config::Config) -> Result<Vec<BuildCombination>> {
    let combinations = if let Some(ref targets) = config.build.targets {
        resolve_target_combinations(config, targets)?
    } else {
        // Legacy mode: use old logic for backwards compatibility
        resolve_legacy_combinations(config)?
    };

    if combinations.is_empty() {
        return Err(BuildError::BuildProcessFailed {
            details: "No valid build combinations found".to_string(),
        }.into());
    }

    println!("Generated {} build combinations:", combinations.len());
    for combo in &combinations {
        println!("  → {}: env={:?}, extensions={:?}, combos={:?}",
                combo.output_dir, combo.environment, combo.extensions, combo.combo_names);
    }

    Ok(combinations)
}

/// Resolve build combinations from new targets configuration
fn resolve_target_combinations(config: &config::Config, targets: &config::BuildTargets) -> Result<Vec<BuildCombination>> {
    let mut combinations = Vec::new();
    
    // Get environments from targets or fallback to global config
    let environments = targets.environments.as_ref()
        .or(config.build.environments.as_ref())
        .map_or_else(Vec::new, |v| v.clone());

    if environments.is_empty() {
        // No environments, create extension-only combinations
        for env_target in targets.environment_configs.values() {
            let direct_extensions = env_target.extensions.as_ref().map_or_else(Vec::new, |ext| ext.clone());
            let combo_names = config::get_target_combo_names(env_target);
            
            // Create combinations for individual extensions
            for ext in &direct_extensions {
                combinations.push(BuildCombination {
                    environment: None,
                    extensions: vec![ext.clone()],
                    combo_names: vec![],
                    output_dir: ext.clone(),
                });
            }
            
            // Create combinations for each named combo
            for combo_name in &combo_names {
                let combo_extensions = config.build.combos.get(combo_name)
                    .ok_or_else(|| ValidationError::ComboNotFound {
                        combo_name: combo_name.clone(),
                        available_combos: config.build.combos.keys().cloned().collect(),
                    })?;
                
                combinations.push(BuildCombination {
                    environment: None,
                    extensions: combo_extensions.clone(),
                    combo_names: vec![combo_name.clone()],
                    output_dir: combo_name.clone(),
                });
            }
        }
        
        // Add base-only combination if no other combinations AND we have multiple environments
        if combinations.is_empty() && environments.len() >= 2 {
            combinations.push(BuildCombination {
                environment: None,
                extensions: vec![],
                combo_names: vec![],
                output_dir: "base".to_string(),
            });
        }
    } else {
        // Process each environment
        for env in &environments {
            // Environment base
            combinations.push(BuildCombination {
                environment: Some(env.clone()),
                extensions: vec![],
                combo_names: vec![],
                output_dir: if environments.len() == 1 { env.clone() } else { format!("{}/base", env) },
            });
            
            // Check if environment has specific configuration
            if let Some(env_target) = targets.environment_configs.get(env) {
                let direct_extensions = env_target.extensions.as_ref().map_or_else(Vec::new, |ext| ext.clone());
                let combo_names = config::get_target_combo_names(env_target);
                
                // Create combinations for individual extensions
                for ext in &direct_extensions {
                    let output_dir = format!("{}/{}", env, ext);
                    
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: vec![ext.clone()],
                        combo_names: vec![],
                        output_dir,
                    });
                }
                
                // Create combinations for each named combo
                for combo_name in &combo_names {
                    let combo_extensions = config.build.combos.get(combo_name)
                        .ok_or_else(|| ValidationError::ComboNotFound {
                            combo_name: combo_name.clone(),
                            available_combos: config.build.combos.keys().cloned().collect(),
                        })?;
                    
                    let output_dir = format!("{}/{}", env, combo_name);
                    
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: combo_extensions.clone(),
                        combo_names: vec![combo_name.clone()],
                        output_dir,
                    });
                }
            }
        }
    }
    
    Ok(combinations)
}

/// Resolve build combinations using legacy configuration
fn resolve_legacy_combinations(config: &config::Config) -> Result<Vec<BuildCombination>> {
    let mut combinations = Vec::new();

    let environments = config.build.environments.as_ref().map_or_else(Vec::new, |v| v.clone());
    let extensions = config.build.extensions.as_ref().map_or_else(Vec::new, |v| v.clone());

    // Use individual extensions, not combinations
    let mut all_extension_combos = vec![vec![]]; // Empty combination
    if !extensions.is_empty() {
        for ext in &extensions {
            all_extension_combos.push(vec![ext.clone()]);
        }
    }

    let num_envs = environments.len();

    match (num_envs, extensions.len()) {
        (0, 0) => {
            return Err(BuildError::InvalidBuildCombination {
                env: None,
                extensions: vec![],
            }.into());
        }
        (1, _) if !extensions.is_empty() => {
            // 1 environment with extensions
            let env = &environments[0];

            // Check if we should create subfolders
            let should_create_subfolders = extensions.len() > 1 || !config.build.skip_base_generation;

            if should_create_subfolders {
                // Create base and extension subfolders
                if !config.build.skip_base_generation {
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: vec![],
                        combo_names: vec![],
                        output_dir: format!("{}/base", env),
                    });
                }

                // Extensions for environment
                for ext_combo in &all_extension_combos {
                    if !ext_combo.is_empty() {
                        let combo_str = ext_combo.join("+");
                        let output_dir = format!("{}/{}", env, combo_str);
                        combinations.push(BuildCombination {
                            environment: Some(env.clone()),
                            extensions: ext_combo.clone(),
                            combo_names: vec![],
                            output_dir,
                        });
                    }
                }
            } else {
                // Single extension, no subfolders - put directly in env folder
                combinations.push(BuildCombination {
                    environment: Some(env.clone()),
                    extensions: extensions.clone(),
                    combo_names: vec![],
                    output_dir: env.clone(),
                });
            }
        }
        (_, 0) if num_envs >= 1 => {
            // Environments only: create folders for each environment
            for env in &environments {
                combinations.push(BuildCombination {
                    environment: Some(env.clone()),
                    extensions: vec![],
                    combo_names: vec![],
                    output_dir: env.clone(),
                });
            }
        }
        (_, _) if !extensions.is_empty() => {
            // Environments with extensions
            if num_envs == 1 {
                // Same logic as above single environment case
                let env = &environments[0];
                let should_create_subfolders = extensions.len() > 1 || !config.build.skip_base_generation;

                if should_create_subfolders {
                    if !config.build.skip_base_generation {
                        combinations.push(BuildCombination {
                            environment: Some(env.clone()),
                            extensions: vec![],
                            combo_names: vec![],
                            output_dir: format!("{}/base", env),
                        });
                    }
                    for ext_combo in &all_extension_combos {
                        if !ext_combo.is_empty() {
                            let combo_str = ext_combo.join("+");
                            let output_dir = format!("{}/{}", env, combo_str);
                            combinations.push(BuildCombination {
                                environment: Some(env.clone()),
                                extensions: ext_combo.clone(),
                                combo_names: vec![],
                                output_dir,
                            });
                        }
                    }
                } else {
                    // Single extension, no subfolders
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: extensions.clone(),
                        combo_names: vec![],
                        output_dir: env.clone(),
                    });
                }
            } else {
                // 2+ environments with extensions: always create env folders with subfolders
                for env in &environments {
                    // Environment base (only if not skipped)
                    if !config.build.skip_base_generation {
                        combinations.push(BuildCombination {
                            environment: Some(env.clone()),
                            extensions: vec![],
                            combo_names: vec![],
                            output_dir: format!("{}/base", env),
                        });
                    }

                    // Environment with extensions
                    for ext_combo in &all_extension_combos {
                        if !ext_combo.is_empty() {
                            let combo_str = ext_combo.join("+");
                            let output_dir = format!("{}/{}", env, combo_str);
                            combinations.push(BuildCombination {
                                environment: Some(env.clone()),
                                extensions: ext_combo.clone(),
                                combo_names: vec![],
                                output_dir,
                            });
                        }
                    }
                }
            }
        }
        _ => {}
    }

    // Add extension-only combinations only if environments = 0
    if num_envs == 0 {
        // For 0 environments case: no subfolders if single extension, subfolders if multiple extensions
        let should_create_subfolders = extensions.len() > 1;
        
        for ext_combo in &all_extension_combos {
            if !ext_combo.is_empty() {
                let combo_str = ext_combo.join("+");
                let output_dir = if should_create_subfolders {
                    combo_str
                } else {
                    // Single extension with 0 environments - put directly in build root
                    "".to_string()
                };
                
                combinations.push(BuildCombination {
                    environment: None,
                    extensions: ext_combo.clone(),
                    combo_names: vec![],
                    output_dir,
                });
            }
        }
    }

    Ok(combinations)
}


/// Resolve all extensions from direct extensions and combo names
fn resolve_all_extensions(config: &config::Config, direct_extensions: &[String], combo_names: &[String]) -> Result<Vec<String>> {
    let mut all_extensions = Vec::new();
    
    // Add direct extensions
    for ext in direct_extensions {
        if !all_extensions.contains(ext) {
            all_extensions.push(ext.clone());
        }
    }
    
    // Add extensions from combos
    if !combo_names.is_empty() {
        let combo_extensions = config::resolve_combo_extensions(config, combo_names)?;
        for ext in combo_extensions {
            if !all_extensions.contains(&ext) {
                all_extensions.push(ext);
            }
        }
    }
    
    Ok(all_extensions)
}

/// Create build directory structure and merge files
fn create_build_structure(executor: &BuildExecutor, combinations: &[BuildCombination]) -> Result<()> {
    let build_dir = Path::new(&executor.config.paths.build_dir);

    // Smart cleanup with .env preservation
    let cleaner = BuildCleaner::new(
        build_dir,
        executor.config.build.preserve_env_files,
        executor.config.build.env_file_patterns.clone(),
    );

    cleaner.clean_build_directory()
        .map_err(|e| BuildError::BuildProcessFailed {
            details: format!("Failed to clean build directory: {}", e),
        })?;

    // Collect new structure paths for .env restoration
    let new_structure: Vec<String> = combinations
        .iter()
        .map(|combo| combo.output_dir.clone())
        .collect();

    for combo in combinations {
        println!("Processing combination: {:?}", combo.output_dir);

        // Special cases for putting file directly in build directory without subfolders:
        // 1. 1 env + 0 ext
        // 2. 0 env + 1 ext (when output_dir is empty)
        let (output_path, file_name) = if (executor.num_envs == 1 && executor.num_extensions == 0) || combo.output_dir.is_empty() {
            (build_dir.to_path_buf(), "docker-compose.yml".to_string())
        } else {
            let path = build_dir.join(&combo.output_dir);
            fs::create_dir_all(&path)
                .map_err(|e| FileSystemError::DirectoryCreationFailed {
                    path: path.clone(),
                    source: e,
                })?;
            (path, "docker-compose.yml".to_string())
        };

        // Merge compose files
        let environment_opt = combo.environment.as_deref();
        
        // Resolve all extensions (direct + from combos)
        let all_extensions = resolve_all_extensions(&executor.config, &combo.extensions, &combo.combo_names)?;
        
        // Choose merger based on configuration
        let final_content = match executor.config.build.yaml_merger {
            YamlMergerType::Yq => {
                // Use yq merger
                let content = yq_merge_compose_files(&executor.yq_merger, environment_opt, &all_extensions)
                    .map_err(|e| BuildError::BuildProcessFailed {
                        details: format!("Failed to merge compose files with yq for combination {:?}: {}", combo.output_dir, e),
                    })?;
                println!("✓ Used yq merger for: {}", combo.output_dir);
                content
            }
            YamlMergerType::Rust => {
                // Use Rust merger directly
                let merged = merge_compose_files(&executor.rust_merger, environment_opt, &all_extensions)
                    .map_err(|e| BuildError::BuildProcessFailed {
                        details: format!("Failed to merge compose files with Rust for combination {:?}: {}", combo.output_dir, e),
                    })?;
                
                println!("✓ Used Rust merger for: {}", combo.output_dir);
                serialize_yaml_with_proper_indentation(&merged)?
            }
        };

        // Write merged file
        let compose_path = output_path.join(&file_name);

        fs::write(&compose_path, final_content)
            .map_err(|e| BuildError::OutputFileWriteError {
                path: compose_path.clone(),
                source: e,
            })?;
        println!("✓ Created {}", compose_path.display());

        // Process .env.example files if enabled
        if executor.config.build.copy_env_example {
            let env_file_path = output_path.join(".env.example");
            let environment_opt = combo.environment.as_deref();
            
            // Resolve all extensions for .env merging
            let all_extensions = resolve_all_extensions(&executor.config, &combo.extensions, &combo.combo_names)?;
            
            match merge_env_files(&executor.env_merger, environment_opt, &all_extensions) {
                Ok(merged_env) => {
                    if !merged_env.variables.is_empty() || !merged_env.header_comments.is_empty() {
                        if let Err(e) = write_merged_env(&merged_env, &env_file_path.to_string_lossy()) {
                            println!("Warning: Failed to write .env.example file for {}: {}", combo.output_dir, e);
                        }
                    } else {
                        println!("No .env.example variables found for combination: {}", combo.output_dir);
                    }
                }
                Err(e) => {
                    println!("Warning: Failed to merge .env.example files for {}: {}", combo.output_dir, e);
                }
            }
        }

        // Copy additional files if enabled
        let file_copier = FileCopier::new(executor.config.clone())
            .map_err(|e| BuildError::BuildProcessFailed {
                details: format!("Failed to initialize file copier: {}", e),
            })?;

        // Resolve all extensions for file copying
        let all_extensions = resolve_all_extensions(&executor.config, &combo.extensions, &combo.combo_names)?;
        
        if let Err(e) = file_copier.copy_additional_files(
            combo.environment.as_deref(),
            &all_extensions,
            &output_path,
        ) {
            println!("Warning: Failed to copy additional files for {}: {}", combo.output_dir, e);
        }
    }

    // Restore preserved .env files after creating new structure
    cleaner.restore_env_files(&new_structure)
        .map_err(|e| BuildError::BuildProcessFailed {
            details: format!("Failed to restore .env files: {}", e),
        })?;

    Ok(())
}

/// Serialize YAML with proper formatting and clean null values
fn serialize_yaml_with_proper_indentation(value: &serde_yaml_ng::Value) -> Result<String> {
    // Use yaml-rust2 for better formatting control
    let mut out_str = String::new();
    {
        let mut emitter = yaml_rust2::YamlEmitter::new(&mut out_str);
        
        // Convert serde_yaml::Value to yaml_rust2::Yaml
        let yaml_str = serde_yaml_ng::to_string(value)
            .map_err(|e| YamlError::SerializationError {
                details: e.to_string(),
            })?;
            
        let docs = yaml_rust2::YamlLoader::load_from_str(&yaml_str)
            .map_err(|e| YamlError::SerializationError {
                details: format!("Failed to parse YAML for formatting: {}", e),
            })?;
            
        if let Some(doc) = docs.first() {
            emitter.dump(doc)
                .map_err(|e| YamlError::SerializationError {
                    details: format!("Failed to emit YAML: {}", e),
                })?;
        }
    }
    
    // Clean up null values (~ symbols)
    let yaml_string = clean_yaml_null_values(out_str);
    
    Ok(yaml_string)
}

/// Clean YAML string from null values (~ symbols) in volumes sections
fn clean_yaml_null_values(yaml_content: String) -> String {
    use regex::Regex;
    
    // Replace patterns like "volume_name: ~" or "volume_name: null" with "volume_name:"
    let re = Regex::new(r"(\s+\w+):\s*(?:~|null)\s*$").unwrap();
    let cleaned = re.replace_all(&yaml_content, "$1:");
    
    // Also handle inline null values in volumes sections
    let re2 = Regex::new(r"(\s+\w+):\s*(?:~|null)\s*\n").unwrap();
    let cleaned2 = re2.replace_all(&cleaned, "$1:\n");
    
    cleaned2.to_string()
}

/// Structure representing a build combination
#[derive(Debug)]
struct BuildCombination {
    pub environment: Option<String>,
    pub extensions: Vec<String>,
    pub combo_names: Vec<String>,
    pub output_dir: String,
}