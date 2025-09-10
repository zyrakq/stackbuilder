use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Structure for managing build directory cleaning with .env file preservation
pub struct BuildCleaner {
    /// Path to the build directory
    build_path: PathBuf,
    /// Configuration for env file preservation
    preserve_env_files: bool,
    /// Patterns for env files to preserve
    env_file_patterns: Vec<String>,
    /// Backup directory path (configured in stackbuilder.toml)
    backup_dir: PathBuf,
    /// In-memory storage for .env files during build process
    preserved_files: std::cell::RefCell<Option<Vec<PreservedEnvFile>>>,
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
        backup_dir: String,
    ) -> Self {
        Self {
            build_path: build_path.as_ref().to_path_buf(),
            preserve_env_files,
            env_file_patterns,
            backup_dir: PathBuf::from(backup_dir),
            preserved_files: std::cell::RefCell::new(None),
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

        // Step 2: Store .env files in memory only (no backup to disk yet)
        self.store_env_files_in_memory(&scan_result.files);

        // Step 3: Clean build directory
        self.standard_cleanup()
            .context("Failed to clean build directory")?;

        println!("✓ Build directory cleaned, .env files preserved in memory for restoration");
        
        Ok(())
    }

    /// Restore preserved .env files to new build structure
    pub fn restore_env_files(&self, new_structure: &[String]) -> Result<()> {
        if !self.preserve_env_files {
            return Ok(());
        }

        // Get preserved files from memory
        let preserved_files_opt = self.preserved_files.borrow().clone();
        let preserved_files = match preserved_files_opt {
            Some(files) => files,
            None => {
                println!("No .env files were preserved, skipping restoration");
                return Ok(());
            }
        };

        if preserved_files.is_empty() {
            println!("No preserved .env files to restore");
            return Ok(());
        }

        println!("Restoring preserved .env files to new build structure");

        // Generate path mappings
        let mappings = self.generate_path_mappings(&preserved_files, new_structure)
            .context("Failed to generate path mappings")?;

        // Try to restore files according to mappings
        let mut restored_count = 0;
        let mut failed_files = Vec::new();

        for preserved_file in &preserved_files {
            let restore_result = self.restore_single_file(preserved_file, &mappings);
            
            match restore_result {
                Ok(RestoreResult::Restored(path)) => {
                    println!("✓ Restored .env file to: {}", path.display());
                    restored_count += 1;
                }
                Ok(RestoreResult::SkippedNoMatch) => {
                    println!("ℹ Skipped .env file (no matching structure): {}", preserved_file.original_path.display());
                    failed_files.push(preserved_file.clone());
                }
                Ok(RestoreResult::SkippedConflict) => {
                    println!("⚠ Skipped .env file (content conflict): {}", preserved_file.original_path.display());
                    failed_files.push(preserved_file.clone());
                }
                Err(e) => {
                    println!("✗ Failed to restore .env file from {}: {}",
                            preserved_file.original_path.display(), e);
                    failed_files.push(preserved_file.clone());
                }
            }
        }

        // Only create backup if some files couldn't be restored
        if !failed_files.is_empty() {
            println!("Creating backup for {} files that couldn't be restored", failed_files.len());
            self.create_backup_for_failed_files(&failed_files)
                .context("Failed to create backup for failed files")?;
        }

        println!("Restoration completed: {} restored successfully", restored_count);
        if !failed_files.is_empty() {
            println!("ℹ {} files backed up to: {}", failed_files.len(), self.backup_dir.display());
        }

        // Clear memory storage
        *self.preserved_files.borrow_mut() = None;

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
                if path == self.backup_dir {
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

    /// Store .env files in memory for temporary preservation during build
    fn store_env_files_in_memory(&self, files: &[PreservedEnvFile]) {
        *self.preserved_files.borrow_mut() = Some(files.to_vec());
        println!("✓ Stored {} .env files in memory for restoration", files.len());
    }

    /// Create backup only for files that couldn't be restored
    fn create_backup_for_failed_files(&self, files: &[PreservedEnvFile]) -> Result<()> {
        let backup_path = self.get_backup_path();
        
        // Create backup directory
        fs::create_dir_all(&backup_path)
            .with_context(|| format!("Failed to create backup directory: {}", backup_path.display()))?;

        // Save metadata
        let metadata_path = backup_path.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(files)
            .context("Failed to serialize .env files metadata")?;
        
        fs::write(&metadata_path, metadata_json)
            .with_context(|| format!("Failed to write metadata file: {}", metadata_path.display()))?;

        // Save files with full path as filename (replacing / and \ with _)
        for file in files.iter() {
            let safe_filename = file.original_path.to_string_lossy()
                .replace(['/', '\\'], "_");
            let backup_file_path = backup_path.join(&safe_filename);
            
            fs::write(&backup_file_path, &file.content)
                .with_context(|| format!("Failed to backup .env file: {}", backup_file_path.display()))?;
            
            println!("  Backed up: {} -> {}", file.original_path.display(), safe_filename);
        }

        println!("✓ Created backup for {} .env files: {}", files.len(), backup_path.display());
        Ok(())
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

    /// Find best path mapping for a preserved .env file - only restore to exact original structure
    fn find_best_path_mapping(&self, file: &PreservedEnvFile, new_structure: &[String]) -> PathMapping {
        // Try to find exact match in new structure
        for new_path_str in new_structure {
            let expected_dir = if let Some(parent) = file.original_path.parent() {
                parent.to_string_lossy().to_string()
            } else {
                String::new()
            };
            
            // Check if this new path matches the expected directory structure
            if new_path_str == &expected_dir || (expected_dir.is_empty() && new_path_str.is_empty()) {
                let filename = file.original_path.file_name()
                    .unwrap_or_default().to_string_lossy();
                let new_path = if new_path_str.is_empty() {
                    PathBuf::from(filename.as_ref())
                } else {
                    PathBuf::from(new_path_str).join(filename.as_ref())
                };
                
                return PathMapping {
                    old_path: file.original_path.clone(),
                    new_path,
                    confidence: 1.0, // Exact match
                };
            }
        }
        
        // No exact match found - file will remain in backup
        PathMapping {
            old_path: file.original_path.clone(),
            new_path: file.original_path.clone(), // Will not be used
            confidence: 0.0, // No restoration possible
        }
    }


    /// Restore a single .env file - only to exact original location, no fallbacks in build
    fn restore_single_file(
        &self,
        file: &PreservedEnvFile,
        mappings: &[PathMapping]
    ) -> Result<RestoreResult> {
        // Find mapping for this file
        let mapping = mappings.iter()
            .find(|m| m.old_path == file.original_path)
            .ok_or_else(|| anyhow::anyhow!("No mapping found for file: {}", file.original_path.display()))?;

        // Only restore if we have high confidence (exact match)
        if mapping.confidence < 1.0 {
            return Ok(RestoreResult::SkippedNoMatch);
        }

        let target_path = self.build_path.join(&mapping.new_path);

        // Create parent directories
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Check for conflicts - if file exists and differs, keep existing and skip restoration
        if target_path.exists() {
            let existing_content = fs::read_to_string(&target_path)
                .context("Failed to read existing .env file")?;
            
            if existing_content != file.content {
                return Ok(RestoreResult::SkippedConflict);
            }
        }

        // Write file to target location
        fs::write(&target_path, &file.content)
            .with_context(|| format!("Failed to write .env file: {}", target_path.display()))?;

        Ok(RestoreResult::Restored(target_path))
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

    /// Get path to backup directory with timestamp
    fn get_backup_path(&self) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.backup_dir.join(format!("backup_{}", timestamp))
    }

}

/// Result of restoring a single .env file
#[derive(Debug)]
pub enum RestoreResult {
    /// File restored to intended location
    Restored(PathBuf),
    /// File skipped due to no matching structure in new build
    SkippedNoMatch,
    /// File skipped due to conflict with existing file
    SkippedConflict,
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
            "/tmp/backup".to_string(),
        );

        assert!(cleaner.is_env_file(Path::new(".env")));
        assert!(cleaner.is_env_file(Path::new(".env.local")));
        assert!(cleaner.is_env_file(Path::new(".env.production")));
        assert!(!cleaner.is_env_file(Path::new("config.yml")));
        assert!(!cleaner.is_env_file(Path::new("docker-compose.yml")));
    }

    #[test]
    fn test_path_analysis() {
        let cleaner = BuildCleaner::new("/tmp/test", true, vec![".env".to_string()], "/tmp/backup".to_string());

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
    fn test_path_mapping() {
        let cleaner = BuildCleaner::new("/tmp/test", true, vec![".env".to_string()], "/tmp/backup".to_string());

        let file = PreservedEnvFile {
            original_path: PathBuf::from("dev/auth/.env"),
            content: "TEST=value".to_string(),
            environment: Some("dev".to_string()),
            extensions: vec!["auth".to_string()],
        };

        let new_structure = vec!["dev/auth".to_string(), "dev/base".to_string(), "production/monitoring".to_string()];

        // Perfect match
        let mapping = cleaner.find_best_path_mapping(&file, &new_structure);
        assert_eq!(mapping.confidence, 1.0);
        assert_eq!(mapping.new_path, PathBuf::from("dev/auth/.env"));

        // No match case
        let file_no_match = PreservedEnvFile {
            original_path: PathBuf::from("staging/auth/.env"),
            content: "TEST=value".to_string(),
            environment: Some("staging".to_string()),
            extensions: vec!["auth".to_string()],
        };
        
        let mapping_no_match = cleaner.find_best_path_mapping(&file_no_match, &new_structure);
        assert_eq!(mapping_no_match.confidence, 0.0);
    }

    #[test]
    fn test_in_memory_storage() {
        let cleaner = BuildCleaner::new("/tmp/test", true, vec![".env".to_string()], "/tmp/backup".to_string());

        let files = vec![
            PreservedEnvFile {
                original_path: PathBuf::from(".env"),
                content: "TEST=value".to_string(),
                environment: None,
                extensions: vec![],
            }
        ];

        // Test storing in memory
        cleaner.store_env_files_in_memory(&files);
        
        // Test retrieving from memory
        let stored = cleaner.preserved_files.borrow().clone();
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().len(), 1);
    }

    #[test]
    fn test_backup_path_generation() {
        let cleaner = BuildCleaner::new("/tmp/test", true, vec![".env".to_string()], "/tmp/backup".to_string());
        let backup_path = cleaner.get_backup_path();
        
        // Should be in the format /tmp/backup/backup_TIMESTAMP
        assert!(backup_path.to_string_lossy().starts_with("/tmp/backup/backup_"));
    }

    #[test]
    fn test_restore_result_enum() {
        // Test that the enum variants work correctly
        let restored = RestoreResult::Restored(PathBuf::from("/test/path"));
        let skipped_no_match = RestoreResult::SkippedNoMatch;
        let skipped_conflict = RestoreResult::SkippedConflict;
        
        match restored {
            RestoreResult::Restored(path) => assert_eq!(path, PathBuf::from("/test/path")),
            _ => panic!("Expected Restored variant"),
        }
        
        match skipped_no_match {
            RestoreResult::SkippedNoMatch => assert!(true),
            _ => panic!("Expected SkippedNoMatch variant"),
        }
        
        match skipped_conflict {
            RestoreResult::SkippedConflict => assert!(true),
            _ => panic!("Expected SkippedConflict variant"),
        }
    }

    #[test]
    fn test_empty_structure_restoration() {
        let cleaner = BuildCleaner::new("/tmp/test", true, vec![".env".to_string()], "/tmp/backup".to_string());
        
        let file = PreservedEnvFile {
            original_path: PathBuf::from(".env"),
            content: "TEST=value".to_string(),
            environment: None,
            extensions: vec![],
        };

        // Test restoration with empty structure (should match root file)
        let empty_structure = vec!["".to_string()];
        let mapping = cleaner.find_best_path_mapping(&file, &empty_structure);
        assert_eq!(mapping.confidence, 1.0);
        assert_eq!(mapping.new_path, PathBuf::from(".env"));
    }
}