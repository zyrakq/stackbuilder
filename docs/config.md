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
- `copy_env_example` (boolean, default: `true`): Enable merging of .env.example files from components into output directories

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
    #[serde(default = "default_copy_env_example")]
    pub copy_env_example: bool,
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
fn default_copy_env_example() -> bool { true }
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

## Environment Variables Merging (.env.example)

Stackbuilder automatically merges `.env.example` files from component directories when the `copy_env_example` option is enabled (default: `true`).

### Merge Order and Priority

Environment variables are merged in the following priority order (later sources override earlier ones):

1. `base/.env.example` - Base environment variables
2. `environments/{env}/.env.example` - Environment-specific variables  
3. `extensions/{ext1}/.env.example` - Extension variables (in order specified)
4. `extensions/{ext2}/.env.example` - Additional extension variables

### Merge Behavior

- **Variable Overriding**: Later files override variables from earlier files with the same name
- **Comment Preservation**: Comments from all source files are preserved and organized by source
- **Missing Files**: Missing `.env.example` files are silently skipped (not an error)
- **File Structure**: Generated files include headers indicating source files and organization

### Generated .env.example Format

```bash
# Generated by stackbuilder from multiple .env.example files
# Source files: base, dev, oidc, monitoring

# Variables from base/.env.example
APP_NAME=myapp
APP_VERSION=1.0.0

# Variables from environments/dev/.env.example  
DEBUG=true
LOG_LEVEL=debug

# Variables from extensions/oidc/.env.example
OIDC_CLIENT_ID=your_client_id
OIDC_CLIENT_SECRET=your_client_secret

# Variables from extensions/monitoring/.env.example
METRICS_ENABLED=true
METRICS_PORT=9090
```

### Configuration Examples

#### Enable .env.example merging (default)

```toml
[build]
copy_env_example = true
environments = ["dev", "prod"]
extensions = ["oidc", "monitoring"]
```

#### Disable .env.example merging

```toml
[build]
copy_env_example = false
environments = ["dev", "prod"]
extensions = ["oidc", "monitoring"]
```

### File Locations

Place `.env.example` files alongside `docker-compose.yml` files in:

- `components/base/.env.example` - Base variables
- `components/environments/{env}/.env.example` - Environment-specific variables
- `components/extensions/{ext}/.env.example` - Extension-specific variables

### Usage Notes

- Variables with spaces or special characters are automatically quoted
- Empty variables are preserved with empty quotes: `VAR=""`
- Comments starting with `#` are preserved from all source files
- Output `.env.example` files are placed in the same directories as generated `docker-compose.yml` files
