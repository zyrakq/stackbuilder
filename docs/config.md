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
- `combos` (table, optional): Named combinations of extensions (see Named Combos section below)
- `targets` (table, optional): New-style target configuration supporting named combos (see Build Targets section below)
- `copy_env_example` (boolean, default: `true`): Enable merging of .env.example files from components into output directories
- `copy_additional_files` (boolean, default: `true`): Enable copying of additional files (configs, scripts, certificates) from components with priority-based overriding
- `exclude_patterns` (array of strings, default: `["docker-compose.yml", ".env.example", "*.tmp", ".git*", "node_modules", "*.log"]`): Glob patterns for files to exclude from additional file copying

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

#### Build Targets

The new `targets` configuration provides more flexible environment and combo management:

```toml
[build.targets]
environments = ["dev", "staging", "prod"]

# Per-environment configuration
[build.targets.dev]
extensions = ["logging"]          # Individual extensions
combos = ["security", "development"]  # Named combos

[build.targets.staging]
extensions = ["backup"]
combos = ["monitoring"]

[build.targets.prod]
combos = ["security", "monitoring"]  # Only combos, no individual extensions
```

Where:

- `environments` (array of strings, optional): List of environments to build
- `[build.targets.{env}]` sections define per-environment configuration:
  - `extensions` (array of strings, optional): Individual extensions for this environment
  - `combos` (array of strings, optional): Named combos to apply to this environment

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
    pub environments: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    #[serde(default)]
    pub combos: HashMap<String, Vec<String>>,
    pub targets: Option<BuildTargets>,
    #[serde(default = "default_copy_env_example")]
    pub copy_env_example: bool,
    #[serde(default = "default_copy_additional_files")]
    pub copy_additional_files: bool,
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BuildTargets {
    pub environments: Option<Vec<String>>,
    #[serde(flatten)]
    pub environment_configs: HashMap<String, EnvironmentTarget>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EnvironmentTarget {
    pub extensions: Option<Vec<String>>,
    pub combos: Option<Vec<String>>,
}

// Legacy support for old configuration format
#[derive(Deserialize, Serialize, Debug, Clone)]
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
fn default_copy_additional_files() -> bool { true }
fn default_exclude_patterns() -> Vec<String> {
    vec![
        "docker-compose.yml".to_string(),
        ".env.example".to_string(),
        "*.tmp".to_string(),
        ".git*".to_string(),
        "node_modules".to_string(),
        "*.log".to_string(),
    ]
}
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

# Define named combos
combos = { security = ["oidc", "guard"] }

# Use targets to apply combos
[build.targets]
environments = ["test"]

[build.targets.test]
combos = ["security"]
```

### Named Combos Configuration Example

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

# Use new targets configuration
[build.targets]
environments = ["dev", "staging", "prod"]

# Per-environment configuration using combos and extensions
[build.targets.dev]
extensions = ["logging"]
combos = ["security", "development"]

[build.targets.staging]
extensions = ["backup"]
combos = ["monitoring"]

[build.targets.prod]
combos = ["security", "monitoring"]
```

This configuration creates the following build structure:

```sh
build/
├── dev/
│   ├── base/docker-compose.yml
│   ├── logging/docker-compose.yml          # Individual extension
│   ├── security/docker-compose.yml         # Named combo (oidc + guard)
│   └── development/docker-compose.yml      # Named combo (debugging + profiling)
├── staging/
│   ├── base/docker-compose.yml
│   ├── backup/docker-compose.yml           # Individual extension
│   └── monitoring/docker-compose.yml       # Named combo (prometheus + grafana + alertmanager)
└── prod/
    ├── base/docker-compose.yml
    ├── security/docker-compose.yml         # Named combo (oidc + guard)
    └── monitoring/docker-compose.yml       # Named combo (prometheus + grafana + alertmanager)
```

### Legacy Configuration (Backwards Compatible)

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

### .env.example Configuration Examples

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

## Additional Files Copying

Stackbuilder can copy additional files (configuration files, scripts, certificates, etc.) from component directories when the `copy_additional_files` option is enabled (default: `true`).

### Copy Priority and Overriding Logic

Additional files are copied with priority-based overriding in the following order (higher priority overrides lower):

1. **Base Priority (1)**: `base/*` - Files from base components (lowest priority)
2. **Environment Priority (2)**: `environments/{env}/*` - Environment-specific files (medium priority)  
3. **Extension Priority (3)**: `extensions/{ext}/*` - Extension-specific files (highest priority)

### File Processing Rules

- **Priority Override**: Higher priority files replace lower priority files with the same relative path
- **Directory Structure**: Original directory structure is preserved in the output
- **Permissions**: File permissions are preserved during copying (Unix systems)
- **Exclusion Patterns**: Files matching `exclude_patterns` are automatically skipped

### Default Exclusion Patterns

```toml
exclude_patterns = [
    "docker-compose.yml",  # Already handled by docker-compose merging
    ".env.example",        # Already handled by env merging
    "*.tmp",              # Temporary files
    ".git*",              # Git files
    "node_modules",       # Node.js dependencies
    "*.log"               # Log files
]
```

### Additional Files Configuration Examples

#### Enable additional file copying (default)

```toml
[build]
copy_additional_files = true
environments = ["dev", "prod"]
extensions = ["oidc", "monitoring"]
```

#### Disable additional file copying

```toml
[build]
copy_additional_files = false
environments = ["dev", "prod"]
extensions = ["oidc", "monitoring"]
```

#### Custom exclusion patterns

```toml
[build]
copy_additional_files = true
exclude_patterns = [
    "docker-compose.yml",
    ".env.example", 
    "*.tmp",
    ".git*",
    "node_modules",
    "*.log",
    "*.backup",        # Custom: exclude backup files
    "test_*",          # Custom: exclude test files
    "*.development"    # Custom: exclude development files
]
```

### File Priority Examples

Consider this component structure:

```log
components/
├── base/
│   ├── config.json          # Priority 1 (Base)
│   └── scripts/setup.sh     # Priority 1 (Base)
├── environments/dev/
│   ├── config.json          # Priority 2 (Environment) - overrides base
│   └── nginx.conf           # Priority 2 (Environment) - new file
└── extensions/auth/
    ├── auth.conf            # Priority 3 (Extension) - new file
    └── config.json          # Priority 3 (Extension) - overrides all
```

**Result in build output:**

- `config.json` → Contains content from `extensions/auth/config.json` (highest priority)
- `scripts/setup.sh` → Contains content from `base/scripts/setup.sh` (only source)
- `nginx.conf` → Contains content from `environments/dev/nginx.conf` (only source)
- `auth.conf` → Contains content from `extensions/auth/auth.conf` (only source)

### Processing Log Output

During the build process, stackbuilder logs file copying operations:

```log
Copying additional files...
  File config.json found from base (priority: Base)
  File config.json: environment:dev overrides base (priority: Environment > Base)
  File config.json: extension:auth overrides environment:dev (priority: Extension > Environment)
  File scripts/setup.sh found from base (priority: Base)
  File nginx.conf found from environment:dev (priority: Environment)
  File auth.conf found from extension:auth (priority: Extension)
  Copied: config.json -> ./build/dev/auth/config.json (from extension:auth)
  Copied: scripts/setup.sh -> ./build/dev/auth/scripts/setup.sh (from base)
  Copied: nginx.conf -> ./build/dev/auth/nginx.conf (from environment:dev)
  Copied: auth.conf -> ./build/dev/auth/auth.conf (from extension:auth)
Additional file copying completed
```

### File Location Guidelines

Place additional files alongside `docker-compose.yml` files in component directories:

- `components/base/` - Base configuration files, common scripts
- `components/environments/{env}/` - Environment-specific configs (nginx.conf, app.conf)
- `components/extensions/{ext}/` - Extension-specific configs (auth.conf, ssl certificates)

### Usage Notes

- Files are copied after docker-compose and .env merging is complete
- Missing component directories are silently skipped (not an error)
- Symbolic links are followed and the target files are copied
- Binary files are supported and copied without modification
- Large files may impact build performance - consider using exclusion patterns
environments = ["dev", "prod"]
extensions = ["oidc", "monitoring"]

```log

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
