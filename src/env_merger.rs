use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use crate::error::{Result, FileSystemError};

/// Structure for managing .env.example file merging process
pub struct EnvMerger {
    pub base_path: String,
    pub environments_path: String,
    pub extensions_paths: Vec<String>,
}

impl EnvMerger {
    /// Create new EnvMerger with given paths
    pub fn new(base_path: String, environments_path: String, extensions_paths: Vec<String>) -> Self {
        Self {
            base_path,
            environments_path,
            extensions_paths,
        }
    }
}

/// Structure representing a parsed .env file with variables and comments
#[derive(Debug, Clone)]
pub struct EnvFile {
    pub variables: BTreeMap<String, String>,
    pub variable_comments: BTreeMap<String, Vec<String>>, // Comments for each variable
    pub header_comments: Vec<String>, // General file comments
    pub source_file: String,
}

impl EnvFile {
    pub fn new(source_file: String) -> Self {
        Self {
            variables: BTreeMap::new(),
            variable_comments: BTreeMap::new(),
            header_comments: Vec::new(),
            source_file,
        }
    }
}

/// Parse .env.example file and extract variables and comments
pub fn parse_env_file(file_path: &str) -> Result<EnvFile> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| FileSystemError::FileReadFailed {
            path: file_path.into(),
            source: e,
        })?;

    let mut env_file = EnvFile::new(file_path.to_string());
    let mut current_comments = Vec::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }
        
        // Handle comments
        if trimmed.starts_with('#') {
            current_comments.push(trimmed.to_string());
            continue;
        }
        
        // Parse variable assignment
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let value = trimmed[eq_pos + 1..].trim().to_string();
            
            // Remove quotes if present
            let value = if (value.starts_with('"') && value.ends_with('"')) ||
                          (value.starts_with('\'') && value.ends_with('\'')) {
                value[1..value.len()-1].to_string()
            } else {
                value
            };
            
            // Associate comments with this variable
            if !current_comments.is_empty() {
                env_file.variable_comments.insert(key.clone(), current_comments.clone());
                current_comments.clear();
            }
            
            env_file.variables.insert(key, value);
        }
    }
    
    // Remaining comments are header comments
    if !current_comments.is_empty() {
        env_file.header_comments.extend(current_comments);
    }

    println!("Parsed .env file: {} with {} variables and {} comment groups",
             file_path, env_file.variables.len(), env_file.variable_comments.len());
    
    Ok(env_file)
}

/// Merge .env.example files in priority order: base -> environment -> extensions
pub fn merge_env_files(
    merger: &EnvMerger,
    environment: Option<&str>,
    extensions: &[String],
) -> Result<EnvFile> {
    let file_paths = resolve_env_merge_order(merger, environment, extensions)?;
    
    let mut merged_variables = BTreeMap::new();
    let mut merged_variable_comments = BTreeMap::new();
    let mut source_files = Vec::new();
    let mut processed_files = 0;
    
    for file_path in file_paths {
        let env_file = match parse_env_file(&file_path) {
            Ok(file) => {
                println!("Loaded and merging .env file: {}", file_path);
                processed_files += 1;
                source_files.push(get_source_name(&file_path));
                file
            }
            Err(e) => {
                // For base file, this is an error
                if file_path.contains("/base/") {
                    return Err(e);
                }
                // For other files, skip with warning
                println!("Warning: Skipping missing .env.example file '{}': {}", file_path, e);
                continue;
            }
        };

        // Merge variables and their comments (later files override earlier ones)
        for (key, value) in env_file.variables {
            merged_variables.insert(key.clone(), value);
            
            // If this variable has comments, store them
            if let Some(comments) = env_file.variable_comments.get(&key) {
                merged_variable_comments.insert(key, comments.clone());
            }
        }
    }

    if processed_files == 0 {
        println!("Warning: No .env.example files found to merge");
        return Ok(EnvFile::new("merged".to_string()));
    }

    let mut merged_file = EnvFile::new("merged".to_string());
    merged_file.variables = merged_variables;
    merged_file.variable_comments = merged_variable_comments;
    
    // Set header comments
    merged_file.header_comments.push("# Generated by stackbuilder from multiple .env.example files".to_string());
    if !source_files.is_empty() {
        merged_file.header_comments.push(format!("# Source files: {}", source_files.join(", ")));
    }

    println!("Successfully merged {} .env.example files with {} total variables",
             processed_files, merged_file.variables.len());
    
    Ok(merged_file)
}

/// Write merged .env.example file to specified path
pub fn write_merged_env(env_file: &EnvFile, output_path: &str) -> Result<()> {
    let mut content = String::new();

    // Write header comments first
    for comment in &env_file.header_comments {
        content.push_str(comment);
        content.push('\n');
    }
    
    // Add separator if we have header comments
    if !env_file.header_comments.is_empty() {
        content.push('\n');
    }

    // Write variables in sorted order with their associated comments
    for (key, value) in &env_file.variables {
        // Write comments for this variable first
        if let Some(comments) = env_file.variable_comments.get(key) {
            for comment in comments {
                content.push_str(comment);
                content.push('\n');
            }
        }
        
        // Quote values that contain spaces or special characters
        let quoted_value = if value.contains(' ') ||
                             value.contains('#') ||
                             value.contains('$') ||
                             value.is_empty() {
            format!("\"{}\"", value)
        } else {
            value.clone()
        };
        
        content.push_str(&format!("{}={}\n", key, quoted_value));
        
        // Add empty line after each variable for readability
        content.push('\n');
    }

    fs::write(output_path, content)
        .map_err(|e| FileSystemError::FileWriteFailed {
            path: output_path.into(),
            source: e,
        })?;

    println!("✓ Created merged .env.example file: {}", output_path);
    Ok(())
}

/// Resolve the order of .env.example files to merge
fn resolve_env_merge_order(
    merger: &EnvMerger,
    environment: Option<&str>,
    extensions: &[String],
) -> Result<Vec<String>> {
    let mut file_paths = Vec::new();

    // Always start with base
    let base_file = Path::new(&merger.base_path).join(".env.example");
    file_paths.push(base_file.to_string_lossy().to_string());

    // Add environment file if specified
    if let Some(env) = environment {
        let env_file = Path::new(&merger.environments_path)
            .join(env)
            .join(".env.example");
        file_paths.push(env_file.to_string_lossy().to_string());
    }

    // Add extension files in order
    for ext in extensions {
        let mut found = false;
        for ext_dir in &merger.extensions_paths {
            let ext_file = Path::new(ext_dir).join(ext).join(".env.example");
            if ext_file.exists() {
                file_paths.push(ext_file.to_string_lossy().to_string());
                found = true;
                break; // Found in first matching directory
            }
        }
        
        if !found {
            println!("Warning: .env.example for extension '{}' not found in any extensions directory", ext);
        }
    }

    Ok(file_paths)
}

/// Extract readable source name from file path
fn get_source_name(file_path: &str) -> String {
    let path = Path::new(file_path);
    
    // Get parent directory name
    if let Some(parent) = path.parent() {
        if let Some(dir_name) = parent.file_name() {
            if let Some(name) = dir_name.to_str() {
                return match name {
                    "base" => "base/.env.example".to_string(),
                    other => format!("{}/.env.example", other),
                };
            }
        }
    }
    
    // Fallback to full path
    file_path.to_string()
}