# Multi-Environment Combos Example

Demonstrates combo functionality with multiple environments in legacy format.

## Configuration

- 2 environments: `dev`, `prod`
- 1 direct extension: `logging`
- 1 combo: `security = ["oidc", "guard"]`

## Build Output

Since we have multiple environments, environment-prefixed subfolders are created:

- `build/dev/base/` - Base configuration for dev environment
- `build/dev/logging/` - Dev environment with logging extension
- `build/dev/security/` - Dev environment with security combo
- `build/prod/base/` - Base configuration for prod environment  
- `build/prod/logging/` - Prod environment with logging extension
- `build/prod/security/` - Prod environment with security combo

## Key Features

- Multiple environment support with combos
- Environment-prefixed subfolder structure
- Both direct extensions and combos are processed for each environment
