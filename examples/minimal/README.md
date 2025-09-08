# Minimal Configuration Example

This example demonstrates stackbuilder with absolutely minimal configuration - just a comment in the config file and default paths everywhere.

## Structure

```sh
minimal/
├── stackbuilder.toml      # Just a comment, all sections optional
├── components/
│   └── base/
│       └── docker-compose.yml
└── build/
    └── docker-compose.yml
```

## Key Features

- No configuration sections needed at all
- All paths use defaults: `./components`, `base`, `./build`
- No environments, extensions, or targets specified
- Creates single `docker-compose.yml` from base configuration only
- Informational message: "No specific targets configured - will build base configuration only"

## Usage

```bash
stackbuilder build
```

This creates `build/docker-compose.yml` using only the base configuration. Perfect for simple projects that just need basic Docker Compose functionality without multiple environments or extensions.

## Default Behavior

When no configuration is provided, stackbuilder automatically:

- Uses `./components/base/` as source
- Outputs to `./build/docker-compose.yml`
- Creates base-only build without subfolders
- Validates only that base configuration exists
