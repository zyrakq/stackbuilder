use std::fs;
use std::path::Path;
use yaml_rust::{YamlEmitter, YamlLoader};

use crate::config;
use crate::merger::{ComposeMerger, merge_compose_files};
use crate::env_merger::{EnvMerger, merge_env_files, write_merged_env};
use crate::error::{Result, BuildError, FileSystemError, YamlError};

/// Structure for managing build process execution
pub struct BuildExecutor {
    pub config: config::Config,
    pub merger: ComposeMerger,
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

        let merger = ComposeMerger::new(
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

        Ok(Self { config, merger, env_merger, num_envs, num_extensions })
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
            // 1 environment with extensions: create base, and extension folders
            let env = &environments[0];

            // Base for environment
            combinations.push(BuildCombination {
                environment: Some(env.clone()),
                extensions: vec![],
                output_dir: env.clone(),
            });

            // Extensions for environment
            for ext_combo in &all_extension_combos {
                if !ext_combo.is_empty() {
                    let combo_str = ext_combo.join("+");
                    let output_dir = format!("{}/{}", env, combo_str);
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: ext_combo.clone(),
                        output_dir,
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
                    output_dir: env.clone(),
                });
            }
        }
        (_, _) if !extensions.is_empty() => {
            // Environments with extensions
            if num_envs == 1 {
                // Same as above case
                let env = &environments[0];
                combinations.push(BuildCombination {
                    environment: Some(env.clone()),
                    extensions: vec![],
                    output_dir: env.clone(),
                });
                for ext_combo in &all_extension_combos {
                    if !ext_combo.is_empty() {
                        let combo_str = ext_combo.join("+");
                        let output_dir = format!("{}/{}", env, combo_str);
                        combinations.push(BuildCombination {
                            environment: Some(env.clone()),
                            extensions: ext_combo.clone(),
                            output_dir,
                        });
                    }
                }
            } else {
                // 2+ environments with extensions: create env folders, each with base and extensions
                for env in &environments {
                    // Environment base
                    combinations.push(BuildCombination {
                        environment: Some(env.clone()),
                        extensions: vec![],
                        output_dir: format!("{}/base", env),
                    });

                    // Environment with extensions
                    for ext_combo in &all_extension_combos {
                        if !ext_combo.is_empty() {
                            let combo_str = ext_combo.join("+");
                            let output_dir = format!("{}/{}", env, combo_str);
                            combinations.push(BuildCombination {
                                environment: Some(env.clone()),
                                extensions: ext_combo.clone(),
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
    // OR if environments = 1 AND there are extensions > 0
    if num_envs == 0 || (num_envs == 1 && !extensions.is_empty()) {
        for ext_combo in &all_extension_combos {
            if !ext_combo.is_empty() {
                let combo_str = ext_combo.join("+");
                combinations.push(BuildCombination {
                    environment: None,
                    extensions: ext_combo.clone(),
                    output_dir: combo_str,
                });
            } else if num_envs == 0 {
                // Base only - but only add this if no environments at all
                combinations.push(BuildCombination {
                    environment: None,
                    extensions: vec![],
                    output_dir: "base".to_string(),
                });
            }
        }
    }

    Ok(combinations)
}

/// Create build directory structure and merge files
fn create_build_structure(executor: &BuildExecutor, combinations: &[BuildCombination]) -> Result<()> {
    let build_dir = Path::new(&executor.config.paths.build_dir);

    // Clean build directory
    if build_dir.exists() {
        fs::remove_dir_all(build_dir)
            .map_err(|e| BuildError::BuildDirectoryCleanupFailed {
                path: build_dir.to_path_buf(),
                source: e,
            })?;
    }
    fs::create_dir_all(build_dir)
        .map_err(|e| BuildError::BuildDirectoryCreationFailed {
            path: build_dir.to_path_buf(),
            source: e,
        })?;

    for combo in combinations {
        println!("Processing combination: {:?}", combo.output_dir);

        // Special case for 1 env 0 ext: put file directly in build directory without subfolders
        let (output_path, file_name) = if executor.num_envs == 1 && executor.num_extensions == 0 {
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
        let merged = merge_compose_files(&executor.merger, environment_opt, &combo.extensions)
            .map_err(|e| BuildError::BuildProcessFailed {
                details: format!("Failed to merge compose files for combination {:?}: {}", combo.output_dir, e),
            })?;

        // Write merged file
        let compose_path = output_path.join(&file_name);
        let yaml_string = serde_yaml::to_string(&merged)
            .map_err(|e| YamlError::SerializationError {
                details: e.to_string(),
            })?;

        let yaml_docs = YamlLoader::load_from_str(&yaml_string)
            .map_err(|e| YamlError::ParseError {
                file: "serialized output".to_string(),
                details: format!("Failed to parse serialized YAML: {}", e),
            })?;
        
        let mut pretty_yaml_string = String::new();
        let mut emitter = YamlEmitter::new(&mut pretty_yaml_string);
        if let Some(yaml_doc) = yaml_docs.first() {
            emitter.dump(yaml_doc)
                .map_err(|e| YamlError::SerializationError {
                    details: format!("Failed to emit YAML: {}", e),
                })?;
        }

        fs::write(&compose_path, pretty_yaml_string)
            .map_err(|e| BuildError::OutputFileWriteError {
                path: compose_path.clone(),
                source: e,
            })?;
        println!("✓ Created {}", compose_path.display());

        // Process .env.example files if enabled
        if executor.config.build.copy_env_example {
            let env_file_path = output_path.join(".env.example");
            let environment_opt = combo.environment.as_deref();
            
            match merge_env_files(&executor.env_merger, environment_opt, &combo.extensions) {
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
    }

    Ok(())
}

/// Structure representing a build combination
#[derive(Debug)]
struct BuildCombination {
    pub environment: Option<String>,
    pub extensions: Vec<String>,
    pub output_dir: String,
}