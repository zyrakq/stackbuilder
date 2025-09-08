# Environment Without Specific Configuration Example

This example demonstrates how stackbuilder handles environments that don't have specific configuration folders.

## Structure

```sh
env-no-folder/
├── stackbuilder.toml
├── components/
│   └── base/
│       └── docker-compose.yml
└── build/
    └── docker-compose.yml
```

## Key Features

- Minimal configuration - no `[paths]` section needed (uses defaults)
- Environment "prod" is defined in configuration but has no specific directory
- No `components/environments/` directory exists at all
- stackbuilder uses only the base configuration for the environment
- No validation errors - environments directories are optional

## Usage

```bash
stackbuilder build
```

This will create `build/docker-compose.yml` using only the base configuration, since the "prod" environment has no specific overrides.

## Expected Output

- Single `docker-compose.yml` file in build directory
- Informational message: "No environments directory found - environments will use base configuration only"
- Warning about skipping missing environment file (this is expected and safe)
