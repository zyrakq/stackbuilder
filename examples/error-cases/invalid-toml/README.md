# Invalid TOML Syntax Example

This directory demonstrates what happens when `stackbuilder.toml` has invalid TOML syntax.

The configuration file contains a syntax error: missing closing bracket in the `extensions_dirs` array.

## Expected Error

When running `stackbuilder build` in this directory, you should get:

```
Error: Invalid TOML syntax in configuration file 'stackbuilder.toml': expected ","
```

## How to Fix

Fix the TOML syntax error by adding the missing closing bracket:

```toml
extensions_dirs = ["extensions"]  # Add missing closing bracket
```

## Test Command

```bash
cd examples/error-cases/invalid-toml
stackbuilder build
