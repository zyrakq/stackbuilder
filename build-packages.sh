#!/bin/bash

set -e

VERSION="0.1.0"
PACKAGE_NAME="stackbuilder"

echo "Building packages for multiple distributions..."
echo "Package: $PACKAGE_NAME"
echo "Version: $VERSION"
echo

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to build Arch Linux package
build_arch() {
    echo "=== Building Arch Linux package ==="
    if ! command_exists makepkg; then
        echo "❌ makepkg not found. Install base-devel package on Arch Linux."
        return 1
    fi
    
    ORIGINAL_DIR="$(pwd)"
    cd packaging/archlinux
    makepkg -sf --noconfirm
    echo "✅ Arch Linux package built successfully"
    echo "Package location: packaging/archlinux/*.pkg.tar.zst"
    cd "$ORIGINAL_DIR"
}

# Function to build Debian package
build_debian() {
    echo "=== Building Debian/Ubuntu package ==="
    local ORIGINAL_DIR="$(pwd)"
    
    if ! command_exists dpkg-deb; then
        echo "❌ dpkg-deb not found. Install dpkg package."
        return 1
    fi
    
    # Clean previous builds
    rm -rf packaging/debian/build
    mkdir -p packaging/debian/build
    
    # Create source tarball and extract to build directory
    tar --exclude='.git' --exclude='target' --exclude='packaging/debian/build' \
        --exclude='packaging/archlinux/src' --exclude='packaging/archlinux/pkg' \
        --transform "s,^,${PACKAGE_NAME}-${VERSION}/," \
        -czf "packaging/debian/build/source.tar.gz" .
    
    cd packaging/debian/build
    tar -xzf source.tar.gz
    cd "${PACKAGE_NAME}-${VERSION}"
    
    # Build using manual debian/rules
    if make -f packaging/debian/debian/rules binary; then
        echo "✅ Debian package built successfully"
        echo "Package location: packaging/debian/build/*.deb"
        cd "$ORIGINAL_DIR"
        return 0
    else
        echo "❌ Debian package build failed"
        cd "$ORIGINAL_DIR"
        return 1
    fi
}

# Function to build Fedora/RPM package
build_fedora() {
    echo "=== Building Fedora/RPM package ==="
    local ORIGINAL_DIR="$(pwd)"
    
    if ! command_exists rpmbuild; then
        echo "❌ rpmbuild not found. Install rpm-build package."
        echo "On Arch Linux: sudo pacman -S rpm-tools"
        echo "On Fedora: sudo dnf install rpm-build"
        return 1
    fi
    
    # Clean up previous builds
    rm -rf ~/rpmbuild
    
    # Setup RPM build environment
    mkdir -p ~/rpmbuild/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
    
    # Create source tarball with proper structure
    tar --exclude='.git' --exclude='target' --exclude='packaging' \
        --exclude='/tmp' --transform "s,^,${PACKAGE_NAME}-${VERSION}/," \
        -czf ~/rpmbuild/SOURCES/${PACKAGE_NAME}-${VERSION}.tar.gz .
    
    # Copy spec file using absolute path
    cp "${ORIGINAL_DIR}/packaging/fedora/stackbuilder.spec" ~/rpmbuild/SPECS/
    
    # Build RPM
    if rpmbuild -ba ~/rpmbuild/SPECS/stackbuilder.spec; then
        # Copy built packages to packaging directory
        mkdir -p "${ORIGINAL_DIR}/packaging/fedora/build"
        if cp ~/rpmbuild/RPMS/*/*.rpm "${ORIGINAL_DIR}/packaging/fedora/build/" 2>/dev/null; then
            cp ~/rpmbuild/SRPMS/*.rpm "${ORIGINAL_DIR}/packaging/fedora/build/" 2>/dev/null || true
            echo "✅ Fedora RPM package built successfully"
            echo "Package location: packaging/fedora/build/*.rpm"
            return 0
        else
            echo "❌ RPM files not found after build"
            return 1
        fi
    else
        echo "❌ RPM build failed"
        return 1
    fi
}

# Main build logic
DISTROS=()
FAILED=()

# Parse command line arguments
if [ $# -eq 0 ]; then
    # Build all if no arguments provided
    DISTROS=("arch" "debian" "fedora")
else
    DISTROS=("$@")
fi

echo "Building packages for: ${DISTROS[*]}"
echo

for distro in "${DISTROS[@]}"; do
    case $distro in
        arch|archlinux)
            if build_arch; then
                echo
            else
                FAILED+=("arch")
                echo
            fi
            ;;
        debian|ubuntu)
            if build_debian; then
                echo
            else
                FAILED+=("debian")
                echo
            fi
            ;;
        fedora|rpm)
            if build_fedora; then
                echo
            else
                FAILED+=("fedora")
                echo
            fi
            ;;
        *)
            echo "❌ Unknown distribution: $distro"
            echo "Supported: arch, debian, fedora"
            FAILED+=("$distro")
            echo
            ;;
    esac
done

# Summary
echo "=== BUILD SUMMARY ==="
if [ ${#FAILED[@]} -eq 0 ]; then
    echo "✅ All packages built successfully!"
else
    echo "❌ Failed builds: ${FAILED[*]}"
    exit 1
fi