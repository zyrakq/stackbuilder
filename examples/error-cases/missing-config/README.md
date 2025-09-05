# Missing Configuration Example

This directory demonstrates what happens when `stackbuilder.toml` configuration file is missing.

## Expected Error

When running `stackbuilder build` in this directory, you should get:

```log
Error: Configuration file 'stackbuilder.toml' not found. Run 'stackbuilder init' to create a new project
```

## How to Fix

Run `stackbuilder init` to create a new project with default configuration.

## Test Command

```bash
cd examples/error-cases/missing-config
stackbuilder build
