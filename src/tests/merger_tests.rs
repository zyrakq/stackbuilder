//! Tests for YAML merging functionality

#[cfg(test)]
mod tests {
    use crate::merger::*;
    use crate::tests::*;
    use serde_yaml::Value;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_load_compose_file_valid() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let compose_path = temp_dir.path().join("docker-compose.yml");
        
        let compose_content = r#"
version: '3.8'
services:
  web:
    image: nginx:alpine
    ports:
      - "80:80"
"#;
        
        fs::write(&compose_path, compose_content).expect("Failed to write compose file");
        
        let result = load_compose_file(compose_path.to_str().unwrap());
        assert!(result.is_ok());
        
        let yaml = result.unwrap();
        if let Value::Mapping(map) = yaml {
            assert!(map.contains_key(&Value::String("version".to_string())));
            assert!(map.contains_key(&Value::String("services".to_string())));
        } else {
            panic!("Expected YAML mapping");
        }
    }

    #[test]
    fn test_load_compose_file_missing_services() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let compose_path = temp_dir.path().join("docker-compose.yml");
        
        let compose_content = r#"
version: '3.8'
networks:
  default:
    driver: bridge
"#;
        
        fs::write(&compose_path, compose_content).expect("Failed to write compose file");
        
        let result = load_compose_file(compose_path.to_str().unwrap());
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Missing required 'services' section"));
    }

    #[test]
    fn test_load_compose_file_invalid_yaml() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let compose_path = temp_dir.path().join("docker-compose.yml");
        
        let compose_content = r#"
version: '3.8'
services:
  web:
    image: nginx:alpine
    ports:
      - "80:80"
      - invalid_port_mapping
"#;
        
        fs::write(&compose_path, compose_content).expect("Failed to write compose file");
        
        let result = load_compose_file(compose_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_compose_file_not_mapping() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let compose_path = temp_dir.path().join("docker-compose.yml");
        
        let compose_content = r#"
- item1
- item2
"#;
        
        fs::write(&compose_path, compose_content).expect("Failed to write compose file");
        
        let result = load_compose_file(compose_path.to_str().unwrap());
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("must be a YAML mapping/object"));
    }

    #[test]
    fn test_merge_yaml_values_mappings() {
        let base = serde_yaml::from_str(r#"
services:
  web:
    image: nginx:alpine
    ports:
      - "80:80"
"#).unwrap();
        
        let override_yaml = serde_yaml::from_str(r#"
services:
  web:
    environment:
      - NODE_ENV=production
  db:
    image: postgres:13
"#).unwrap();
        
        let result = merge_yaml_values(base, override_yaml);
        
        if let Value::Mapping(map) = result {
            if let Some(Value::Mapping(services)) = map.get(&Value::String("services".to_string())) {
                // Check that web service has both original and new properties
                if let Some(Value::Mapping(web)) = services.get(&Value::String("web".to_string())) {
                    assert!(web.contains_key(&Value::String("image".to_string())));
                    assert!(web.contains_key(&Value::String("ports".to_string())));
                    assert!(web.contains_key(&Value::String("environment".to_string())));
                }
                
                // Check that db service was added
                assert!(services.contains_key(&Value::String("db".to_string())));
            } else {
                panic!("Expected services mapping");
            }
        } else {
            panic!("Expected root mapping");
        }
    }

    #[test]
    fn test_merge_yaml_values_sequences() {
        let base = serde_yaml::from_str(r#"
services:
  web:
    ports:
      - "80:80"
      - "443:443"
"#).unwrap();
        
        let override_yaml = serde_yaml::from_str(r#"
services:
  web:
    ports:
      - "8080:80"
"#).unwrap();
        
        let result = merge_yaml_values(base, override_yaml);
        
        if let Value::Mapping(map) = result {
            if let Some(Value::Mapping(services)) = map.get(&Value::String("services".to_string())) {
                if let Some(Value::Mapping(web)) = services.get(&Value::String("web".to_string())) {
                    if let Some(Value::Sequence(ports)) = web.get(&Value::String("ports".to_string())) {
                        assert_eq!(ports.len(), 3); // Original 2 + 1 new
                        assert!(ports.contains(&Value::String("80:80".to_string())));
                        assert!(ports.contains(&Value::String("443:443".to_string())));
                        assert!(ports.contains(&Value::String("8080:80".to_string())));
                    } else {
                        panic!("Expected ports sequence");
                    }
                } else {
                    panic!("Expected web service");
                }
            } else {
                panic!("Expected services mapping");
            }
        } else {
            panic!("Expected root mapping");
        }
    }

    #[test]
    fn test_merge_yaml_values_primitives() {
        let base = serde_yaml::from_str(r#"
version: '3.8'
services:
  web:
    restart: "no"
"#).unwrap();
        
        let override_yaml = serde_yaml::from_str(r#"
version: '3.9'
services:
  web:
    restart: always
"#).unwrap();
        
        let result = merge_yaml_values(base, override_yaml);
        
        if let Value::Mapping(map) = result {
            // Version should be overridden
            assert_eq!(map.get(&Value::String("version".to_string())), Some(&Value::String("3.9".to_string())));
            
            if let Some(Value::Mapping(services)) = map.get(&Value::String("services".to_string())) {
                if let Some(Value::Mapping(web)) = services.get(&Value::String("web".to_string())) {
                    // Restart should be overridden
                    assert_eq!(web.get(&Value::String("restart".to_string())), Some(&Value::String("always".to_string())));
                } else {
                    panic!("Expected web service");
                }
            } else {
                panic!("Expected services mapping");
            }
        } else {
            panic!("Expected root mapping");
        }
    }

    #[test]
    fn test_resolve_merge_order_base_only() {
        let merger = ComposeMerger::new(
            "/path/to/base".to_string(),
            "/path/to/environments".to_string(),
            vec!["/path/to/extensions".to_string()],
        );
        
        let result = resolve_merge_order(&merger, None, &[]);
        assert!(result.is_ok());
        
        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].contains("base"));
        assert!(files[0].contains("docker-compose.yml"));
    }

    #[test]
    fn test_resolve_merge_order_with_environment() {
        let merger = ComposeMerger::new(
            "/path/to/base".to_string(),
            "/path/to/environments".to_string(),
            vec!["/path/to/extensions".to_string()],
        );
        
        let result = resolve_merge_order(&merger, Some("dev"), &[]);
        assert!(result.is_ok());
        
        let files = result.unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].contains("base"));
        assert!(files[1].contains("environments/dev"));
    }

    #[test]
    fn test_resolve_merge_order_with_extensions() {
        let merger = ComposeMerger::new(
            "/path/to/base".to_string(),
            "/path/to/environments".to_string(),
            vec!["/path/to/extensions".to_string()],
        );
        
        let result = resolve_merge_order(&merger, None, &["monitoring".to_string(), "auth".to_string()]);
        assert!(result.is_ok());
        
        let files = result.unwrap();
        assert_eq!(files.len(), 3); // base + 2 extensions
        assert!(files[0].contains("base"));
        assert!(files[1].contains("extensions/monitoring") || files[2].contains("extensions/monitoring"));
        assert!(files[1].contains("extensions/auth") || files[2].contains("extensions/auth"));
    }

    #[test]
    fn test_resolve_merge_order_complete() {
        let merger = ComposeMerger::new(
            "/path/to/base".to_string(),
            "/path/to/environments".to_string(),
            vec!["/path/to/extensions".to_string()],
        );
        
        let result = resolve_merge_order(&merger, Some("prod"), &["monitoring".to_string()]);
        assert!(result.is_ok());
        
        let files = result.unwrap();
        assert_eq!(files.len(), 3); // base + environment + extension
        assert!(files[0].contains("base"));
        assert!(files[1].contains("environments/prod"));
        assert!(files[2].contains("extensions/monitoring"));
    }

    #[test]
    fn test_merge_compose_files_integration() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        create_test_project(temp_dir.path()).expect("Failed to create test project");
        
        let merger = ComposeMerger::new(
            temp_dir.path().join("components/base").to_string_lossy().to_string(),
            temp_dir.path().join("components/environments").to_string_lossy().to_string(),
            vec![temp_dir.path().join("components/extensions").to_string_lossy().to_string()],
        );
        
        let result = merge_compose_files(&merger, Some("dev"), &["monitoring".to_string()]);
        assert!(result.is_ok());
        
        let merged = result.unwrap();
        if let Value::Mapping(map) = merged {
            assert!(map.contains_key(&Value::String("version".to_string())));
            
            if let Some(Value::Mapping(services)) = map.get(&Value::String("services".to_string())) {
                // Should have base service with dev environment
                assert!(services.contains_key(&Value::String("test-service".to_string())));
                
                // Should have monitoring extension
                assert!(services.contains_key(&Value::String("prometheus".to_string())));
                
                // Check that dev environment was applied
                if let Some(Value::Mapping(test_service)) = services.get(&Value::String("test-service".to_string())) {
                    if let Some(Value::Sequence(env)) = test_service.get(&Value::String("environment".to_string())) {
                        assert!(env.contains(&Value::String("ENV=development".to_string())));
                    }
                }
            } else {
                panic!("Expected services mapping");
            }
        } else {
            panic!("Expected root mapping");
        }
    }

    #[test]
    fn test_parse_extension_combination() {
        let combo = "monitoring+auth+logging";
        let result = parse_extension_combination(combo);
        
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "monitoring");
        assert_eq!(result[1], "auth");
        assert_eq!(result[2], "logging");
    }

    #[test]
    fn test_parse_extension_combination_single() {
        let combo = "monitoring";
        let result = parse_extension_combination(combo);
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "monitoring");
    }

    #[test]
    fn test_parse_extension_combination_with_spaces() {
        let combo = "monitoring + auth + logging";
        let result = parse_extension_combination(combo);
        
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "monitoring");
        assert_eq!(result[1], "auth");
        assert_eq!(result[2], "logging");
    }
}