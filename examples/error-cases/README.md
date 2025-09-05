# Error Cases Examples

This directory contains examples that demonstrate various error scenarios in stackbuilder and how the improved error handling system works.

## Available Error Cases

### 1. missing-config/

Demonstrates what happens when `stackbuilder.toml` configuration file is missing.

- **Error Type**: ConfigError::ConfigFileNotFound
- **Exit Code**: 1
- **Suggestion**: Run `stackbuilder init`

### 2. invalid-toml/

Shows how the application handles invalid TOML syntax in configuration files.

- **Error Type**: ConfigError::InvalidTomlSyntax  
- **Exit Code**: 1
- **Suggestion**: Fix TOML syntax errors

### 3. missing-base/

Demonstrates missing base directory validation.

- **Error Type**: ValidationError::BaseDirectoryNotFound
- **Exit Code**: 2
- **Suggestion**: Create base/docker-compose.yml file

### 4. invalid-yaml/

Shows YAML parsing errors in docker-compose files.

- **Error Type**: YamlError::ParseError
- **Exit Code**: 5
- **Suggestion**: Fix YAML syntax

## Testing All Error Cases

Run the following commands to test each error case:

```bash
# Test missing config
cd examples/error-cases/missing-config && stackbuilder build

# Test invalid TOML
cd examples/error-cases/invalid-toml && stackbuilder build

# Test missing base directory
cd examples/error-cases/missing-base && stackbuilder build

# Test invalid YAML
cd examples/error-cases/invalid-yaml && stackbuilder build
```

## Error Handling Features

- **Specific Error Types**: Each error has a specific type with detailed context
- **User-Friendly Messages**: Clear explanations of what went wrong
- **Actionable Suggestions**: Helpful hints on how to fix the issues
- **Appropriate Exit Codes**: Different exit codes for different error categories
- **No Stack Traces**: Clean error output without Rust panic information
