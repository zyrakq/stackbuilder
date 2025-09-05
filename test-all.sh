#!/bin/bash

# Comprehensive test script for stackbuilder
# Tests all examples, validates YAML output, and performs performance testing

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Test results log
TEST_LOG="test_results_$(date +%Y%m%d_%H%M%S).log"

# Performance tracking
PERFORMANCE_LOG="performance_$(date +%Y%m%d_%H%M%S).log"

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$TEST_LOG"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1" | tee -a "$TEST_LOG"
    ((PASSED_TESTS++))
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1" | tee -a "$TEST_LOG"
    ((FAILED_TESTS++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1" | tee -a "$TEST_LOG"
}

# Build stackbuilder binary
build_binary() {
    log_info "Building stackbuilder binary..."
    if cargo build --release; then
        log_success "Binary built successfully"
        STACKBUILDER_BIN="./target/release/stackbuilder"
    else
        log_error "Failed to build binary"
        exit 1
    fi
}

# Test YAML validation
validate_yaml() {
    local file="$1"
    log_info "Validating YAML syntax: $file"
    
    if command -v python3 &> /dev/null; then
        if python3 -c "import yaml; yaml.safe_load(open('$file'))" 2>/dev/null; then
            log_success "YAML validation passed: $file"
            return 0
        else
            log_error "YAML validation failed: $file"
            return 1
        fi
    else
        log_warning "Python3 not available, skipping YAML validation for $file"
        return 0
    fi
}

# Test docker-compose validation
validate_docker_compose() {
    local file="$1"
    log_info "Validating docker-compose syntax: $file"
    
    if command -v docker-compose &> /dev/null; then
        if docker-compose -f "$file" config --quiet 2>/dev/null; then
            log_success "Docker-compose validation passed: $file"
            return 0
        else
            log_error "Docker-compose validation failed: $file"
            return 1
        fi
    else
        log_warning "docker-compose not available, skipping validation for $file"
        return 0
    fi
}

# Test stackbuilder init command
test_init_command() {
    local test_dir="test_init_$(date +%s)"
    
    log_info "Testing stackbuilder init command..."
    ((TOTAL_TESTS++))
    
    mkdir -p "$test_dir"
    cd "$test_dir"
    
    if "$STACKBUILDER_BIN" init --skip-folders; then
        if [ -f "stackbuilder.toml" ]; then
            log_success "Init command created config file"
            validate_yaml "stackbuilder.toml" || true
        else
            log_error "Init command did not create config file"
        fi
    else
        log_error "Init command failed"
    fi
    
    cd ..
    rm -rf "$test_dir"
}

# Test example building with performance measurement
test_example() {
    local example_dir="$1"
    local example_name=$(basename "$example_dir")
    
    log_info "Testing example: $example_name"
    ((TOTAL_TESTS++))
    
    cd "$example_dir"
    
    # Clean previous build
    rm -rf build output 2>/dev/null || true
    
    # Measure build time
    local start_time=$(date +%s.%N)
    
    if "$STACKBUILDER_BIN" build; then
        local end_time=$(date +%s.%N)
        local duration=$(echo "$end_time - $start_time" | bc -l)
        
        echo "$example_name: ${duration}s" >> "../../$PERFORMANCE_LOG"
        log_success "Example $example_name built successfully (${duration}s)"
        
        # Count generated files
        local file_count=0
        for build_dir in build output; do
            if [ -d "$build_dir" ]; then
                file_count=$(find "$build_dir" -name "*.yml" -o -name "*.yaml" | wc -l)
                log_info "Generated $file_count YAML files in $build_dir"
                
                # Validate all generated YAML files
                find "$build_dir" -name "*.yml" -o -name "*.yaml" | while read -r file; do
                    validate_yaml "$file" || true
                    validate_docker_compose "$file" || true
                done
            fi
        done
        
    else
        log_error "Example $example_name build failed"
    fi
    
    cd - > /dev/null
}

# Test error cases
test_error_cases() {
    log_info "Testing error cases..."
    
    for error_case in examples/error-cases/*/; do
        if [ -d "$error_case" ]; then
            local case_name=$(basename "$error_case")
            log_info "Testing error case: $case_name"
            ((TOTAL_TESTS++))
            
            cd "$error_case"
            
            # Error cases should fail, so we expect non-zero exit codes
            if "$STACKBUILDER_BIN" build 2>/dev/null; then
                log_error "Error case $case_name should have failed but succeeded"
            else
                log_success "Error case $case_name correctly failed"
            fi
            
            cd - > /dev/null
        fi
    done
}

# Performance stress test
performance_stress_test() {
    log_info "Running performance stress test..."
    ((TOTAL_TESTS++))
    
    local stress_dir="stress_test_$(date +%s)"
    mkdir -p "$stress_dir"
    cd "$stress_dir"
    
    # Create a large configuration
    cat > stackbuilder.toml << EOF
[paths]
components_dir = "./components"
base_dir = "base"
environments_dir = "environments"
extensions_dirs = ["extensions"]
build_dir = "./build"

[build]
environments = ["env1", "env2", "env3", "env4", "env5"]
extensions = ["ext1", "ext2", "ext3", "ext4", "ext5"]
EOF
    
    # Create base service with many services
    mkdir -p components/base
    cat > components/base/docker-compose.yml << EOF
version: '3.8'
services:
$(for i in {1..20}; do
    echo "  service$i:"
    echo "    image: nginx:alpine"
    echo "    ports:"
    echo "      - \"$((8000+i)):80\""
done)
EOF
    
    # Create environments and extensions
    for env in env1 env2 env3 env4 env5; do
        mkdir -p "components/environments/$env"
        echo "version: '3.8'" > "components/environments/$env/docker-compose.yml"
        echo "services:" >> "components/environments/$env/docker-compose.yml"
        echo "  service1:" >> "components/environments/$env/docker-compose.yml"
        echo "    environment:" >> "components/environments/$env/docker-compose.yml"
        echo "      - ENV=$env" >> "components/environments/$env/docker-compose.yml"
    done
    
    for ext in ext1 ext2 ext3 ext4 ext5; do
        mkdir -p "components/extensions/$ext"
        echo "version: '3.8'" > "components/extensions/$ext/docker-compose.yml"
        echo "services:" >> "components/extensions/$ext/docker-compose.yml"
        echo "  ${ext}-service:" >> "components/extensions/$ext/docker-compose.yml"
        echo "    image: alpine:latest" >> "components/extensions/$ext/docker-compose.yml"
    done
    
    # Measure performance
    local start_time=$(date +%s.%N)
    
    if "$STACKBUILDER_BIN" build; then
        local end_time=$(date +%s.%N)
        local duration=$(echo "$end_time - $start_time" | bc -l)
        
        echo "stress_test: ${duration}s" >> "../$PERFORMANCE_LOG"
        
        local generated_files=$(find build -name "*.yml" | wc -l)
        log_success "Stress test completed: ${duration}s, $generated_files files generated"
        
        # Memory usage approximation
        if command -v du &> /dev/null; then
            local build_size=$(du -sh build | cut -f1)
            log_info "Build directory size: $build_size"
        fi
        
    else
        log_error "Stress test failed"
    fi
    
    cd ..
    rm -rf "$stress_dir"
}

# Edge case tests
test_edge_cases() {
    log_info "Testing edge cases..."
    
    # Test empty docker-compose files
    local edge_dir="edge_test_$(date +%s)"
    mkdir -p "$edge_dir/components/base"
    cd "$edge_dir"
    
    cat > stackbuilder.toml << EOF
[paths]
components_dir = "./components"
base_dir = "base"
build_dir = "./build"

[build]
extensions = []
EOF
    
    # Create minimal valid docker-compose
    echo "version: '3.8'" > components/base/docker-compose.yml
    echo "services:" >> components/base/docker-compose.yml
    echo "  minimal:" >> components/base/docker-compose.yml
    echo "    image: alpine:latest" >> components/base/docker-compose.yml
    
    ((TOTAL_TESTS++))
    if "$STACKBUILDER_BIN" build; then
        log_success "Edge case: minimal configuration works"
    else
        log_error "Edge case: minimal configuration failed"
    fi
    
    cd ..
    rm -rf "$edge_dir"
}

# Main test execution
main() {
    log_info "Starting comprehensive stackbuilder testing..."
    log_info "Test results will be logged to: $TEST_LOG"
    log_info "Performance results will be logged to: $PERFORMANCE_LOG"
    
    # Initialize performance log
    echo "# Stackbuilder Performance Test Results - $(date)" > "$PERFORMANCE_LOG"
    echo "# Format: example_name: duration_in_seconds" >> "$PERFORMANCE_LOG"
    
    # Build the binary
    build_binary
    
    # Test init command
    test_init_command
    
    # Test all examples
    log_info "Testing all examples..."
    for example in examples/*/; do
        if [ -d "$example" ] && [ "$example" != "examples/error-cases/" ]; then
            test_example "$example"
        fi
    done
    
    # Test error cases
    test_error_cases
    
    # Performance and edge case tests
    performance_stress_test
    test_edge_cases
    
    # Generate summary
    log_info "=========================================="
    log_info "TEST SUMMARY"
    log_info "=========================================="
    log_info "Total tests: $TOTAL_TESTS"
    log_success "Passed: $PASSED_TESTS"
    log_error "Failed: $FAILED_TESTS"
    
    local success_rate=0
    if [ "$TOTAL_TESTS" -gt 0 ]; then
        success_rate=$(echo "scale=2; $PASSED_TESTS * 100 / $TOTAL_TESTS" | bc -l)
    fi
    log_info "Success rate: ${success_rate}%"
    
    # Performance summary
    if [ -f "$PERFORMANCE_LOG" ]; then
        log_info "Performance results:"
        tail -n +3 "$PERFORMANCE_LOG" | while read -r line; do
            log_info "  $line"
        done
    fi
    
    log_info "Detailed logs: $TEST_LOG"
    log_info "Performance logs: $PERFORMANCE_LOG"
    
    if [ "$FAILED_TESTS" -eq 0 ]; then
        log_success "All tests passed! 🎉"
        exit 0
    else
        log_error "Some tests failed. Check logs for details."
        exit 1
    fi
}

# Check dependencies
check_dependencies() {
    local missing_deps=0
    
    if ! command -v cargo &> /dev/null; then
        log_error "cargo is required but not installed"
        ((missing_deps++))
    fi
    
    if ! command -v bc &> /dev/null; then
        log_warning "bc is not installed - performance timing may be inaccurate"
    fi
    
    if ! command -v python3 &> /dev/null; then
        log_warning "python3 is not installed - YAML validation will be skipped"
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        log_warning "docker-compose is not installed - compose validation will be skipped"
    fi
    
    if [ "$missing_deps" -gt 0 ]; then
        log_error "Missing required dependencies. Please install them and try again."
        exit 1
    fi
}

# Script entry point
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    check_dependencies
    main "$@"
fi