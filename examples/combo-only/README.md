# Combo-Only Example

Demonstrates combo-only configuration with `skip_base_generation = true`.

## Configuration

- 1 environment: `prod`
- 0 direct extensions
- 1 combo: `fullstack = ["frontend", "backend", "database"]`
- `skip_base_generation = true`

## Build Output

Since we have only 1 variant (combo) and `skip_base_generation = true`, no subfolders are created:

- `build/docker-compose.yml` - Single file with all combo extensions merged

## Key Features

- Combo-only configuration support
- Skip base generation functionality
- Single variant logic puts file directly in build root
- No unnecessary subfolder creation when not needed
