use std::process::{Command, Stdio};
use std::path::Path;
use crate::error::{Result, YamlError, BuildError};

/// Structure for managing docker-compose file merging process using yq
pub struct YqMerger {
    pub base_path: String,
    pub environments_path: String,
    pub extensions_paths: Vec<String>,
}

impl YqMerger {
    /// Create new YqMerger with given paths
    pub fn new(base_path: String, environments_path: String, extensions_paths: Vec<String>) -> Self {
        Self {
            base_path,
            environments_path,
            extensions_paths,
        }
    }
}

/// Check if yq is available in the system and get its version
pub fn check_yq_availability() -> Result<String> {
    let output = Command::new("yq")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| BuildError::BuildProcessFailed {
            details: format!(
                "yq command not found. Please install yq v4+ from https://github.com/mikefarah/yq\n\
                Installation instructions:\n\
                - Ubuntu/Debian: sudo apt install yq\n\
                - macOS: brew install yq\n\
                - Binary: wget https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -O /usr/bin/yq && chmod +x /usr/bin/yq\n\
                Error: {}", e
            ),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BuildError::BuildProcessFailed {
            details: format!(
                "yq command failed. Please ensure yq v4+ is properly installed.\n\
                Installation instructions: https://github.com/mikefarah/yq#install\n\
                Error: {}", stderr
            ),
        }.into());
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    
    // Check if it's yq v4+ (mikefarah's version)
    if !version_output.contains("mikefarah") && !version_output.starts_with("yq (https://github.com/mikefarah/yq/)") {
        return Err(BuildError::BuildProcessFailed {
            details: format!(
                "Wrong yq version detected. Please install yq v4+ from mikefarah (Go version).\n\
                Current version: {}\n\
                Required: yq v4+ from https://github.com/mikefarah/yq\n\
                Installation: https://github.com/mikefarah/yq#install", 
                version_output.trim()
            ),
        }.into());
    }

    println!("✓ yq version: {}", version_output.trim());
    Ok(version_output.trim().to_string())
}

/// Load and validate docker-compose.yml file using yq
pub fn yq_load_compose_file(file_path: &str) -> Result<()> {
    // First check if file exists
    if !Path::new(file_path).exists() {
        return Err(YamlError::ParseError {
            file: file_path.to_string(),
            details: "File does not exist".to_string(),
        }.into());
    }

    // Validate YAML syntax using yq
    let output = Command::new("yq")
        .arg("eval")
        .arg(".")
        .arg(file_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| YamlError::ParseError {
            file: file_path.to_string(),
            details: format!("Failed to execute yq: {}", e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(YamlError::ParseError {
            file: file_path.to_string(),
            details: format!("YAML syntax error: {}", stderr),
        }.into());
    }

    Ok(())
}

/// Validate docker-compose structure using yq
pub fn yq_validate_compose_structure(file_path: &str) -> Result<()> {
    // Check if services section exists
    let output = Command::new("yq")
        .arg("eval")
        .arg(".services")
        .arg(file_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| YamlError::InvalidComposeFormat {
            file: file_path.to_string(),
            details: format!("Failed to validate structure: {}", e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(YamlError::InvalidComposeFormat {
            file: file_path.to_string(),
            details: format!("Structure validation failed: {}", stderr),
        }.into());
    }

    let services_output = String::from_utf8_lossy(&output.stdout);
    
    // Check if services section is null or empty
    if services_output.trim() == "null" || services_output.trim().is_empty() {
        return Err(YamlError::InvalidComposeFormat {
            file: file_path.to_string(),
            details: "Missing required 'services' section in docker-compose file".to_string(),
        }.into());
    }

    println!("✓ Validated structure for: {}", file_path);
    Ok(())
}

/// Merge compose files using yq eval-all
pub fn yq_merge_compose_files(
    merger: &YqMerger,
    environment: Option<&str>,
    extensions: &[String],
) -> Result<String> {
    let file_paths = resolve_merge_order(merger, environment, extensions)?;
    
    if file_paths.is_empty() {
        return Err(YamlError::MergeError {
            details: "No files to merge".to_string(),
        }.into());
    }

    // Validate all files first
    let mut valid_files = Vec::new();
    let mut processed_files = 0;

    for file_path in file_paths {
        match yq_load_compose_file(&file_path) {
            Ok(_) => {
                match yq_validate_compose_structure(&file_path) {
                    Ok(_) => {
                        println!("✓ Loaded and validated: {}", file_path);
                        valid_files.push(file_path);
                        processed_files += 1;
                    }
                    Err(e) => {
                        // For base file, this is an error
                        if file_path.contains("/base/") {
                            return Err(e);
                        }
                        // For other files, skip with warning
                        println!("Warning: Skipping invalid file '{}': {}", file_path, e);
                        continue;
                    }
                }
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
        }
    }

    if processed_files == 0 {
        return Err(YamlError::MergeError {
            details: "No valid docker-compose files found to merge".to_string(),
        }.into());
    }

    // If only one file, just return its content
    if valid_files.len() == 1 {
        return yq_format_file(&valid_files[0]);
    }

    // Merge multiple files using yq eval-all
    let mut cmd = Command::new("yq");
    cmd.arg("eval-all")
        .arg(". as $item ireduce ({}; . *+ $item)")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Add all valid files as arguments
    for file_path in &valid_files {
        cmd.arg(file_path);
    }

    let output = cmd.output()
        .map_err(|e| YamlError::MergeError {
            details: format!("Failed to execute yq merge: {}", e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(YamlError::MergeError {
            details: format!("yq merge failed: {}", stderr),
        }.into());
    }

    let merged_content = String::from_utf8_lossy(&output.stdout);
    
    // Clean up null values and format
    let cleaned_content = clean_yaml_null_values(merged_content.to_string());
    
    Ok(cleaned_content)
}

/// Format YAML file using yq
pub fn yq_format_file(file_path: &str) -> Result<String> {
    let output = Command::new("yq")
        .arg("eval")
        .arg(".")
        .arg(file_path)
        .arg("--output-format")
        .arg("yaml")
        .arg("--indent")
        .arg("2")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| YamlError::SerializationError {
            details: format!("Failed to format YAML: {}", e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(YamlError::SerializationError {
            details: format!("yq format failed: {}", stderr),
        }.into());
    }

    let formatted_content = String::from_utf8_lossy(&output.stdout);
    let cleaned_content = clean_yaml_null_values(formatted_content.to_string());
    
    Ok(cleaned_content)
}

/// Resolve the order of files to merge based on environment and extensions
pub fn resolve_merge_order(
    merger: &YqMerger,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_yaml_null_values() {
        let input = "volumes:\n  data: ~\n  config: null\n  logs:\n";
        let expected = "volumes:\n  data:\n  config:\n  logs:\n";
        assert_eq!(clean_yaml_null_values(input.to_string()), expected);
    }

    #[test]
    fn test_resolve_merge_order() {
        let merger = YqMerger::new(
            "/components/base".to_string(),
            "/components/environments".to_string(),
            vec!["/components/extensions".to_string()],
        );
        
        let result = resolve_merge_order(&merger, Some("dev"), &["ext1".to_string()]).unwrap();
        
        assert_eq!(result.len(), 3);
        assert!(result[0].contains("base/docker-compose.yml"));
        assert!(result[1].contains("environments/dev/docker-compose.yml"));
        assert!(result[2].contains("extensions/ext1/docker-compose.yml"));
    }
}