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
    
    #[error("Configuration file '{file}' has invalid structure: {details}")]
    InvalidConfigStructure { file: String, details: String },
}

/// Validation-related errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Required directory '{path}' does not exist. Please create it or update your configuration")]
    DirectoryNotFound { path: PathBuf },
    
    #[error("Components directory '{path}' does not exist. Run 'stackbuilder init' to create project structure")]
    ComponentsDirectoryNotFound { path: PathBuf },
    
    #[error("Base directory '{path}' does not exist in components directory. Create base/docker-compose.yml file")]
    BaseDirectoryNotFound { path: PathBuf },
    
    #[error("Environment '{name}' does not exist in environments directory '{path}'")]
    EnvironmentNotFound { name: String, path: PathBuf },
    
    #[error("Extension '{name}' not found in any extensions directory. Available directories: {available_dirs:?}")]
    ExtensionNotFound { name: String, available_dirs: Vec<String> },
    
    #[error("Combo '{combo_name}' not found in combo definitions. Available combos: {available_combos:?}")]
    ComboNotFound { combo_name: String, available_combos: Vec<String> },
    
    #[error("Invalid combo definition for '{combo_name}': {details}")]
    InvalidComboDefinition { combo_name: String, details: String },
    
    #[error("Configuration must specify at least one environment or extension to build")]
    NoTargetsSpecified,
    
    #[error("Invalid path resolution for '{path}': {details}")]
    PathResolutionError { path: String, details: String },
}

/// Build process errors
#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Build directory '{path}' cannot be created: {source}")]
    BuildDirectoryCreationFailed { path: PathBuf, source: std::io::Error },
    
    #[error("Failed to clean build directory '{path}': {source}")]
    BuildDirectoryCleanupFailed { path: PathBuf, source: std::io::Error },
    
    #[error("No valid docker-compose files found to merge. Check your component structure")]
    NoValidFilesToMerge,
    
    #[error("Failed to write merged docker-compose file to '{path}': {source}")]
    OutputFileWriteError { path: PathBuf, source: std::io::Error },
    
    #[error("Build process failed: {details}")]
    BuildProcessFailed { details: String },
    
    #[error("Invalid build combination: environment='{env:?}', extensions={extensions:?}")]
    InvalidBuildCombination { env: Option<String>, extensions: Vec<String> },
}

/// File system operation errors
#[derive(Error, Debug)]
pub enum FileSystemError {
    #[error("Failed to create directory '{path}': {source}")]
    DirectoryCreationFailed { path: PathBuf, source: std::io::Error },
    
    #[error("Failed to read directory '{path}': {source}")]
    DirectoryReadFailed { path: PathBuf, source: std::io::Error },
    
    #[error("Failed to read file '{path}': {source}")]
    FileReadFailed { path: PathBuf, source: std::io::Error },
    
    #[error("Failed to write file '{path}': {source}")]
    FileWriteFailed { path: PathBuf, source: std::io::Error },
    
    #[error("Permission denied for file operation on '{path}'. Check file permissions")]
    PermissionDenied { path: PathBuf },
    
    #[error("File '{path}' already exists")]
    FileAlreadyExists { path: PathBuf },
}

/// YAML processing errors
#[derive(Error, Debug)]
pub enum YamlError {
    #[error("Failed to parse YAML file '{file}': {details}")]
    ParseError { file: String, details: String },
    
    #[error("Invalid YAML structure in file '{file}': {details}")]
    InvalidYamlStructure { file: String, details: String },
    
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
    #[error("Failed to initialize project: {details}")]
    InitializationFailed { details: String },
    
    #[error("Project already exists at current location. Use --force to overwrite")]
    ProjectAlreadyExists,
    
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
            StackBuilderError::Validation(ValidationError::EnvironmentNotFound { name, .. }) => {
                Some(format!("Create an environment directory and docker-compose.yml file for '{}'", name))
            }
            StackBuilderError::Validation(ValidationError::ExtensionNotFound { name, .. }) => {
                Some(format!("Create an extension directory and docker-compose.yml file for '{}'", name))
            }
            StackBuilderError::FileSystem(FileSystemError::PermissionDenied { .. }) => {
                Some("Check file permissions and ensure you have write access to the directory".to_string())
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
    pub fn directory_not_found(path: impl Into<PathBuf>) -> Self {
        Self::DirectoryNotFound { path: path.into() }
    }
    
    pub fn environment_not_found(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::EnvironmentNotFound {
            name: name.into(),
            path: path.into(),
        }
    }
    
    pub fn extension_not_found(name: impl Into<String>, available_dirs: Vec<String>) -> Self {
        Self::ExtensionNotFound {
            name: name.into(),
            available_dirs,
        }
    }
}

impl FileSystemError {
    pub fn from_io_error(path: impl Into<PathBuf>, error: std::io::Error) -> Self {
        let path = path.into();
        match error.kind() {
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied { path },
            std::io::ErrorKind::AlreadyExists => Self::FileAlreadyExists { path },
            _ => Self::FileReadFailed { path, source: error },
        }
    }
}

impl YamlError {
    pub fn parse_error(file: impl Into<String>, error: impl std::fmt::Display) -> Self {
        Self::ParseError {
            file: file.into(),
            details: error.to_string(),
        }
    }
    
    pub fn serde_error(file: impl Into<String>, error: serde_yaml::Error) -> Self {
        Self::ParseError {
            file: file.into(),
            details: error.to_string(),
        }
    }
}