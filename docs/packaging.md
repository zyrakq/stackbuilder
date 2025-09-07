# Packaging Guide

This document describes how to build packages for different Linux distributions.

## Supported Distributions

StackBuilder supports packaging for the following distributions:

- **Arch Linux** - using PKGBUILD
- **Debian/Ubuntu** - using debian packaging system
- **Fedora/RHEL/CentOS** - using RPM spec files

## Quick Start

Use the `build-packages.sh` script to build packages:

```bash
# Build packages for all supported distributions
./build-packages.sh

# Build for specific distribution
./build-packages.sh arch
./build-packages.sh debian
./build-packages.sh fedora

# Build for multiple distributions
./build-packages.sh arch debian
```

## Prerequisites

### Arch Linux

- `base-devel` package group
- `makepkg` tool

```bash
sudo pacman -S base-devel
```

### Debian/Ubuntu

- `devscripts` package
- `build-essential` package
- `debhelper-compat` package

```bash
sudo apt update
sudo apt install devscripts build-essential debhelper-compat
```

### Fedora/RHEL/CentOS

- `rpm-build` package
- `rust` and `cargo`

```bash
sudo dnf install rpm-build rust cargo
# or for older versions
sudo yum install rpm-build rust cargo
```

## Manual Building

### Arch Linux Building

```bash
cd packaging/archlinux
makepkg -sf --noconfirm
```

Output: `*.pkg.tar.zst` files

### Debian/Ubuntu Building

```bash
# Create source tarball
tar --exclude='.git' --exclude='target' --exclude='packaging' \
    -czf packaging/debian/stackbuilder_0.1.0.orig.tar.gz .

# Prepare build directory
mkdir -p packaging/debian/build/stackbuilder-0.1.0
tar -xzf packaging/debian/stackbuilder_0.1.0.orig.tar.gz \
    -C packaging/debian/build/stackbuilder-0.1.0
cp -r packaging/debian/debian packaging/debian/build/stackbuilder-0.1.0/

# Build package
cd packaging/debian/build/stackbuilder-0.1.0
dpkg-buildpackage -us -uc -b
```

Output: `*.deb` files in `packaging/debian/build/`

### Fedora/RPM

```bash
# Setup RPM build environment
mkdir -p ~/rpmbuild/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

# Create source tarball
tar --exclude='.git' --exclude='target' --exclude='packaging' \
    -czf ~/rpmbuild/SOURCES/stackbuilder-0.1.0.tar.gz .

# Copy spec file
cp packaging/fedora/stackbuilder.spec ~/rpmbuild/SPECS/

# Build RPM
rpmbuild -ba ~/rpmbuild/SPECS/stackbuilder.spec
```

Output: `*.rpm` files in `~/rpmbuild/RPMS/` and `~/rpmbuild/SRPMS/`

## Package Structure

All packages install:

- **Binary**: `/usr/bin/stackbuilder`
- **Symlink**: `/usr/bin/sb` → `stackbuilder`
- **Documentation**: `/usr/share/doc/stackbuilder/`
- **Examples**: `/usr/share/doc/stackbuilder/examples/`
- **Licenses**: Distribution-specific license directories

## Dependencies

### Runtime Dependencies

- **yq** - YAML processor (required for YAML operations)
- **gcc-libs** (Arch) / **libc6** (Debian) / **glibc** (Fedora) - C library

### Build Dependencies

- **rust** (>= 1.70) - Rust compiler
- **cargo** - Rust package manager
- **gcc** - C compiler

## Version Management

Update version in the following files when releasing:

1. `Cargo.toml` - package version
2. `packaging/archlinux/PKGBUILD` - pkgver
3. `packaging/archlinux/.SRCINFO` - pkgver (regenerate with `makepkg --printsrcinfo`)
4. `packaging/debian/debian/changelog` - add new entry
5. `packaging/fedora/stackbuilder.spec` - Version field
6. `build-packages.sh` - VERSION variable

## Troubleshooting

### Common Issues

1. **Missing dependencies**: Install the required build tools for your distribution
2. **Permission errors**: Ensure you have write permissions in packaging directories
3. **Rust version**: Ensure you have Rust 1.70 or newer
4. **Network access**: Some builds may require internet access to fetch dependencies

### Debugging

Enable verbose output by modifying the build scripts:

```bash
# For Arch Linux
makepkg -sf --noconfirm --log

# For Debian
dpkg-buildpackage -us -uc -b -v

# For RPM
rpmbuild -ba --verbose ~/rpmbuild/SPECS/stackbuilder.spec
```

## Contributing

When adding support for new distributions:

1. Create a new directory under `packaging/`
2. Add distribution-specific package files
3. Update `build-packages.sh` script
4. Add build instructions to this document
5. Test the packaging on the target distribution

## License

The packaging files are licensed under the same terms as the main project (MIT OR Apache-2.0).
