use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub paths: Paths,
    pub build: Build,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            paths: Paths::default(),
            build: Build::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Paths {
    #[serde(default = "default_components_dir")]
    pub components_dir: String,
    #[serde(default = "default_base_dir")]
    pub base_dir: String,
    #[serde(default = "default_environments_dir")]
    pub environments_dir: String,
    #[serde(default = "default_extensions_dirs")]
    pub extensions_dirs: Vec<String>,
    #[serde(default = "default_build_dir")]
    pub build_dir: String,
}

impl Default for Paths {
    fn default() -> Self {
        Paths {
            components_dir: default_components_dir(),
            base_dir: default_base_dir(),
            environments_dir: default_environments_dir(),
            extensions_dirs: default_extensions_dirs(),
            build_dir: default_build_dir(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Build {
    pub environments: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    pub combos: Option<Vec<String>>,
    pub environment: Option<Vec<EnvironmentConfig>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EnvironmentConfig {
    pub name: String,
    pub extensions: Option<Vec<String>>,
}

// Default functions
fn default_components_dir() -> String {
    "./components".to_string()
}

fn default_base_dir() -> String {
    "base".to_string()
}

fn default_environments_dir() -> String {
    "environments".to_string()
}

fn default_extensions_dirs() -> Vec<String> {
    vec!["extensions".to_string()]
}

fn default_build_dir() -> String {
    "./build".to_string()
}