use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Structure for managing build directory cleaning with .env file preservation
pub struct BuildCleaner {
    /// Path to the build directory
    build_path: PathBuf,
    /// Configuration for env file preservation
    preserve_env_files: bool,
    /// Patterns for env files to preserve
    env_file_patterns: Vec<String>,
    /// Temporary backup directory name
    backup_dir_name: String,
}

/// Represents a preserved .env file with its original location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreservedEnvFile {
    /// Relative path from build directory where file was found
    pub original_path: PathBuf,
    /// Content of the .env file
    pub content: String,
    /// Environment name if detected from path
    pub environment: Option<String>,
    /// Extension names if detected from path  
    pub extensions: Vec<String>,
}

/// Result of scanning for .env files
#[derive(Debug)]
pub struct EnvFileScanResult {
    /// All found .env files with their metadata
    pub files: Vec<PreservedEnvFile>,
    /// Total number of files found
    pub count: usize,
}

/// Mapping between old and new paths for .env file restoration
#[derive(Debug)]
pub struct PathMapping {
    /// Original relative path in build directory
    pub old_path: PathBuf,
    /// New relative path in build directory
    pub new_path: PathBuf,
    /// Confidence score for this mapping (0.0 - 1.0)
    pub confidence: f32,
}

impl BuildCleaner {
    /// Create a new BuildCleaner instance
    pub fn new<P: AsRef<Path>>(
        build_path: P,
        preserve_env_files: bool,
        env_file_patterns: Vec<String>,
    ) -> Self {
        Self {
            build_path: build_path.as_ref().to_path_buf(),
            preserve_env_files,
            env_file_patterns,
            backup_dir_name: ".stackbuilder_env_backup".to_string(),
        }
    }

    /// Main function to clean build directory with .env preservation
    pub fn clean_build_directory(&self) -> Result<()> {
        if !self.preserve_env_files {
            println!("Env file preservation disabled, performing standard cleanup");
            return self.standard_cleanup();
        }

        println!("Starting intelligent build directory cleanup with .env preservation");

        // Step 1: Scan for .env files before cleanup
        let scan_result = self.scan_env_files()
            .context("Failed to scan for .env files")?;

        if scan_result.count == 0 {
            println!("No .env files found, performing standard cleanup");
            return self.standard_cleanup();
        }

        println!("Found {} .env files to preserve", scan_result.count);

        // Step 2: Preserve .env files to temporary location
        self.preserve_env_files(&scan_result.files)
            .context("Failed to preserve .env files")?;

        // Step 3: Clean build directory
        self.standard_cleanup()
            .context("Failed to clean build directory")?;

        // Step 4: Restore .env files (will be called after new structure is created)
        println!("✓ Build directory cleaned, .env files preserved for restoration");
        
        Ok(())
    }

    /// Restore preserved .env files to new build structure
    pub fn restore_env_files(&self, new_structure: &[String]) -> Result<()> {
        if !self.preserve_env_files {
            return Ok(());
        }

        let backup_path = self.get_backup_path();
        if !backup_path.exists() {
            println!("No .env backup found, skipping restoration");
            return Ok(());
        }

        println!("Restoring preserved .env files to new build structure");

        // Load preserved files
        let preserved_files = self.load_preserved_files(&backup_path)
            .context("Failed to load preserved .env files")?;

        if preserved_files.is_empty() {
            println!("No preserved .env files to restore");
            return self.cleanup_backup();
        }

        // Generate path mappings
        let mappings = self.generate_path_mappings(&preserved_files, new_structure)
            .context("Failed to generate path mappings")?;

        // Restore files according to mappings
        let mut restored_count = 0;
        let mut fallback_count = 0;

        for preserved_file in &preserved_files {
            let restore_result = self.restore_single_file(preserved_file, &mappings);
            
            match restore_result {
                Ok(RestoreResult::Restored(path)) => {
                    println!("✓ Restored .env file to: {}", path.display());
                    restored_count += 1;
                }
                Ok(RestoreResult::Fallback(path)) => {
                    println!("⚠ Restored .env file to fallback location: {}", path.display());
                    fallback_count += 1;
                }
                Err(e) => {
                    println!("✗ Failed to restore .env file from {}: {}", 
                            preserved_file.original_path.display(), e);
                }
            }
        }

        println!("Restoration completed: {} restored, {} fallback placements", 
                restored_count, fallback_count);

        // Cleanup backup directory
        self.cleanup_backup()
            .context("Failed to cleanup backup directory")?;

        Ok(())
    }

    /// Scan build directory for .env files
    pub fn scan_env_files(&self) -> Result<EnvFileScanResult> {
        let mut files = Vec::new();

        if !self.build_path.exists() {
            return Ok(EnvFileScanResult { files, count: 0 });
        }

        self.scan_directory_recursive(&self.build_path, &self.build_path, &mut files)
            .context("Failed to scan build directory recursively")?;

        let count = files.len();
        println!("Scanned build directory, found {} .env files", count);

        Ok(EnvFileScanResult { files, count })
    }

    /// Recursively scan directory for .env files
    fn scan_directory_recursive(
        &self,
        current_dir: &Path,
        base_dir: &Path,
        files: &mut Vec<PreservedEnvFile>,
    ) -> Result<()> {
        for entry in fs::read_dir(current_dir)
            .with_context(|| format!("Failed to read directory: {}", current_dir.display()))? {
            
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.is_dir() {
                // Skip our own backup directory
                if path.file_name().unwrap_or_default().to_string_lossy() == self.backup_dir_name {
                    continue;
                }
                self.scan_directory_recursive(&path, base_dir, files)?;
            } else if self.is_env_file(&path) {
                let relative_path = path.strip_prefix(base_dir)
                    .context("Failed to calculate relative path")?;
                
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read .env file: {}", path.display()))?;

                let (environment, extensions) = self.analyze_env_file_path(relative_path);

                files.push(PreservedEnvFile {
                    original_path: relative_path.to_path_buf(),
                    content,
                    environment: environment.clone(),
                    extensions: extensions.clone(),
                });

                println!("Found .env file: {} (env: {:?}, ext: {:?})",
                        relative_path.display(), environment, extensions);
            }
        }

        Ok(())
    }

    /// Check if file matches .env patterns
    fn is_env_file(&self, path: &Path) -> bool {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            self.env_file_patterns.iter().any(|pattern| {
                // Exact match only - no partial matching or extensions
                filename == pattern
            })
        } else {
            false
        }
    }

    /// Analyze .env file path to extract environment and extension info
    fn analyze_env_file_path(&self, path: &Path) -> (Option<String>, Vec<String>) {
        let path_components: Vec<&str> = path.components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        let mut environment = None;
        let mut extensions = Vec::new();

        // Analyze path structure to detect environment and extensions
        // Examples:
        // - dev/auth/.env -> env: "dev", ext: ["auth"]
        // - production/base/.env -> env: "production", ext: []
        // - staging/monitoring/.env -> env: "staging", ext: ["monitoring"]

        if path_components.len() >= 2 {
            // First component is likely environment
            environment = Some(path_components[0].to_string());
            
            // Subsequent components before filename are likely extensions
            for &component in &path_components[1..path_components.len()-1] {
                if component != "base" {
                    extensions.push(component.to_string());
                }
            }
        } else if path_components.len() == 1 {
            // File in root - might be single environment or extension-only
            // Additional heuristics could be applied here
        }

        (environment, extensions)
    }

    /// Preserve .env files to temporary backup location
    fn preserve_env_files(&self, files: &[PreservedEnvFile]) -> Result<()> {
        let backup_path = self.get_backup_path();
        
        // Create backup directory
        fs::create_dir_all(&backup_path)
            .with_context(|| format!("Failed to create backup directory: {}", backup_path.display()))?;

        // Save metadata and files
        let metadata_path = backup_path.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(files)
            .context("Failed to serialize .env files metadata")?;
        
        fs::write(&metadata_path, metadata_json)
            .with_context(|| format!("Failed to write metadata file: {}", metadata_path.display()))?;

        // Copy actual files with preserved directory structure
        for (index, file) in files.iter().enumerate() {
            let backup_file_path = backup_path.join(format!("file_{}.env", index));
            fs::write(&backup_file_path, &file.content)
                .with_context(|| format!("Failed to backup .env file: {}", backup_file_path.display()))?;
        }

        println!("✓ Preserved {} .env files to backup location", files.len());
        Ok(())
    }

    /// Load preserved files from backup
    fn load_preserved_files(&self, backup_path: &Path) -> Result<Vec<PreservedEnvFile>> {
        let metadata_path = backup_path.join("metadata.json");
        
        if !metadata_path.exists() {
            return Ok(Vec::new());
        }

        let metadata_content = fs::read_to_string(&metadata_path)
            .with_context(|| format!("Failed to read metadata file: {}", metadata_path.display()))?;

        let mut files: Vec<PreservedEnvFile> = serde_json::from_str(&metadata_content)
            .context("Failed to deserialize .env files metadata")?;

        // Load actual file contents
        for (index, file) in files.iter_mut().enumerate() {
            let backup_file_path = backup_path.join(format!("file_{}.env", index));
            if backup_file_path.exists() {
                file.content = fs::read_to_string(&backup_file_path)
                    .with_context(|| format!("Failed to read backup file: {}", backup_file_path.display()))?;
            }
        }

        Ok(files)
    }

    /// Generate path mappings from old to new structure
    fn generate_path_mappings(
        &self, 
        preserved_files: &[PreservedEnvFile], 
        new_structure: &[String]
    ) -> Result<Vec<PathMapping>> {
        let mut mappings = Vec::new();

        for file in preserved_files {
            let best_mapping = self.find_best_path_mapping(file, new_structure);
            mappings.push(best_mapping);
        }

        println!("Generated {} path mappings for .env restoration", mappings.len());
        Ok(mappings)
    }

    /// Find best path mapping for a preserved .env file
    fn find_best_path_mapping(&self, file: &PreservedEnvFile, new_structure: &[String]) -> PathMapping {
        let mut best_mapping = PathMapping {
            old_path: file.original_path.clone(),
            new_path: PathBuf::from(format!(".env.backup.{}", 
                file.original_path.to_string_lossy().replace('/', "_"))),
            confidence: 0.0,
        };

        // Try to match environment and extensions
        if let Some(ref _env) = file.environment {
            for new_path_str in new_structure {
                let confidence = self.calculate_path_confidence(file, new_path_str);
                
                if confidence > best_mapping.confidence {
                    let mut new_path = PathBuf::from(new_path_str);
                    
                    // Determine appropriate filename
                    let filename = file.original_path.file_name()
                        .unwrap_or_default().to_string_lossy();
                    new_path.push(filename.as_ref());
                    
                    best_mapping = PathMapping {
                        old_path: file.original_path.clone(),
                        new_path,
                        confidence,
                    };
                }
            }
        }

        best_mapping
    }

    /// Calculate confidence score for path mapping
    fn calculate_path_confidence(&self, file: &PreservedEnvFile, new_path: &str) -> f32 {
        let mut score: f32 = 0.0;
        let new_components: Vec<&str> = new_path.split('/').collect();

        // Environment matching
        if let Some(ref env) = file.environment {
            if new_components.contains(&env.as_str()) {
                score += 0.5;
            }
        }

        // Extension matching
        for ext in &file.extensions {
            if new_components.contains(&ext.as_str()) {
                score += 0.3;
            }
        }

        // Bonus for similar structure
        if new_components.len() == file.original_path.components().count() - 1 {
            score += 0.2;
        }

        score.min(1.0)
    }

    /// Restore a single .env file
    fn restore_single_file(
        &self, 
        file: &PreservedEnvFile, 
        mappings: &[PathMapping]
    ) -> Result<RestoreResult> {
        // Find mapping for this file
        let mapping = mappings.iter()
            .find(|m| m.old_path == file.original_path)
            .ok_or_else(|| anyhow::anyhow!("No mapping found for file: {}", file.original_path.display()))?;

        let target_path = self.build_path.join(&mapping.new_path);

        // Create parent directories
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Check for conflicts
        if target_path.exists() {
            let existing_content = fs::read_to_string(&target_path)
                .context("Failed to read existing .env file")?;
            
            if existing_content != file.content {
                // Create backup of existing file and use fallback name
                let fallback_path = self.generate_fallback_path(&target_path);
                fs::write(&fallback_path, &file.content)
                    .with_context(|| format!("Failed to write to fallback path: {}", fallback_path.display()))?;
                return Ok(RestoreResult::Fallback(fallback_path));
            }
        }

        // Write file to target location
        fs::write(&target_path, &file.content)
            .with_context(|| format!("Failed to write .env file: {}", target_path.display()))?;

        if mapping.confidence > 0.7 {
            Ok(RestoreResult::Restored(target_path))
        } else {
            Ok(RestoreResult::Fallback(target_path))
        }
    }

    /// Generate fallback path for conflicting files
    fn generate_fallback_path(&self, original_path: &Path) -> PathBuf {
        let parent = original_path.parent().unwrap_or_else(|| Path::new("."));
        let stem = original_path.file_stem().unwrap_or_default().to_string_lossy();
        let extension = original_path.extension().unwrap_or_default().to_string_lossy();
        
        let mut counter = 1;
        loop {
            let fallback_name = if extension.is_empty() {
                format!("{}.backup.{}", stem, counter)
            } else {
                format!("{}.backup.{}.{}", stem, counter, extension)
            };
            
            let fallback_path = parent.join(fallback_name);
            if !fallback_path.exists() {
                return fallback_path;
            }
            counter += 1;
        }
    }

    /// Perform standard cleanup (remove all build directory contents)
    fn standard_cleanup(&self) -> Result<()> {
        if self.build_path.exists() {
            fs::remove_dir_all(&self.build_path)
                .with_context(|| format!("Failed to remove build directory: {}", self.build_path.display()))?;
            println!("✓ Removed existing build directory");
        }

        fs::create_dir_all(&self.build_path)
            .with_context(|| format!("Failed to create build directory: {}", self.build_path.display()))?;
        println!("✓ Created clean build directory");

        Ok(())
    }

    /// Get path to backup directory
    fn get_backup_path(&self) -> PathBuf {
        Path::new(".").join(&self.backup_dir_name)
    }

    /// Cleanup backup directory
    fn cleanup_backup(&self) -> Result<()> {
        let backup_path = self.get_backup_path();
        if backup_path.exists() {
            fs::remove_dir_all(&backup_path)
                .with_context(|| format!("Failed to remove backup directory: {}", backup_path.display()))?;
            println!("✓ Cleaned up temporary backup directory");
        }
        Ok(())
    }
}

/// Result of restoring a single .env file
#[derive(Debug)]
pub enum RestoreResult {
    /// File restored to intended location
    Restored(PathBuf),
    /// File restored to fallback location due to conflicts or low confidence
    Fallback(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_file_pattern_matching() {
        let cleaner = BuildCleaner::new(
            "/tmp/test",
            true,
            vec![".env".to_string(), ".env.local".to_string(), ".env.production".to_string()],
        );

        assert!(cleaner.is_env_file(Path::new(".env")));
        assert!(cleaner.is_env_file(Path::new(".env.local")));
        assert!(cleaner.is_env_file(Path::new(".env.production")));
        assert!(!cleaner.is_env_file(Path::new("config.yml")));
        assert!(!cleaner.is_env_file(Path::new("docker-compose.yml")));
    }

    #[test]
    fn test_path_analysis() {
        let cleaner = BuildCleaner::new("/tmp/test", true, vec![".env".to_string()]);

        let (env, ext) = cleaner.analyze_env_file_path(Path::new("dev/auth/.env"));
        assert_eq!(env, Some("dev".to_string()));
        assert_eq!(ext, vec!["auth".to_string()]);

        let (env, ext) = cleaner.analyze_env_file_path(Path::new("production/base/.env"));
        assert_eq!(env, Some("production".to_string()));
        assert_eq!(ext, Vec::<String>::new());

        let (env, ext) = cleaner.analyze_env_file_path(Path::new("staging/monitoring/.env"));
        assert_eq!(env, Some("staging".to_string()));
        assert_eq!(ext, vec!["monitoring".to_string()]);
    }

    #[test]
    fn test_confidence_scoring() {
        let cleaner = BuildCleaner::new("/tmp/test", true, vec![".env".to_string()]);

        let file = PreservedEnvFile {
            original_path: PathBuf::from("dev/auth/.env"),
            content: "TEST=value".to_string(),
            environment: Some("dev".to_string()),
            extensions: vec!["auth".to_string()],
        };

        // Perfect match
        let confidence = cleaner.calculate_path_confidence(&file, "dev/auth");
        assert!(confidence > 0.7);

        // Partial match (environment only)
        let confidence = cleaner.calculate_path_confidence(&file, "dev/base");
        assert!(confidence > 0.4 && confidence < 0.8);

        // No match
        let confidence = cleaner.calculate_path_confidence(&file, "production/monitoring");
        assert!(confidence < 0.3);
    }
}