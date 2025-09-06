# Cross-Platform Build Guide

## Prerequisites

### Required Tools

- [Rust](https://rustup.rs/) with stable toolchain
- [Zig](https://ziglang.org/) for macOS cross-compilation

### Install target architectures

```bash
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu  
rustup target add x86_64-apple-darwin
rustup target add aarch64-unknown-linux-gnu
rustup target add aarch64-apple-darwin
```

### Platform-specific requirements

#### Windows (x86_64-pc-windows-gnu)

- **Linux/macOS**: Install mingw-w64
  - Arch Linux: `sudo pacman -S mingw-w64-gcc`
  - Ubuntu/Debian: `sudo apt install mingw-w64`
  - macOS: `brew install mingw-w64`

#### ARM Linux (aarch64-unknown-linux-gnu)  

- **Linux**: Install cross-compiler
  - Arch Linux: `sudo pacman -S aarch64-linux-gnu-gcc`
  - Ubuntu/Debian: `sudo apt install gcc-aarch64-linux-gnu`

#### macOS targets (x86_64-apple-darwin, aarch64-apple-darwin)

- **All platforms**: Install Zig
  - Arch Linux: `sudo pacman -S zig`
  - Ubuntu/Debian: `sudo snap install zig --classic`
  - macOS: `brew install zig`
  - Windows: `scoop install zig`

## Building

### Single target

```bash
cargo build --release --target <target>
```

### All targets

```bash
./build-all.sh
```

## Supported Targets

- `x86_64-unknown-linux-gnu` - Linux x86_64
- `x86_64-pc-windows-gnu` - Windows x86_64  
- `x86_64-apple-darwin` - macOS Intel
- `aarch64-unknown-linux-gnu` - Linux ARM64
- `aarch64-apple-darwin` - macOS Apple Silicon

## Output

Binaries are created in `target/<target>/release/`

## How it works

- **Linux targets**: Native compilation or GCC cross-compilers
- **Windows targets**: mingw-w64 GCC toolchain
- **macOS targets**: Zig as cross-linker (no Apple SDK required)
