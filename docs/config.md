# Stackbuilder Configuration Specification

This document specifies the structure of the `stackbuilder.toml` configuration file used by the stackbuilder CLI tool to assemble docker-compose files from components in base, environments, and extensions directories.

## Configuration Structure

The `stackbuilder.toml` file uses TOML syntax and consists of two main sections: `[paths]` and `[build]`.

### [paths] Section

Defines the file system paths used by stackbuilder.

- `components_dir` (string, default: `"./components"`): Base directory containing all component folders
- `base_dir` (string, default: `"base"`): Relative path to the base components directory (within `components_dir`)
- `environments_dir` (string, default: `"environments"`): Relative path to the environments components directory (within `components_dir`)
- `extensions_dirs` (array of strings, default: `["extensions"]`): Relative paths to extension components directories (within `components_dir`)
- `build_dir` (string, default: `"./build"`): Output directory for assembled docker-compose files

### [build] Section

Defines the build rules and configurations.

- `environments` (array of strings, optional): List of environment names to build. If omitted, all found environments are built.
- `extensions` (array of strings, optional): Global list of extensions to apply to all environments. Alternative to per-environment configuration.
- `combos` (array of strings, optional): List of extension combinations (e.g., `["oidc+guard", "monitoring+security"]`)

#### Per-Environment Configuration

For more granular control, you can configure extensions per environment using the `[[build.environment]]` table array:

```toml
[[build.environment]]
name = "dev"
extensions = ["oidc", "monitoring"]
```

Where:

- `name` (string, required): Environment name matching a directory in `environments_dir`
- `extensions` (array of strings, optional): Extensions to apply to this environment

## Validation Rules

1. **Existence Check**: All specified paths must exist or be creatable
2. **Minimum Requirements**: At least one environment OR one extension must be specified (either globally or per-environment)
3. **Component Validation**: Base directory must contain valid components, environment and extension directories must exist if specified
4. **Combos Validation**: Extension combinations must reference valid extension names separated by '+'

## Default Values

If `stackbuilder.toml` is missing or incomplete, these defaults apply:

- `components_dir = "./components"`
- `base_dir = "base"`
- `environments_dir = "environments"`
- `extensions_dirs = ["extensions"]`
- `build_dir = "./build"`
- If no environments specified: build all found
- If no extensions specified: use all found
- If no combos: no combinations applied

## Rust Struct Definitions

For TOML deserialization using serde, the following Rust structure can be used:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub paths: Paths,
    pub build: Build,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
pub struct Build {
    pub environments: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    pub combos: Option<Vec<String>>,
    pub environment: Option<Vec<EnvironmentConfig>>,
}

#[derive(Deserialize)]
pub struct EnvironmentConfig {
    pub name: String,
    pub extensions: Option<Vec<String>>,
}

// Default functions
fn default_components_dir() -> String { "./components".to_string() }
fn default_base_dir() -> String { "base".to_string() }
fn default_environments_dir() -> String { "environments".to_string() }
fn default_extensions_dirs() -> Vec<String> { vec!["extensions".to_string()] }
fn default_build_dir() -> String { "./build".to_string() }
```

## Configuration Examples

### Minimal Configuration (Base Only)

```toml
[paths]
components_dir = "./components"
base_dir = "base"
build_dir = "./build"

[build]
# No environments or extensions - only base components
```

### Multi-Environment Configuration

```toml
[paths]
components_dir = "./components"
environments_dir = "environments"
build_dir = "./build"

[build]
environments = ["dev", "staging", "prod"]

# Per-environment extensions
[[build.environment]]
name = "dev"
extensions = []

[[build.environment]]
name = "staging"
extensions = ["monitoring"]

[[build.environment]]
name = "prod"
extensions = ["monitoring", "security"]
```

### Extensions Configuration

```toml
[paths]
extensions_dirs = ["extensions", "custom_extensions"]
build_dir = "./build"

[build]
environments = ["test"]
extensions = ["oidc", "guard", "monitoring"]
combos = ["oidc+guard"]
```

### Full Configuration with All Features

```toml
[paths]
components_dir = "./components"
base_dir = "base"
environments_dir = "environments"
extensions_dirs = ["extensions", "community_extensions"]
build_dir = "./build"

[build]
environments = ["dev", "test", "staging", "prod"]
extensions = ["basic_auth"]  # Global extensions
combos = ["oidc+guard", "monitoring+backup"]

# Per-environment overrides
[[build.environment]]
name = "dev"
extensions = ["oidc", "guard", "debug_tools"]

[[build.environment]]
name = "prod"
extensions = ["oidc", "guard", "security_hardening"]

[[build.environment]]
name = "test"
extensions = ["basic_auth", "test_tools"]
```

This configuration provides flexible component assembly while maintaining intuitive defaults for quick setup.
