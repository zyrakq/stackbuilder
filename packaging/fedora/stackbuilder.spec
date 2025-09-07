Name:           stackbuilder
Version:        0.1.0
Release:        1%{?dist}
Summary:        A powerful CLI tool for building Docker Compose files from modular components

License:        MIT or Apache-2.0
URL:            https://github.com/zyrakq/stackbuilder
Source0:        https://github.com/zyrakq/stackbuilder/archive/v%{version}.tar.gz#/%{name}-%{version}.tar.gz

# BuildRequires skipped for cross-distro compatibility
# On actual Fedora system, uncomment these:
# BuildRequires:  rust >= 1.70
# BuildRequires:  cargo
# BuildRequires:  gcc

Requires:       yq

%description
StackBuilder is a command-line tool that helps you build Docker Compose files
from modular components with multi-environment support and extension system.

Key features:
- Modular component system for reusable configurations
- Multi-environment support (development, staging, production)
- Extension system for additional functionality
- YAML merging and templating capabilities
- Simple configuration management

%prep
%setup -q -n %{name}-%{version}

%build
# Build step - assumes cargo/rust are available in PATH
if command -v cargo >/dev/null 2>&1; then
    export RUSTFLAGS="${RUSTFLAGS:-}"
    cargo build --release --all-features
else
    echo "Warning: cargo not found, assuming pre-built binary"
fi

%install
    install -Dm755 target/release/stackbuilder %{buildroot}%{_bindir}/stackbuilder
    ln -s stackbuilder %{buildroot}%{_bindir}/sb
    
    # Install documentation
    install -dm755 %{buildroot}%{_docdir}/%{name}
    install -Dm644 docs/config.md %{buildroot}%{_docdir}/%{name}/config.md
    install -Dm644 docs/build.md %{buildroot}%{_docdir}/%{name}/build.md
    install -Dm644 docs/testing-report.md %{buildroot}%{_docdir}/%{name}/testing-report.md
    install -Dm644 docs/yaml-merger.md %{buildroot}%{_docdir}/%{name}/yaml-merger.md
    
    # Install examples
    cp -r examples %{buildroot}%{_docdir}/%{name}/

# Install licenses (will be handled by %license directive)

%check
# Test step - skip if cargo not available
if command -v cargo >/dev/null 2>&1; then
    cargo test --release --all-features
else
    echo "Skipping tests - cargo not available"
fi

%files
%license LICENSE-MIT LICENSE-APACHE
%doc README.md
%{_bindir}/stackbuilder
%{_bindir}/sb
%{_docdir}/%{name}/config.md
%{_docdir}/%{name}/build.md
%{_docdir}/%{name}/testing-report.md
%{_docdir}/%{name}/yaml-merger.md
%{_docdir}/%{name}/examples/

%changelog
* Sat Sep 07 2025 Zyrakq <serg.shehov@tutanota.com> - 0.1.0-1
- Initial package for Fedora
- A powerful CLI tool for building Docker Compose files from modular components
- Multi-environment support and extension system
- YAML merging and templating capabilities