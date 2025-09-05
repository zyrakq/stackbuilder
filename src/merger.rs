use std::fs;
use std::path::Path;
use serde_yaml::Value;
use crate::error::{Result, YamlError, FileSystemError};

/// Structure for managing docker-compose file merging process
pub struct ComposeMerger {
    pub base_path: String,
    pub environments_path: String,
    pub extensions_paths: Vec<String>,
}

impl ComposeMerger {
    /// Create new ComposeMerger with given paths
    pub fn new(base_path: String, environments_path: String, extensions_paths: Vec<String>) -> Self {
        Self {
            base_path,
            environments_path,
            extensions_paths,
        }
    }
}

/// Load and parse docker-compose.yml file from given path
pub fn load_compose_file(file_path: &str) -> Result<Value> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| FileSystemError::FileReadFailed {
            path: file_path.into(),
            source: e,
        })?;

    let yaml_value: Value = serde_yaml::from_str(&content)
        .map_err(|e| YamlError::serde_error(file_path, e))?;

    // Validate basic docker-compose structure
    if let Value::Mapping(ref map) = yaml_value {
        if !map.contains_key(Value::String("services".to_string())) {
            return Err(YamlError::InvalidComposeFormat {
                file: file_path.to_string(),
                details: "Missing required 'services' section in docker-compose file".to_string(),
            }.into());
        }
    } else {
        return Err(YamlError::InvalidComposeFormat {
            file: file_path.to_string(),
            details: "Docker Compose file must be a YAML mapping/object".to_string(),
        }.into());
    }

    Ok(yaml_value)
}

/// Recursively merge YAML values with priority logic
/// Later values overwrite earlier ones for objects, primitives, and append for arrays
pub fn merge_yaml_values(base: Value, override_: Value) -> Value {
    match (base, override_) {
        (Value::Mapping(mut base_map), Value::Mapping(override_map)) => {
            // For objects, merge recursively and allow overrides
            for (key, value) in override_map {
                if let Some(base_value) = base_map.get(&key) {
                    base_map.insert(key, merge_yaml_values(base_value.clone(), value));
                } else {
                    base_map.insert(key, value);
                }
            }
            Value::Mapping(base_map)
        }
        (Value::Sequence(mut base_seq), Value::Sequence(override_seq)) => {
            // For arrays, append override values (no removal of base elements)
            base_seq.extend(override_seq);
            Value::Sequence(base_seq)
        }
        // For primitives or other types, override completely
        (_, override_val) => override_val,
    }
}

/// Merge compose files in priority order: base -> environment -> extensions
pub fn merge_compose_files(
    merger: &ComposeMerger,
    environment: Option<&str>,
    extensions: &[String],
) -> Result<Value> {
    let file_paths = resolve_merge_order(merger, environment, extensions)?;

    let mut merged: Option<Value> = None;
    let mut processed_files = 0;

    for file_path in file_paths {
        let yaml_value = match load_compose_file(&file_path) {
            Ok(val) => {
                println!("Loaded and merging: {}", file_path);
                processed_files += 1;
                val
            }
            Err(e) => {
                // For base file, this is an error
                if file_path.contains("/base/") {
                    return Err(e);
                }
                // For other files, skip with warning
                println!("Warning: Skipping missing or invalid file '{}': {}", file_path, e);
                continue;
            }
        };

        if let Some(current) = merged {
            merged = Some(merge_yaml_values(current, yaml_value));
        } else {
            merged = Some(yaml_value);
        }
    }

    if processed_files == 0 {
        return Err(YamlError::MergeError {
            details: "No valid docker-compose files found to merge".to_string(),
        }.into());
    }

    merged.ok_or_else(|| YamlError::MergeError {
        details: "Failed to merge docker-compose files".to_string(),
    }.into())
}

/// Parse extension combination string like "oidc+guard" into vec of strings
pub fn parse_extension_combination(combo: &str) -> Vec<String> {
    combo.split('+').map(|s| s.trim().to_string()).collect()
}

/// Resolve the order of files to merge based on environment and extensions
pub fn resolve_merge_order(
    merger: &ComposeMerger,
    environment: Option<&str>,
    extensions: &[String],
) -> Result<Vec<String>> {
    let mut file_paths = Vec::new();

    // Always start with base
    let base_file = Path::new(&merger.base_path).join("docker-compose.yml");
    file_paths.push(base_file.to_string_lossy().to_string());

    // Add environment file if specified
    if let Some(env) = environment {
        let env_file = Path::new(&merger.environments_path)
            .join(env)
            .join("docker-compose.yml");
        file_paths.push(env_file.to_string_lossy().to_string());
    }

    // Add extension files in order
    for ext in extensions {
        let mut found = false;
        for ext_dir in &merger.extensions_paths {
            let ext_file = Path::new(ext_dir).join(ext).join("docker-compose.yml");
            if ext_file.exists() {
                file_paths.push(ext_file.to_string_lossy().to_string());
                found = true;
                break; // Found in first matching directory
            }
        }
        
        if !found {
            println!("Warning: Extension '{}' not found in any extensions directory", ext);
        }
    }

    Ok(file_paths)
}

/// Build file paths from components
pub fn build_file_paths(
    root_dir: &str,
    environment: Option<&str>,
    extensions: &[String],
) -> Result<Vec<String>> {
    let mut paths = Vec::new();

    // Base path
    paths.push(format!("{}/base/docker-compose.yml", root_dir));

    // Environment path
    if let Some(env) = environment {
        paths.push(format!("{}/environments/{}/docker-compose.yml", root_dir, env));
    }

    // Extension paths
    for ext in extensions {
        paths.push(format!("{}/extensions/{}/docker-compose.yml", root_dir, ext));
    }

    Ok(paths)
}