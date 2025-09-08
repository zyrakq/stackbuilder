# Legacy Combos Example

Demonstrates how combos work in legacy configuration format (without `[build.targets]`).

## Configuration

- 1 environment: `dev`
- 1 direct extension: `logging`
- 1 combo: `security = ["oidc", "guard"]`

## Build Output

Since we have multiple variants (1 extension + 1 combo = 2 total), subfolders are created:

- `build/base/` - Base configuration for dev environment
- `build/logging/` - Dev environment with logging extension
- `build/security/` - Dev environment with security combo (oidc + guard extensions)

## Key Features

- Legacy combo support in `resolve_legacy_combinations()`
- Proper subfolder creation logic based on total variants count
- Combo extensions are resolved and merged correctly
