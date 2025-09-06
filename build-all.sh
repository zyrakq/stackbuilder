#!/bin/bash

TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-pc-windows-gnu"
    "x86_64-apple-darwin"
    "aarch64-unknown-linux-gnu"
    "aarch64-apple-darwin"
)

echo "Building for all targets..."
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

echo "=== BUILD SUMMARY ==="
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
echo "Binaries available in target/*/release/"