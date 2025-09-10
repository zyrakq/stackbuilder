use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use crate::error::{Result, FileSystemError};

/// Structure for managing .env.example file merging process
#[derive(Debug)]
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
    pub variables: Vec<(String, String)>, // Use Vec to preserve order
    pub variable_comments: BTreeMap<String, Vec<String>>, // Comments for each variable
    pub header_comments: Vec<String>, // General file comments
    pub variable_order: Vec<String>, // Track order of variable names
}

impl EnvFile {
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            variable_comments: BTreeMap::new(),
            header_comments: Vec::new(),
            variable_order: Vec::new(),
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

    let mut env_file = EnvFile::new();
    let mut comment_group_accumulator = Vec::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Handle comments
        if trimmed.starts_with('#') {
            if !comment_group_accumulator.is_empty() {
                comment_group_accumulator.push("".to_string()); // Separator between comment groups
            }
            comment_group_accumulator.push(trimmed.to_string());
            continue;
        }
        
        // Handle empty lines - preserve as comment separators
        if trimmed.is_empty() {
            if !comment_group_accumulator.is_empty() {
                comment_group_accumulator.push("".to_string());
            }
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
            
            // Associate accumulated comments with this variable
            if !comment_group_accumulator.is_empty() {
                env_file.variable_comments.insert(key.clone(), comment_group_accumulator.clone());
                comment_group_accumulator.clear();
            }
            
            // Store variable in order and track its name
            env_file.variables.push((key.clone(), value));
            env_file.variable_order.push(key);
        }
    }
    
    // Remaining comments are header comments
    if !comment_group_accumulator.is_empty() {
        env_file.header_comments.extend(comment_group_accumulator);
    }

    println!("Parsed .env file: {} with {} variables and {} comment groups",
             file_path, env_file.variables.len(), env_file.variable_comments.len());
    
    Ok(env_file)
}

/// Concatenate .env.example files in specified order: base -> environment -> extensions
pub fn merge_env_files(
    merger: &EnvMerger,
    environment: Option<&str>,
    extensions: &[String],
) -> Result<EnvFile> {
    let file_paths = resolve_env_merge_order(merger, environment, extensions)?;
    
    let mut all_content = String::new();
    let mut source_files = Vec::new();
    let mut processed_files = 0;
    
    for file_path in file_paths {
        match fs::read_to_string(&file_path) {
            Ok(content) => {
                println!("Loaded and concatenating .env file: {}", file_path);
                processed_files += 1;
                source_files.push(get_source_name(&file_path));
                
                // Add file content with separator
                if !all_content.is_empty() && !all_content.ends_with('\n') {
                    all_content.push('\n'); // Add separator between files if needed
                }
                all_content.push_str(&content);
            }
            Err(e) => {
                // For base file, this is an error
                if file_path.contains("/base/") {
                    return Err(FileSystemError::FileReadFailed {
                        path: file_path.into(),
                        source: e,
                    }.into());
                }
                // For other files, skip with warning
                println!("Warning: Skipping missing .env.example file '{}': {}", file_path, e);
                continue;
            }
        }
    }

    if processed_files == 0 {
        println!("Warning: No .env.example files found to concatenate");
        return Ok(EnvFile::new());
    }

    // Create simple structure with all content
    let mut env_file = EnvFile::new();
    env_file.header_comments.push("# Generated by stackbuilder from concatenated .env.example files".to_string());
    if !source_files.is_empty() {
        env_file.header_comments.push(format!("# Source files: {}", source_files.join(", ")));
    }

    // Store the entire concatenated content as single lines
    for line in all_content.lines() {
        env_file.variables.push((format!("line_{}", env_file.variables.len()), line.to_string()));
        env_file.variable_order.push(format!("line_{}", env_file.variable_order.len()));
    }

    println!("Successfully concatenated {} .env.example files with {} total lines",
             processed_files, env_file.variables.len());
    
    Ok(env_file)
}

/// Write concatenated .env.example file to specified path
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

    // Simply write all lines in order as they appeared in original files
    for (_, line_content) in &env_file.variables {
        content.push_str(line_content);
        content.push('\n');
    }

    // Remove trailing newline if present
    if content.ends_with('\n') {
        content.pop();
    }

    fs::write(output_path, content)
        .map_err(|e| FileSystemError::FileWriteFailed {
            path: output_path.into(),
            source: e,
        })?;

    println!("✓ Created concatenated .env.example file: {}", output_path);
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