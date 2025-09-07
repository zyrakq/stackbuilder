//! Tests for error handling

#[cfg(test)]
mod tests {
    use crate::error::*;

    #[test]
    fn test_error_exit_codes() {
        let config_error = StackBuilderError::Config(ConfigError::ConfigFileNotFound {
            file: "test.toml".to_string(),
        });
        assert_eq!(config_error.exit_code(), 1);

        let validation_error = StackBuilderError::Validation(ValidationError::NoTargetsSpecified);
        assert_eq!(validation_error.exit_code(), 2);

        let build_error = StackBuilderError::Build(BuildError::BuildProcessFailed {
            details: "test".to_string(),
        });
        assert_eq!(build_error.exit_code(), 3);

        let fs_error = StackBuilderError::FileSystem(FileSystemError::FileReadFailed {
            path: "/test".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
        });
        assert_eq!(fs_error.exit_code(), 4);

        let yaml_error = StackBuilderError::Yaml(YamlError::ParseError {
            file: "test.yml".to_string(),
            details: "test".to_string(),
        });
        assert_eq!(yaml_error.exit_code(), 5);

        let init_error = StackBuilderError::Init(InitError::ProjectStructureCreationFailed {
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "test"),
        });
        assert_eq!(init_error.exit_code(), 6);
    }

    #[test]
    fn test_error_suggests_init() {
        let config_error = StackBuilderError::Config(ConfigError::ConfigFileNotFound {
            file: "test.toml".to_string(),
        });
        assert!(config_error.suggests_init());

        let _validation_error = StackBuilderError::Validation(ValidationError::ComponentsDirectoryNotFound {
            path: "/test".into(),
        });
        assert!(config_error.suggests_init());

        let build_error = StackBuilderError::Build(BuildError::BuildProcessFailed {
            details: "test".to_string(),
        });
        assert!(!build_error.suggests_init());
    }

    #[test]
    fn test_error_suggestions() {
        let config_error = StackBuilderError::Config(ConfigError::ConfigFileNotFound {
            file: "test.toml".to_string(),
        });
        assert!(config_error.suggestion().is_some());
        assert!(config_error.suggestion().unwrap().contains("stackbuilder init"));

        let env_error = StackBuilderError::Validation(ValidationError::EnvironmentNotFound {
            name: "dev".to_string(),
            path: "/test".into(),
        });
        assert!(env_error.suggestion().is_some());
        assert!(env_error.suggestion().unwrap().contains("dev"));

        let build_error = StackBuilderError::Build(BuildError::BuildProcessFailed {
            details: "test".to_string(),
        });
        assert!(build_error.suggestion().is_none());
    }

    #[test]
    fn test_config_error_helpers() {
        let error = ConfigError::config_not_found("test.toml");
        assert!(error.to_string().contains("test.toml"));
        assert!(error.to_string().contains("not found"));

        // Create a simple TOML parse error by trying to parse invalid content
        let toml_result: std::result::Result<toml::Value, toml::de::Error> = toml::from_str("invalid toml [");
        if let Err(toml_error) = toml_result {
            let error = ConfigError::toml_parse_error("test.toml", toml_error);
            assert!(error.to_string().contains("test.toml"));
            assert!(error.to_string().contains("Invalid TOML syntax"));
        }
    }

    #[test]
    fn test_validation_error_helpers() {
        let error = ValidationError::environment_not_found("dev", "/test/envs");
        assert!(error.to_string().contains("dev"));
        assert!(error.to_string().contains("/test/envs"));

        let error = ValidationError::extension_not_found("monitoring", vec!["ext1".to_string(), "ext2".to_string()]);
        assert!(error.to_string().contains("monitoring"));
        assert!(error.to_string().contains("ext1"));
        assert!(error.to_string().contains("ext2"));
    }

    #[test]
    fn test_yaml_error_helpers() {
        let error = YamlError::parse_error("test.yml", "invalid syntax");
        assert!(error.to_string().contains("test.yml"));
        assert!(error.to_string().contains("invalid syntax"));

        // Create a serde_yaml error by trying to parse invalid YAML
        let yaml_result: std::result::Result<serde_yaml_ng::Value, serde_yaml_ng::Error> = serde_yaml_ng::from_str("invalid: yaml: [");
        if let Err(serde_error) = yaml_result {
            let error = YamlError::serde_error("test.yml", serde_error);
            assert!(error.to_string().contains("test.yml"));
        }
    }

    #[test]
    fn test_error_display() {
        let error = ConfigError::ConfigFileNotFound {
            file: "stackbuilder.toml".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("stackbuilder.toml"));
        assert!(display.contains("not found"));
        assert!(display.contains("stackbuilder init"));
    }

    #[test]
    fn test_error_chain() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let fs_error = FileSystemError::FileReadFailed {
            path: "/test/file".into(),
            source: io_error,
        };
        let stack_error = StackBuilderError::FileSystem(fs_error);
        
        let display = format!("{}", stack_error);
        assert!(display.contains("/test/file"));
        assert!(display.contains("Failed to read file"));
    }
}