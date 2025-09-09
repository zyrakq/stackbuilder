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

- `combos` (table, optional): Named combinations of extensions (see Named Combos section below)
- `environments` (table, optional): Environments configuration section (see Build Environments section below)
- `copy_env_example` (boolean, default: `true`): Enable merging of .env.example files from components into output directories
- `copy_additional_files` (boolean, default: `true`): Enable copying of additional files (configs, scripts, certificates) from components with priority-based overriding
- `exclude_patterns` (array of strings, default: `["docker-compose.yml", ".env.example", "*.tmp", ".git*", "node_modules", "*.log"]`): Glob patterns for files to exclude from additional file copying
- `preserve_env_files` (boolean, default: `true`): Enable intelligent preservation of existing .env files during build directory cleanup
- `env_file_patterns` (array of strings, default: `[".env", ".env.local", ".env.production"]`): Patterns for .env files to preserve during smart cleanup
- `backup_dir` (string, default: `"./.stackbuilder/backup"`): Directory path for storing .env file backups during build directory cleanup
- `skip_base_generation` (boolean, default: `false`): Skip generation of base configuration variants, useful for extension-only or combo-only scenarios

#### Named Combos

Named combos allow you to define reusable combinations of extensions that can be referenced by name:

```toml
[build]
# Define named combos as inline tables
combos = {
    security = ["oidc", "guard"],
    monitoring = ["prometheus", "grafana", "alertmanager"],
    development = ["debugging", "profiling"]
}
```

Benefits of named combos:

- **Reusability**: Define once, use multiple times across environments
- **Maintainability**: Change combo definition updates all usages
- **Readability**: Semantic names instead of extension lists
- **Consistency**: Ensure same extension combinations across environments

#### Build Environments

The `[build.environments]` configuration provides an intuitive way to manage environments and their specific configurations:

```toml
[build]
# Define named combos
combos = {
    security = ["oidc", "guard"],
    monitoring = ["prometheus", "grafana"]
}

# Environments API
[build.environments]
available = ["dev", "staging", "prod"]

# Per-environment configurations
[build.environments.dev]
extensions = ["logging"]                 # Apply logging extension to dev
combos = ["security"]                    # Apply security combo to dev
skip_base_generation = true             # Skip base generation for dev

[build.environments.staging]
extensions = ["backup"]                  # Apply backup extension to staging
combos = ["monitoring"]                  # Apply monitoring combo to staging

[build.environments.prod]
combos = ["security", "monitoring"]      # Apply both combos to prod
```

**API structure:**

- `[build.environments]` - Main environments configuration section
  - `available` (array of strings): List of available environment names
- `[build.environments.{env}]` - Per-environment configuration sections:
  - `extensions` (array of strings, optional): Extensions to apply to this environment
  - `combos` (array of strings, optional): Named combos to apply to this environment
  - `skip_base_generation` (boolean, optional): Override global skip_base_generation for this environment

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
[build.environments]
available = ["dev", "staging", "prod"]

[build.environments.dev]
extensions = ["monitoring"]

[build.environments.staging]
extensions = ["monitoring"]

[build.environments.prod]
extensions = ["monitoring"]
```

### Advanced Configuration with Named Combos

```toml
[paths]
components_dir = "./components"
base_dir = "base"
environments_dir = "environments"
extensions_dirs = ["extensions"]
build_dir = "./build"

[build]
# Define named combos
combos = {
    security = ["oidc", "guard"],
    monitoring = ["prometheus", "grafana", "alertmanager"],
    development = ["debugging", "profiling"]
}

# Environments API
[build.environments]
available = ["dev", "staging", "prod"]

# Per-environment configurations
[build.environments.dev]
extensions = ["logging"]                   # dev: only logging extension
combos = ["security", "development"]       # dev: only security and development combos
skip_base_generation = true               # dev: skip base generation

[build.environments.staging]
extensions = ["backup"]                    # staging: only backup extension
combos = ["monitoring"]                    # staging: only monitoring combo

[build.environments.prod]
combos = ["security", "monitoring"]        # prod: only specified combos
```

This configuration creates the following build structure:

```sh
build/
├── dev/
│   ├── logging/docker-compose.yml          # Only logging extension (base skipped)
│   ├── security/docker-compose.yml         # Only security combo (oidc + guard)
│   └── development/docker-compose.yml      # Only development combo (debugging + profiling)
├── staging/
│   ├── base/docker-compose.yml             # Base included
│   ├── backup/docker-compose.yml           # Only backup extension
│   └── monitoring/docker-compose.yml       # Only monitoring combo (prometheus + grafana + alertmanager)
└── prod/
    ├── base/docker-compose.yml             # Base included
    ├── security/docker-compose.yml         # Only security combo (oidc + guard)
    └── monitoring/docker-compose.yml       # Only monitoring combo (prometheus + grafana + alertmanager)
```

#### Skip Base Generation Examples

**Extension-only with skip_base_generation:**

```toml
[build]
[build.environments]
available = ["prod"]

[build.environments.prod]
extensions = ["monitoring"]
skip_base_generation = true
```

Output: `build/docker-compose.yml` (single file, no subfolders)

**Combo-only with skip_base_generation:**

```toml
[build]
combos = { fullstack = ["frontend", "backend", "database"] }

[build.environments]
available = ["prod"]

[build.environments.prod]
combos = ["fullstack"]
skip_base_generation = true
```

Output: `build/docker-compose.yml` (combo merged directly)

**Multiple variants with skip_base_generation:**

```toml
[build]
combos = { security = ["oidc", "guard"] }

[build.environments]
available = ["prod"]

[build.environments.prod]
extensions = ["logging"]
combos = ["security"]
skip_base_generation = true
```

Output: `build/logging/` and `build/security/` (no base/ subfolder)

## Validation Rules

1. **Existence Check**: All specified paths must exist or be creatable
2. **Minimum Requirements**: At least one environment OR one extension must be specified (either globally or per-environment)
3. **Component Validation**: Base directory must contain valid components, environment and extension directories must exist if specified
4. **Combos Validation**: Named combinations must reference valid extension names defined in available extensions

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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    pub paths: Paths,
    pub build: Build,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Build {
    pub extensions: Option<Vec<String>>,
    #[serde(default)]
    pub combos: HashMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environments_config: Option<BuildEnvironments>,
    #[serde(default)]
    pub yaml_merger: YamlMergerType,
    #[serde(default = "default_copy_env_example")]
    pub copy_env_example: bool,
    #[serde(default = "default_copy_additional_files")]
    pub copy_additional_files: bool,
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    #[serde(default = "default_preserve_env_files")]
    pub preserve_env_files: bool,
    #[serde(default = "default_env_file_patterns")]
    pub env_file_patterns: Vec<String>,
    #[serde(default = "default_backup_dir")]
    pub backup_dir: String,
    #[serde(default = "default_skip_base_generation")]
    pub skip_base_generation: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BuildEnvironments {
    pub available: Option<Vec<String>>,
    #[serde(flatten)]
    pub environment_configs: HashMap<String, EnvironmentConfig>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EnvironmentConfig {
    pub extensions: Option<Vec<String>>,
    pub combos: Option<Vec<String>>,
    pub skip_base_generation: Option<bool>,
}
```

## Environment Variables Merging (.env.example)

Stackbuilder automatically merges `.env.example` files from component directories when the `copy_env_example` option is enabled (default: `true`).

### Merge Order and Priority

Environment variables are merged in the following priority order (later sources override earlier ones):

1. `base/.env.example` - Base environment variables
2. `environments/{env}/.env.example` - Environment-specific variables  
3. `extensions/{ext1}/.env.example` - Extension variables (in order specified)
4. `extensions/{ext2}/.env.example` - Additional extension variables

### Configuration Examples for File Copying

#### Enable .env.example merging and additional file copying (default)

```toml
[build]
copy_env_example = true
copy_additional_files = true

[build.environments]
available = ["dev", "prod"]

[build.environments.dev]
extensions = ["oidc"]

[build.environments.prod]
extensions = ["monitoring"]
```

#### Disable file copying features

```toml
[build]
copy_env_example = false
copy_additional_files = false
```

## Additional Files Copying

Stackbuilder can copy additional files (configuration files, scripts, certificates, etc.) from component directories when the `copy_additional_files` option is enabled (default: `true`).

### Copy Priority and Overriding Logic

Additional files are copied with priority-based overriding in the following order (higher priority overrides lower):

1. **Base Priority (1)**: `base/*` - Files from base components (lowest priority)
2. **Environment Priority (2)**: `environments/{env}/*` - Environment-specific files (medium priority)
3. **Extension Priority (3)**: `extensions/{ext}/*` - Extension-specific files (highest priority)

### File Location Guidelines

Place additional files alongside `docker-compose.yml` files in component directories:

- `components/base/` - Base configuration files, common scripts
- `components/environments/{env}/` - Environment-specific configs (nginx.conf, app.conf)
- `components/extensions/{ext}/` - Extension-specific configs (auth.conf, ssl certificates)

## Intelligent .env Files Preservation

Stackbuilder includes an intelligent build directory cleanup system that preserves existing `.env` files during rebuilds when the `preserve_env_files` option is enabled (default: `true`).

### How It Works

1. **Backup Phase**: Before cleaning the build directory, all `.env` files matching `env_file_patterns` are backed up to `backup_dir`
2. **Restoration Phase**: After creating the new build structure, files are restored only to their exact original locations
3. **Centralized Backup**: Files that cannot be restored (due to changed structure) remain in the backup directory for manual recovery

### Backup Directory Structure

The backup directory uses timestamped folders to preserve multiple backup versions:

```sh
.stackbuilder/backup/
└── backup_1694268450/
    ├── metadata.json
    ├── devcontainer_base_.env
    ├── dev_auth_.env.local
    └── prod_monitoring_.env.production
```

### Configuration Options

#### Enable .env preservation with custom backup location

```toml
[build]
preserve_env_files = true
env_file_patterns = [".env", ".env.local", ".env.production"]
backup_dir = "./my-custom-backup"
```

#### Default .env preservation

```toml
[build]
preserve_env_files = true
env_file_patterns = [".env", ".env.local", ".env.production"]
backup_dir = "./.stackbuilder/backup"  # default
```

#### Disable .env preservation

```toml
[build]
preserve_env_files = false
```

This performs standard cleanup (complete removal) without .env file scanning or restoration.

### Important Notes

- Files are **only restored to their exact original locations** - no fallback files are created in build directories
- If the build structure changes and original locations no longer exist, files remain safely in the backup directory
- Backup directories are **not automatically deleted** - they serve as a safety net for manual recovery
- In case of content conflicts, existing files take priority and preserved files remain in backup
