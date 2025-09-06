# Arch Linux Packaging

This document describes how to build and install stackbuilder on Arch Linux.

## Package Information

- **Package name**: `stackbuilder`
- **Binary name**: `stackbuilder` (with `sb` symlink)
- **Architecture**: x86_64, i686, aarch64
- **Dependencies**: gcc-libs, go-yq
- **Build dependencies**: rust, cargo

## Installation

### Using makepkg (Local Build)

1. Navigate to the packaging directory:

```bash
cd packaging/archlinux
```

1. Build and install the package:

```bash
makepkg -si
```

This will:

- Download the source code
- Build the application
- Install it as `stackbuilder` command
- Create a symlink so `sb` command also works

## Usage

After installation, you can use either command:

```bash
sb --help
# or
stackbuilder --help
```

## Uninstallation

To remove the package:

```bash
sudo pacman -R stackbuilder
```

## Package Contents

The package installs:

- **Binary**: `/usr/bin/stackbuilder`
- **Symlink**: `/usr/bin/sb` → `stackbuilder`
- **Documentation**: `/usr/share/doc/stackbuilder/`
  - README.md
  - config.md
  - build.md
  - testing-report.md
  - yaml-merger.md
  - examples/
- **Licenses**: `/usr/share/licenses/stackbuilder/`
  - LICENSE-MIT
  - LICENSE-APACHE

## Dependencies

### Runtime Dependencies

- `gcc-libs`: Standard C++ runtime libraries
- `go-yq`: YAML processor used by stackbuilder for YAML manipulation

### Build Dependencies

- `rust`: Rust programming language compiler
- `cargo`: Rust package manager

## Building from Source

If you want to build manually without using the PKGBUILD:

```bash
# Install dependencies
sudo pacman -S rust cargo go-yq

# Clone and build
git clone https://github.com/zyrakq/stackbuilder.git
cd stackbuilder
cargo build --release

# Install manually
sudo cp target/release/stackbuilder /usr/local/bin/
sudo ln -s stackbuilder /usr/local/bin/sb
```

## Package Maintenance

To update the package version:

1. Update `pkgver` in `packaging/archlinux/PKGBUILD`
1. Regenerate `.SRCINFO`:

```bash
cd packaging/archlinux
makepkg --printsrcinfo > .SRCINFO
```

1. Update checksums if needed
1. Test the build:

```bash
makepkg -f
```

## Notes

- Both `sb` and `stackbuilder` commands are available after installation
- The package includes all documentation and examples
- go-yq is required as stackbuilder uses it for YAML processing
- The package follows Arch Linux packaging guidelines
