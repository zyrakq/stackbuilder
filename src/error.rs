use std::path::PathBuf;
use thiserror::Error;

/// Main error type for stackbuilder application
#[derive(Error, Debug)]
pub enum StackBuilderError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    
    #[error(transparent)]
    Validation(#[from] ValidationError),
    
    #[error(transparent)]
    Build(#[from] BuildError),
    
    #[error(transparent)]
    FileSystem(#[from] FileSystemError),
    
    #[error(transparent)]
    Yaml(#[from] YamlError),
    
    #[error(transparent)]
    Init(#[from] InitError),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file '{file}' not found. Run 'stackbuilder init' to create a new project")]
    ConfigFileNotFound { file: String },
    
    #[error("Failed to read configuration file '{file}': {source}")]
    ConfigFileReadError { file: String, source: std::io::Error },
    
    #[error("Invalid TOML syntax in configuration file '{file}': {details}")]
    InvalidTomlSyntax { file: String, details: String },
    
    #[error("Failed to serialize configuration to TOML: {details}")]
    TomlSerializationError { details: String },
}

/// Validation-related errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Components directory '{path}' does not exist. Run 'stackbuilder init' to create project structure")]
    ComponentsDirectoryNotFound { path: PathBuf },
    
    #[error("Base directory '{path}' does not exist in components directory. Create base/docker-compose.yml file")]
    BaseDirectoryNotFound { path: PathBuf },
    
    #[cfg(test)]
    #[error("Environment '{name}' does not exist in environments directory '{path}'")]
    EnvironmentNotFound { name: String, path: PathBuf },
    
    #[error("Extension '{name}' not found in any extensions directory. Available directories: {available_dirs:?}")]
    ExtensionNotFound { name: String, available_dirs: Vec<String> },
    
    #[error("Combo '{combo_name}' not found in combo definitions. Available combos: {available_combos:?}")]
    ComboNotFound { combo_name: String, available_combos: Vec<String> },
    
    #[error("Invalid combo definition for '{combo_name}': {details}")]
    InvalidComboDefinition { combo_name: String, details: String },
    
    #[error("Invalid path resolution for '{path}': {details}")]
    PathResolutionError { path: String, details: String },
}

/// Build process errors
#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Failed to write merged docker-compose file to '{path}': {source}")]
    OutputFileWriteError { path: PathBuf, source: std::io::Error },
    
    #[error("Build process failed: {details}")]
    BuildProcessFailed { details: String },
}

/// File system operation errors
#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum FileSystemError {
    #[error("Failed to create directory '{path}': {source}")]
    DirectoryCreationFailed { path: PathBuf, source: std::io::Error },
    
    #[error("Failed to read directory '{path}': {source}")]
    DirectoryReadFailed { path: PathBuf, source: std::io::Error },
    
    #[error("Failed to read file '{path}': {source}")]
    FileReadFailed { path: PathBuf, source: std::io::Error },
    
    #[error("Failed to write file '{path}': {source}")]
    FileWriteFailed { path: PathBuf, source: std::io::Error },
}

/// YAML processing errors
#[derive(Error, Debug)]
pub enum YamlError {
    #[error("Failed to parse YAML file '{file}': {details}")]
    ParseError { file: String, details: String },
    
    #[error("YAML serialization failed: {details}")]
    SerializationError { details: String },
    
    #[error("YAML merge operation failed: {details}")]
    MergeError { details: String },
    
    #[error("Docker Compose file '{file}' has invalid format: {details}")]
    InvalidComposeFormat { file: String, details: String },
}

/// Initialization errors
#[derive(Error, Debug)]
pub enum InitError {
    #[error("Cannot create project structure: {source}")]
    ProjectStructureCreationFailed { source: std::io::Error },
    
    #[error("Failed to create example files: {details}")]
    ExampleFileCreationFailed { details: String },
}

impl StackBuilderError {
    /// Get the exit code for this error type
    pub fn exit_code(&self) -> i32 {
        match self {
            StackBuilderError::Config(_) => 1,
            StackBuilderError::Validation(_) => 2,
            StackBuilderError::Build(_) => 3,
            StackBuilderError::FileSystem(_) => 4,
            StackBuilderError::Yaml(_) => 5,
            StackBuilderError::Init(_) => 6,
        }
    }
    
    /// Check if this error suggests running init command
    pub fn suggests_init(&self) -> bool {
        matches!(
            self,
            StackBuilderError::Config(ConfigError::ConfigFileNotFound { .. }) |
            StackBuilderError::Validation(ValidationError::ComponentsDirectoryNotFound { .. }) |
            StackBuilderError::Validation(ValidationError::BaseDirectoryNotFound { .. })
        )
    }
    
    /// Get helpful suggestion for fixing this error
    pub fn suggestion(&self) -> Option<String> {
        match self {
            StackBuilderError::Config(ConfigError::ConfigFileNotFound { .. }) => {
                Some("Run 'stackbuilder init' to create a new project with default configuration".to_string())
            }
            StackBuilderError::Validation(ValidationError::ComponentsDirectoryNotFound { .. }) => {
                Some("Run 'stackbuilder init' to create the required project structure".to_string())
            }
            StackBuilderError::Validation(ValidationError::BaseDirectoryNotFound { .. }) => {
                Some("Create a base/docker-compose.yml file in your components directory".to_string())
            }
            StackBuilderError::Validation(ValidationError::ExtensionNotFound { name, .. }) => {
                Some(format!("Create an extension directory and docker-compose.yml file for '{}'", name))
            }
            StackBuilderError::Yaml(YamlError::InvalidComposeFormat { .. }) => {
                Some("Verify your docker-compose.yml files have valid YAML syntax and Docker Compose structure".to_string())
            }
            _ => None,
        }
    }
}

// Convenience type alias for Results
pub type Result<T> = std::result::Result<T, StackBuilderError>;

// Helper functions for creating common errors
impl ConfigError {
    pub fn config_not_found(file: impl Into<String>) -> Self {
        Self::ConfigFileNotFound { file: file.into() }
    }
    
    pub fn toml_parse_error(file: impl Into<String>, error: toml::de::Error) -> Self {
        Self::InvalidTomlSyntax {
            file: file.into(),
            details: error.to_string(),
        }
    }
    
    pub fn toml_serialize_error(error: toml::ser::Error) -> Self {
        Self::TomlSerializationError {
            details: error.to_string(),
        }
    }
}

impl ValidationError {
    #[cfg(test)]
    pub fn environment_not_found(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::EnvironmentNotFound {
            name: name.into(),
            path: path.into(),
        }
    }
    
    #[cfg(test)]
    pub fn extension_not_found(name: impl Into<String>, available_dirs: Vec<String>) -> Self {
        Self::ExtensionNotFound {
            name: name.into(),
            available_dirs,
        }
    }
}

impl YamlError {
    pub fn serde_error(file: impl Into<String>, error: serde_yaml_ng::Error) -> Self {
        Self::ParseError {
            file: file.into(),
            details: error.to_string(),
        }
    }
    
    #[cfg(test)]
    pub fn parse_error(file: impl Into<String>, details: impl Into<String>) -> Self {
        Self::ParseError {
            file: file.into(),
            details: details.into(),
        }
    }
}