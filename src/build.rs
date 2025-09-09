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
    pub num_combos: usize,
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

        let num_envs = config::get_environments_list(&config).len();
        let num_extensions = config.build.extensions.as_ref().map_or(0, |e| e.len());
        let num_combos = config.build.combos.len();

        Ok(Self { config, rust_merger, yq_merger, env_merger, num_envs, num_extensions, num_combos })
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
    let combinations = if config::is_using_new_environments_api(config) {
        // New API mode: use new environments structure
        resolve_new_api_combinations(config)?
    } else if let Some(ref targets) = config.build.targets {
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

/// Resolve build combinations using new environments API
fn resolve_new_api_combinations(config: &config::Config) -> Result<Vec<BuildCombination>> {
    if let Some(ref env_config) = config.build.environments_config {
        // Convert new API to legacy format for compatibility
        let legacy_targets = config::BuildTargets {
            environment_configs: env_config.environment_configs.iter()
                .map(|(name, cfg)| (name.clone(), config::EnvironmentTarget {
                    extensions: cfg.extensions.clone(),
                    combos: cfg.combos.clone(),
                    skip_base_generation: cfg.skip_base_generation,
                }))
                .collect(),
        };
        
        resolve_legacy_combinations_with_targets(config, Some(&legacy_targets))
    } else {
        Ok(vec![])
    }
}

/// Resolve build combinations from targets configuration (delegates to legacy with filtering)
fn resolve_target_combinations(config: &config::Config, targets: &config::BuildTargets) -> Result<Vec<BuildCombination>> {
    // Use legacy logic with targets filtering
    resolve_legacy_combinations_with_targets(config, Some(targets))
}

/// Resolve build combinations using legacy configuration with optional targets filtering
fn resolve_legacy_combinations(config: &config::Config) -> Result<Vec<BuildCombination>> {
    resolve_legacy_combinations_with_targets(config, None)
}

/// Internal function to resolve combinations with optional targets filtering
fn resolve_legacy_combinations_with_targets(config: &config::Config, targets: Option<&config::BuildTargets>) -> Result<Vec<BuildCombination>> {
    let mut combinations = Vec::new();

    let environments = config::get_environments_list(config);
    
    // Get global extensions (applies to all environments if no per-env config)
    let global_extensions = config.build.extensions.as_ref().map_or_else(Vec::new, |v| v.clone());

    // Build per-environment extension/combo/skip_base map from targets
    let mut env_specific_configs = std::collections::HashMap::new();
    if let Some(targets) = targets {
        for (env_name, env_target) in &targets.environment_configs {
            let env_extensions = env_target.extensions.as_ref().map_or_else(Vec::new, |e| e.clone());
            let env_combo_names = env_target.combos.as_ref().map_or_else(Vec::new, |c| c.clone());
            let env_skip_base = env_target.skip_base_generation.unwrap_or(config.build.skip_base_generation);
            env_specific_configs.insert(env_name.clone(), (env_extensions, env_combo_names, env_skip_base));
        }
    } else {
        // Try new API if no legacy targets
        for env in &environments {
            if let Some(env_cfg) = config::get_environment_config(config, env) {
                let env_extensions = env_cfg.extensions.as_ref().map_or_else(Vec::new, |e| e.clone());
                let env_combo_names = env_cfg.combos.as_ref().map_or_else(Vec::new, |c| c.clone());
                let env_skip_base = env_cfg.skip_base_generation.unwrap_or(config.build.skip_base_generation);
                env_specific_configs.insert(env.clone(), (env_extensions, env_combo_names, env_skip_base));
            }
        }
    }

    // Calculate total variants for each environment
    let mut total_variants = 0;
    for env in &environments {
        if let Some((env_extensions, env_combo_names, _env_skip_base)) = env_specific_configs.get(env) {
            // Per-environment specific extensions/combos
            total_variants += env_extensions.len() + env_combo_names.len();
        } else {
            // Use global extensions/combos
            total_variants += global_extensions.len() + config.build.combos.len();
        }
    }
    
    // If no environments, count global variants
    if environments.is_empty() {
        total_variants = global_extensions.len() + config.build.combos.len();
    }

    let num_envs = environments.len();

    match (num_envs, total_variants) {
        (0, 0) => {
            // No environments and no extensions - create base-only combination
            combinations.push(BuildCombination {
                environment: None,
                extensions: vec![],
                combo_names: vec![],
                output_dir: "".to_string(), // Put directly in build root
            });
        }
        (1, _) if total_variants > 0 => {
            // 1 environment with extensions/combos
            let env = &environments[0];

            // Get extensions, combos, and skip_base for this environment (either per-env or global)
            let (env_extensions, env_combo_names, env_skip_base) = if let Some((ext, combos, skip_base)) = env_specific_configs.get(env) {
                (ext.clone(), combos.clone(), *skip_base)
            } else {
                (global_extensions.clone(), config.build.combos.keys().cloned().collect(), config.build.skip_base_generation)
            };

            let env_total_variants = env_extensions.len() + env_combo_names.len();
            let should_create_subfolders = env_total_variants > 1 || !env_skip_base;

            if should_create_subfolders {
                // Create base subfolder if not skipped (NO env prefix for single environment)
                if !env_skip_base {
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: vec![],
                        combo_names: vec![],
                        output_dir: "base".to_string(),
                    });
                }

                // Extensions for environment (NO env prefix for single environment)
                for ext in &env_extensions {
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: vec![ext.clone()],
                        combo_names: vec![],
                        output_dir: ext.clone(),
                    });
                }
                
                // Add combo combinations for environment (NO env prefix for single environment)
                for combo_name in &env_combo_names {
                    let combo_extensions = config.build.combos.get(combo_name)
                        .ok_or_else(|| ValidationError::ComboNotFound {
                            combo_name: combo_name.clone(),
                            available_combos: config.build.combos.keys().cloned().collect(),
                        })?;
                    
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: combo_extensions.clone(),
                        combo_names: vec![combo_name.clone()],
                        output_dir: combo_name.clone(),
                    });
                }
            } else {
                // Single variant, no subfolders - put directly in build root
                if env_extensions.len() == 1 {
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: env_extensions.clone(),
                        combo_names: vec![],
                        output_dir: "".to_string(), // Empty = build root
                    });
                } else if env_combo_names.len() == 1 {
                    // Single combo case
                    let combo_name = &env_combo_names[0];
                    let combo_extensions = config.build.combos.get(combo_name)
                        .ok_or_else(|| ValidationError::ComboNotFound {
                            combo_name: combo_name.clone(),
                            available_combos: config.build.combos.keys().cloned().collect(),
                        })?;
                    
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: combo_extensions.clone(),
                        combo_names: vec![combo_name.clone()],
                        output_dir: "".to_string(), // Empty = build root
                    });
                }
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
        _ => {
            // Multiple environments OR single env with variants - handle each environment individually
            for env in &environments {
                // Get extensions, combos, and skip_base for this environment
                let (env_extensions, env_combo_names, env_skip_base) = if let Some((ext, combos, skip_base)) = env_specific_configs.get(env) {
                    (ext.clone(), combos.clone(), *skip_base)
                } else {
                    (global_extensions.clone(), config.build.combos.keys().cloned().collect(), config.build.skip_base_generation)
                };

                let env_total_variants = env_extensions.len() + env_combo_names.len();
                
                if num_envs > 1 {
                    // Multiple environments - always use env/subfolder structure
                    // Note: Base creation is handled inside should_create_env_subfolders logic below

                    // Check if this specific environment should create subfolders
                    let should_create_env_subfolders = env_total_variants > 1 || !env_skip_base;
                    
                    if should_create_env_subfolders {
                        // Environment base (only if not skipped)
                        if !env_skip_base {
                            combinations.push(BuildCombination {
                                environment: Some(env.clone()),
                                extensions: vec![],
                                combo_names: vec![],
                                output_dir: format!("{}/base", env),
                            });
                        }

                        // Environment with extensions - with subfolders
                        for ext in &env_extensions {
                            combinations.push(BuildCombination {
                                environment: Some(env.clone()),
                                extensions: vec![ext.clone()],
                                combo_names: vec![],
                                output_dir: format!("{}/{}", env, ext),
                            });
                        }
                        
                        // Environment with combos - with subfolders
                        for combo_name in &env_combo_names {
                            let combo_extensions = config.build.combos.get(combo_name)
                                .ok_or_else(|| ValidationError::ComboNotFound {
                                    combo_name: combo_name.clone(),
                                    available_combos: config.build.combos.keys().cloned().collect(),
                                })?;
                            
                            combinations.push(BuildCombination {
                                environment: Some(env.clone()),
                                extensions: combo_extensions.clone(),
                                combo_names: vec![combo_name.clone()],
                                output_dir: format!("{}/{}", env, combo_name),
                            });
                        }
                    } else {
                        // Single variant for this environment - put directly in env folder
                        if env_extensions.len() == 1 {
                            combinations.push(BuildCombination {
                                environment: Some(env.clone()),
                                extensions: env_extensions.clone(),
                                combo_names: vec![],
                                output_dir: env.clone(),
                            });
                        } else if env_combo_names.len() == 1 {
                            let combo_name = &env_combo_names[0];
                            let combo_extensions = config.build.combos.get(combo_name)
                                .ok_or_else(|| ValidationError::ComboNotFound {
                                    combo_name: combo_name.clone(),
                                    available_combos: config.build.combos.keys().cloned().collect(),
                                })?;
                            
                            combinations.push(BuildCombination {
                                environment: Some(env.clone()),
                                extensions: combo_extensions.clone(),
                                combo_names: vec![combo_name.clone()],
                                output_dir: env.clone(),
                            });
                        }
                    }
                } else if env_total_variants == 0 {
                    // Single environment, no variants - environment only
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: vec![],
                        combo_names: vec![],
                        output_dir: env.clone(),
                    });
                }
            }
        }
    }

    // Add extension-only combinations only if environments = 0
    if num_envs == 0 {
        // For 0 environments case: no subfolders if single variant, subfolders if multiple variants
        let should_create_subfolders = total_variants > 1;
        
        // Use global extensions if no environments
        for ext in &global_extensions {
            let output_dir = if should_create_subfolders {
                ext.clone()
            } else {
                "".to_string() // Single variant - put directly in build root
            };
            
            combinations.push(BuildCombination {
                environment: None,
                extensions: vec![ext.clone()],
                combo_names: vec![],
                output_dir,
            });
        }
        
        // Add combo combinations for 0 environments case
        for (combo_name, combo_extensions) in &config.build.combos {
            let output_dir = if should_create_subfolders {
                combo_name.clone()
            } else {
                "".to_string() // Single combo - put directly in build root
            };
            
            combinations.push(BuildCombination {
                environment: None,
                extensions: combo_extensions.clone(),
                combo_names: vec![combo_name.clone()],
                output_dir,
            });
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
        executor.config.build.backup_dir.clone(),
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
        // 1. 1 env + 0 ext + 0 combos
        // 2. 0 env + 1 total variant (when output_dir is empty)
        let total_variants = executor.num_extensions + executor.num_combos;
        let (output_path, file_name) = if (executor.num_envs == 1 && total_variants == 0) || combo.output_dir.is_empty() {
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