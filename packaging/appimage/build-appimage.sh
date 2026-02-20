#!/bin/bash

# Build AppImage for stackbuilder
# Usage: ./build-appimage.sh [x86_64|aarch64]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
VERSION=$(grep "^version = " "${PROJECT_ROOT}/Cargo.toml" | sed 's/version = "//;s/"$//')
PKGNAME="stackbuilder"

# Determine architecture
ARCH="${1:-$(uname -m)}"
case "${ARCH}" in
    x86_64) ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "Unsupported architecture: ${ARCH}"; exit 1 ;;
esac

echo "Building AppImage for ${PKGNAME} v${VERSION} (${ARCH})"
echo "================================================"

# Check for required tools
check_requirements() {
    local missing=()
    
    if ! command -v appimagetool &> /dev/null; then
        missing+=("appimagetool")
    fi
    
    if ! command -v cargo &> /dev/null; then
        missing+=("cargo")
    fi
    
    if [ ${#missing[@]} -ne 0 ]; then
        echo "Error: Missing required tools: ${missing[*]}"
        echo ""
        echo "Install on Arch Linux:"
        echo "  sudo pacman -S rust"
        echo "  yay -S appimagetool-bin  # or download from https://github.com/AppImage/AppImageKit/releases"
        exit 1
    fi
}

# Create AppDir structure
prepare_appdir() {
    echo "→ Preparing AppDir structure..."
    
    local appdir="${PROJECT_ROOT}/build/appimage/AppDir"
    rm -rf "${appdir}"
    mkdir -p "${appdir}/usr/bin"
    mkdir -p "${appdir}/usr/share/doc/${PKGNAME}"
    mkdir -p "${appdir}/usr/share/licenses/${PKGNAME}"
    
    # Build the binary
    echo "→ Building release binary for ${ARCH}..."
    cd "${PROJECT_ROOT}"
    
    # Set appropriate target
    local rust_target="${ARCH}-unknown-linux-gnu"
    if [ "${ARCH}" == "aarch64" ]; then
        # Check if aarch64 target is installed
        if ! rustup target list --installed | grep -q "${rust_target}"; then
            echo "Installing Rust target: ${rust_target}"
            rustup target add "${rust_target}"
        fi
    fi
    
    RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target "${rust_target}"
    
    # Copy binary
    cp "${PROJECT_ROOT}/target/${rust_target}/release/${PKGNAME}" "${appdir}/usr/bin/"
    chmod +x "${appdir}/usr/bin/${PKGNAME}"
    
    # Copy documentation
    cp "${PROJECT_ROOT}/README.md" "${appdir}/usr/share/doc/${PKGNAME}/"
    cp "${PROJECT_ROOT}/LICENSE-MIT" "${appdir}/usr/share/licenses/${PKGNAME}/"
    cp "${PROJECT_ROOT}/LICENSE-APACHE" "${appdir}/usr/share/licenses/${PKGNAME}/"
    
    # Copy examples if they exist
    if [ -d "${PROJECT_ROOT}/examples" ]; then
        cp -r "${PROJECT_ROOT}/examples" "${appdir}/usr/share/doc/${PKGNAME}/"
    fi
    
    # Copy AppRun script
    cp "${SCRIPT_DIR}/AppRun" "${appdir}/AppRun"
    chmod +x "${appdir}/AppRun"
    
    # Create desktop file
    echo "→ Creating desktop file..."
    cat > "${appdir}/${PKGNAME}.desktop" << EOF
[Desktop Entry]
Type=Application
Name=StackBuilder
Comment=A powerful CLI tool for building Docker Compose files
Exec=${PKGNAME}
Icon=${PKGNAME}
Terminal=true
Categories=Development;System;
Keywords=docker;compose;devops;cli;
EOF
    
    # Copy icon if exists
    if [ -f "${PROJECT_ROOT}/logo.svg" ]; then
        echo "→ Copying icon..."
        cp "${PROJECT_ROOT}/logo.svg" "${appdir}/${PKGNAME}.svg"
    fi
    
    echo "✓ AppDir prepared"
}

# Build AppImage
build_appimage() {
    echo "→ Building AppImage..."
    
    local appdir="${PROJECT_ROOT}/build/appimage/AppDir"
    local output_name="${PKGNAME}-${VERSION}-${ARCH}.AppImage"
    local output_dir="${PROJECT_ROOT}/build/appimage"
    
    mkdir -p "${output_dir}"
    
    cd "${output_dir}"
    appimagetool --appimage-extract-and-run "${appdir}" "${output_name}"
    
    echo ""
    echo "================================================"
    echo "✓ AppImage built successfully!"
    echo "  Output: ${output_dir}/${output_name}"
    echo "================================================"
}

# Main
main() {
    check_requirements
    prepare_appdir
    build_appimage
}

main "$@"
