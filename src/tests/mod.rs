//! Integration and unit tests for stackbuilder

pub mod config_tests;
pub mod build_tests;
pub mod merger_tests;
pub mod init_tests;
pub mod error_tests;

#[cfg(test)]
mod test_utils {
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    
    /// Create a temporary directory with test files
    pub fn create_test_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }
    
    /// Create a basic stackbuilder.toml file in the given directory
    pub fn create_test_config(dir: &Path) -> std::io::Result<()> {
        let config_content = r#"
[paths]
components_dir = "./components"
base_dir = "base"
environments_dir = "environments"
extensions_dirs = ["extensions"]
build_dir = "./build"

[build]
environments = ["dev", "prod"]
extensions = ["monitoring"]
"#;
        fs::write(dir.join("stackbuilder.toml"), config_content)
    }
    
    /// Create a basic docker-compose.yml file
    pub fn create_test_compose(path: &Path) -> std::io::Result<()> {
        let compose_content = r#"
version: '3.8'
services:
  test-service:
    image: nginx:alpine
    ports:
      - "8080:80"
"#;
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, compose_content)
    }
    
    /// Create a complete test project structure
    pub fn create_test_project(dir: &Path) -> std::io::Result<()> {
        create_test_config(dir)?;
        
        // Create base component
        create_test_compose(&dir.join("components/base/docker-compose.yml"))?;
        
        // Create environments
        let dev_compose = r#"
version: '3.8'
services:
  test-service:
    environment:
      - ENV=development
"#;
        fs::create_dir_all(dir.join("components/environments/dev"))?;
        fs::write(dir.join("components/environments/dev/docker-compose.yml"), dev_compose)?;
        
        let prod_compose = r#"
version: '3.8'
services:
  test-service:
    environment:
      - ENV=production
    deploy:
      replicas: 2
"#;
        fs::create_dir_all(dir.join("components/environments/prod"))?;
        fs::write(dir.join("components/environments/prod/docker-compose.yml"), prod_compose)?;
        
        // Create extension
        let monitoring_compose = r#"
version: '3.8'
services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
"#;
        fs::create_dir_all(dir.join("components/extensions/monitoring"))?;
        fs::write(dir.join("components/extensions/monitoring/docker-compose.yml"), monitoring_compose)?;
        
        Ok(())
    }
}

pub use test_utils::*;