# Missing Base Directory Example

This directory demonstrates what happens when the base directory is missing from the components directory.

The configuration file exists and is valid, but the required `components/base` directory does not exist.

## Expected Error

When running `stackbuilder build` in this directory, you should get:

```log
Error: Base directory 'components/base' does not exist in components_dir. Create base/docker-compose.yml file
```

## How to Fix

Create the missing base directory and docker-compose.yml file:

```bash
mkdir -p components/base
echo 'version: "3.8"
services:
  app:
    image: nginx:latest' > components/base/docker-compose.yml
```

Or run `stackbuilder init --skip-folders` to create the base structure.

## Test Command

```bash
cd examples/error-cases/missing-base
stackbuilder build
