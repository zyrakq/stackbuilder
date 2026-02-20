#!/bin/bash

set -e

TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-pc-windows-gnu"
    "x86_64-apple-darwin"
    "aarch64-unknown-linux-gnu"
    "aarch64-apple-darwin"
)

echo "Building for all targets..."
echo

# Code quality checks
echo "=== Code Quality Checks ==="
echo "→ Formatting code..."
cargo fmt

echo "→ Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "→ Running tests..."
cargo test --all-features

echo "✅ Code quality checks passed"
echo

SUCCESS=()
FAILED=()

for target in "${TARGETS[@]}"; do
    echo "Building for $target..."
    if cargo build --release --target "$target"; then
        SUCCESS+=("$target")
        echo "✓ $target - SUCCESS"
    else
        FAILED+=("$target")
        echo "✗ $target - FAILED"
    fi
    echo
done

echo "=== RUST BUILD SUMMARY ==="
echo "Successful builds (${#SUCCESS[@]}):"
for target in "${SUCCESS[@]}"; do
    echo "  ✓ $target"
done

if [ ${#FAILED[@]} -gt 0 ]; then
    echo "Failed builds (${#FAILED[@]}):"
    for target in "${FAILED[@]}"; do
        echo "  ✗ $target"
    done
fi

echo
echo "Rust binaries available in target/*/release/"

# Build AppImage
echo
echo "=== Building AppImage ==="
if [ -f "./packaging/appimage/build-appimage.sh" ]; then
    chmod +x ./packaging/appimage/build-appimage.sh
    if ./packaging/appimage/build-appimage.sh; then
        echo "✅ AppImage built successfully"
        echo "AppImage available in build/appimage/"
    else
        echo "⚠️  AppImage build failed or skipped (this is OK for testing)"
    fi
else
    echo "⚠️  AppImage build script not found"
fi

# Build Arch Linux packages
echo
echo "=== Building Arch Linux packages ==="
if [ -f "./build-packages.sh" ]; then
    chmod +x ./build-packages.sh
    if ./build-packages.sh arch; then
        echo "✅ Arch Linux packages built successfully"
    else
        echo "⚠️  Some Arch Linux packages failed to build"
    fi
else
    echo "⚠️  build-packages.sh not found"
fi

echo
echo "=== FULL BUILD SUMMARY ==="
echo "✓ Rust binaries: target/*/release/"
echo "✓ AppImage: build/appimage/ (if successful)"
echo "✓ Arch packages: packaging/archlinux/*/ (if successful)"