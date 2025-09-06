use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use anyhow::{Context, Result};
use glob::Pattern;

use crate::config::Config;

/// File copy priority - higher number = higher priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FilePriority {
    Base = 1,
    Environment = 2,
    Extension = 3,
}

/// Information about a file to be copied
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub source_path: PathBuf,
    pub priority: FilePriority,
    pub source_component: String,
}

/// Manages file copying operations with priority-based overriding
pub struct FileCopier {
    config: Config,
    exclude_patterns: Vec<Pattern>,
}

impl FileCopier {
    /// Create a new FileCopier instance
    pub fn new(config: Config) -> Result<Self> {
        let exclude_patterns = config.build.exclude_patterns
            .iter()
            .map(|pattern| Pattern::new(pattern))
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to compile exclude patterns")?;

        Ok(FileCopier {
            config,
            exclude_patterns,
        })
    }

    /// Copy all additional files for the specified environment and extensions
    pub fn copy_additional_files(
        &self,
        environment: Option<&str>,
        extensions: &[String],
        output_dir: &Path,
    ) -> Result<()> {
        if !self.config.build.copy_additional_files {
            println!("Skipping additional file copying (disabled in config)");
            return Ok(());
        }

        println!("Copying additional files...");

        // Discover all files from components
        let mut file_map = HashMap::new();
        
        // 1. Discover base files (lowest priority)
        self.discover_files(
            Path::new(&self.config.paths.base_dir),
            FilePriority::Base,
            "base",
            &mut file_map,
        )?;

        // 2. Discover environment files (medium priority)
        if let Some(env) = environment {
            let env_path = Path::new(&self.config.paths.environments_dir).join(env);
            if env_path.exists() {
                self.discover_files(
                    &env_path,
                    FilePriority::Environment,
                    &format!("environment:{}", env),
                    &mut file_map,
                )?;
            }
        }

        // 3. Discover extension files (highest priority)
        for extension in extensions {
            for ext_dir in &self.config.paths.extensions_dirs {
                let ext_path = Path::new(ext_dir).join(extension);
                if ext_path.exists() {
                    self.discover_files(
                        &ext_path,
                        FilePriority::Extension,
                        &format!("extension:{}", extension),
                        &mut file_map,
                    )?;
                    break; // Use first found extension directory
                }
            }
        }

        // Copy files with priority resolution
        for (relative_path, file_info) in file_map {
            self.copy_file_with_priority(&file_info, &relative_path, output_dir)?;
        }

        println!("Additional file copying completed");
        Ok(())
    }

    /// Discover all files in a component directory
    fn discover_files(
        &self,
        component_dir: &Path,
        priority: FilePriority,
        component_name: &str,
        file_map: &mut HashMap<PathBuf, FileInfo>,
    ) -> Result<()> {
        if !component_dir.exists() {
            return Ok(());
        }

        self.discover_files_recursive(
            component_dir,
            component_dir,
            priority,
            component_name,
            file_map,
        )
    }

    /// Recursively discover files in a directory
    fn discover_files_recursive(
        &self,
        root_dir: &Path,
        current_dir: &Path,
        priority: FilePriority,
        component_name: &str,
        file_map: &mut HashMap<PathBuf, FileInfo>,
    ) -> Result<()> {
        for entry in fs::read_dir(current_dir)
            .with_context(|| format!("Failed to read directory: {}", current_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively process subdirectories
                self.discover_files_recursive(
                    root_dir,
                    &path,
                    priority,
                    component_name,
                    file_map,
                )?;
            } else if path.is_file() {
                // Process file
                let relative_path = path.strip_prefix(root_dir)
                    .with_context(|| format!("Failed to get relative path for: {}", path.display()))?
                    .to_path_buf();

                // Check if file should be excluded
                if self.should_exclude_file(&relative_path) {
                    println!("  Excluding file: {} (matches exclude pattern)", relative_path.display());
                    continue;
                }

                let file_info = FileInfo {
                    source_path: path.clone(),
                    priority,
                    source_component: component_name.to_string(),
                };

                // Apply priority-based resolution
                self.resolve_file_priority(&relative_path, file_info, file_map);
            }
        }

        Ok(())
    }

    /// Determine if a file should be excluded based on patterns
    fn should_exclude_file(&self, relative_path: &Path) -> bool {
        let path_str = relative_path.to_string_lossy();
        
        for pattern in &self.exclude_patterns {
            if pattern.matches(&path_str) {
                return true;
            }
            
            // Also check just the filename
            if let Some(filename) = relative_path.file_name() {
                let filename_str = filename.to_string_lossy();
                if pattern.matches(&filename_str) {
                    return true;
                }
            }
        }

        false
    }

    /// Resolve file priority conflicts
    fn resolve_file_priority(
        &self,
        relative_path: &PathBuf,
        new_file: FileInfo,
        file_map: &mut HashMap<PathBuf, FileInfo>,
    ) {
        match file_map.get(relative_path) {
            Some(existing_file) => {
                if new_file.priority > existing_file.priority {
                    println!(
                        "  File {}: {} overrides {} (priority: {:?} > {:?})",
                        relative_path.display(),
                        new_file.source_component,
                        existing_file.source_component,
                        new_file.priority,
                        existing_file.priority
                    );
                    file_map.insert(relative_path.clone(), new_file);
                } else if new_file.priority == existing_file.priority {
                    // Same priority - last one wins (order matters)
                    println!(
                        "  File {}: {} replaces {} (same priority: {:?})",
                        relative_path.display(),
                        new_file.source_component,
                        existing_file.source_component,
                        new_file.priority
                    );
                    file_map.insert(relative_path.clone(), new_file);
                } else {
                    println!(
                        "  File {}: keeping {} (priority: {:?} > {:?})",
                        relative_path.display(),
                        existing_file.source_component,
                        existing_file.priority,
                        new_file.priority
                    );
                }
            }
            None => {
                println!(
                    "  File {} found from {} (priority: {:?})",
                    relative_path.display(),
                    new_file.source_component,
                    new_file.priority
                );
                file_map.insert(relative_path.clone(), new_file);
            }
        }
    }

    /// Copy a file with priority information
    fn copy_file_with_priority(
        &self,
        file_info: &FileInfo,
        relative_path: &PathBuf,
        output_dir: &Path,
    ) -> Result<()> {
        let dest_path = output_dir.join(relative_path);
        
        // Create parent directories if they don't exist
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Copy the file
        fs::copy(&file_info.source_path, &dest_path)
            .with_context(|| format!(
                "Failed to copy file from {} to {}",
                file_info.source_path.display(),
                dest_path.display()
            ))?;

        // Preserve permissions on Unix systems
        #[cfg(unix)]
        {
            if let Ok(metadata) = fs::metadata(&file_info.source_path) {
                let permissions = metadata.permissions();
                let _ = fs::set_permissions(&dest_path, permissions);
            }
        }

        println!(
            "  Copied: {} -> {} (from {})",
            relative_path.display(),
            dest_path.display(),
            file_info.source_component
        );

        Ok(())
    }
}