# YAML Merger Configuration

StackBuilder supports two methods for merging YAML files: external `yq` command (recommended) and built-in Rust libraries.

## Configuration

In your `stackbuilder.toml` file, add the `yaml_merger` parameter to the `[build]` section:

```toml
[build]
yaml_merger = "yq"  # or "rust"
```

### Available values

- **`"yq"`** (default) - uses external yq v4+ command
- **`"rust"`** - uses built-in yaml-rust2 and serde_yaml libraries

## YQ Merger (Recommended)

### Advantages

- Cleaner and more readable YAML output
- Minimal use of quotes
- Follows modern YAML standards
- Better performance for large files
- Consistent with DevOps ecosystem tools

### Requirements

- yq v4+ (mikefarah's Go version) must be installed
- The system must have `yq` command available in PATH

### Installation

**Ubuntu/Debian:**

```bash
sudo apt install yq
```

**macOS:**

```bash
brew install yq
```

**Manual binary installation:**

```bash
wget https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -O /usr/bin/yq && chmod +x /usr/bin/yq
```

### YQ Output Example

```yaml
version: '3.8'
services:
  web:
    image: nginx:alpine
    ports:
      - "8080:80"
    command: nginx -g 'daemon off;'
```

## Rust Merger

### Rust Advantages

- No external dependencies
- Works out of the box
- Consistent behavior across environments
- Better error handling for invalid YAML

### Dependencies

- yaml-rust2 (built-in)
- serde_yaml (built-in)

### Rust Output Example

```yaml
---
version: "3.8"
services:
  web:
    image: "nginx:alpine"
    ports:
      - "8080:80"
    command: "nginx -g 'daemon off;'"
```

## Error Handling

### YQ Not Available

If `yaml_merger = "yq"` is configured but yq is not installed, StackBuilder will show an error:

```log
Error: yq is required but not available. Please either:
1. Install yq v4+ from https://github.com/mikefarah/yq
2. Or set yaml_merger = "rust" in your stackbuilder.toml config file
```

### Wrong YQ Version

If you have yq v3 (Python version) installed, you'll see:

```log
Error: Wrong yq version detected. Please install yq v4+ from mikefarah (Go version).
Current version: yq 3.4.3
Required: yq v4+ from https://github.com/mikefarah/yq
```

## Migration Guide

### From Default (yq) to Rust

If you want to use the Rust merger, add this to your config:

```toml
[build]
yaml_merger = "rust"
```

### From Rust to YQ

1. Install yq v4+
2. Either remove the `yaml_merger` parameter (defaults to "yq") or set it explicitly:

```toml
[build]
yaml_merger = "yq"
```

## Performance Comparison

| Aspect | YQ Merger | Rust Merger |
|--------|-----------|-------------|
| **Startup Time** | Slower (external process) | Faster (in-memory) |
| **Large Files** | Better | Good |
| **Dependencies** | Requires yq | None |
| **Output Quality** | Cleaner | More verbose |
| **Error Messages** | Good | Excellent |

## Troubleshooting

### Common Issues

1. **Command not found: yq**
   - Install yq v4+ using the instructions above

2. **Wrong yq version**
   - Remove old yq: `sudo apt remove yq` (if installed via apt)
   - Install yq v4+ using the manual binary method

3. **Permission denied**
   - Ensure yq binary has execute permissions: `chmod +x /usr/bin/yq`

4. **PATH issues**
   - Verify yq is in PATH: `which yq`
   - Add yq location to PATH if needed

### Validation

Test your yq installation:

```bash
yq --version
# Should output: yq (https://github.com/mikefarah/yq/) version v4.x.x
```

## Best Practices

1. **Use yq merger for production environments** - cleaner output and better toolchain integration
2. **Use rust merger for development** - no external dependencies, faster builds
3. **Document your choice** - include yaml_merger setting in your project documentation
4. **Test both mergers** - ensure your YAML structures work with both approaches

## Configuration Examples

### Basic Configuration (YQ - Default)

```toml
[build]
environments = ["dev", "prod"]
extensions = ["logging", "monitoring"]
# yaml_merger = "yq" # This is the default
```

### Rust Merger Configuration

```toml
[build]
environments = ["dev", "prod"]
extensions = ["logging", "monitoring"]
yaml_merger = "rust"
```

### Mixed Environment Setup

For teams where some developers don't have yq installed:

**stackbuilder.toml** (default):

```toml
[build]
yaml_merger = "yq"
```

**stackbuilder.local.toml** (for developers without yq):

```toml
[build]
yaml_merger = "rust"
