# Invalid YAML Example

This directory demonstrates what happens when docker-compose.yml files have invalid YAML syntax.

The base docker-compose.yml file contains multiple YAML syntax errors:

- Unclosed bracket in array
- Missing colon after `networks`
- Invalid environment variable syntax

## Expected Error

When running `stackbuilder build` in this directory, you should get:

```log
Error: Failed to parse YAML file 'components/base/docker-compose.yml': Flow sequence in block collection must be sufficiently indented and end with a ]
```

## How to Fix

Fix the YAML syntax errors in the docker-compose.yml file:

```yaml
version: '3.8'
services:
  app:
    image: nginx:latest
    ports:
      - "8080:80"
    environment:
      - VALID_VAR=value
    volumes:
      - ./data:/app/data
    networks:
      - web
```

## Test Command

```bash
cd examples/error-cases/invalid-yaml
stackbuilder build
