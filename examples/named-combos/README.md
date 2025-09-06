# Named Combos Example

This example demonstrates the new **named combos** feature in StackBuilder, which allows you to define reusable combinations of extensions and reference them by name in environment configurations.

## Configuration Overview

The `stackbuilder.toml` defines three named combos:

- **security**: `["oidc", "guard"]` - Authentication and reverse proxy
- **monitoring**: `["prometheus", "grafana", "alertmanager"]` - Complete monitoring stack  
- **development**: `["debugging", "profiling"]` - Development and debugging tools

## Environment Configurations

### Development Environment

- **Extensions**: `["logging"]` (ELK stack)
- **Combos**: `["security", "development"]`
- **Result**: oidc + guard + debugging + profiling + logging

### Staging Environment  

- **Extensions**: `["backup"]` (Restic backup)
- **Combos**: `["monitoring"]`
- **Result**: prometheus + grafana + alertmanager + backup

### Production Environment

- **Combos**: `["security", "monitoring"]`
- **Result**: oidc + guard + prometheus + grafana + alertmanager

## Expected Build Output

When you run `stackbuilder build`, the following structure will be created:

```sh
build/
├── dev/
│   ├── base/docker-compose.yml                    # Base app
│   ├── logging+security+development/              # Combined extensions and combos
│   │   └── docker-compose.yml                     # All services merged
├── staging/
│   ├── base/docker-compose.yml                    # Base app  
│   ├── backup+monitoring/                         # Mixed extension + combo
│   │   └── docker-compose.yml                     # All services merged
└── prod/
    ├── base/docker-compose.yml                    # Base app
    ├── security+monitoring/                       # Named combos only
    │   └── docker-compose.yml                     # All services merged
```

## Key Features Demonstrated

1. **Named Combo Definitions**: Define reusable extension combinations
2. **Mixed Usage**: Combine individual extensions with named combos
3. **Smart Folder Naming**: Output directories reflect both extensions and combo names
4. **Extension Deduplication**: Extensions are automatically deduplicated across combos
5. **Validation**: Non-existent combos and extensions are validated at build time

## Running the Example

```bash
cd examples/named-combos
stackbuilder build
```

The build process will:

1. Validate all combo definitions exist
2. Validate all referenced extensions exist  
3. Resolve combos into their constituent extensions
4. Create merged docker-compose files for each combination
5. Copy additional configuration files with proper priority

## Benefits of Named Combos

- **Reusability**: Define once, use multiple times
- **Maintainability**: Change combo definition updates all usages
- **Readability**: Semantic names instead of extension lists
- **Consistency**: Ensure same extension combinations across environments
- **Flexibility**: Mix individual extensions with predefined combos
