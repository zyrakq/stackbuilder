#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLES_DIR="$SCRIPT_DIR/examples"

echo "Building all examples..."
echo "========================"

if [ ! -d "$EXAMPLES_DIR" ]; then
    echo "Error: Examples directory not found!"
    exit 1
fi

# Count the examples
count=0
for dir in "$EXAMPLES_DIR"/*/; do
    if [ -d "$dir" ]; then
        count=$((count + 1))
    fi
done

echo "Found $count examples"
echo ""

# Build each example
success=0
failed=0
failed_examples=""

for dir in "$EXAMPLES_DIR"/*/; do
    if [ -d "$dir" ]; then
        name=$(basename "$dir")
        log_file="/tmp/build-$name.log"
        echo -n "Building $name... "
        
        if (cd "$dir" && cargo run --quiet -- build > "$log_file" 2>&1); then
            echo "✓ OK"
            success=$((success + 1))
            rm -f "$log_file"
        else
            echo "✗ FAILED"
            failed=$((failed + 1))
            failed_examples="$failed_examples $name"
            # Show the cause of the failure
            if [ -f "$log_file" ]; then
                error_msg=$(tail -1 "$log_file")
                echo "    $error_msg"
            fi
        fi
    fi
done

echo ""
echo "========================"
echo "Results: $success/$count passed"

if [ $failed -gt 0 ]; then
    echo "Failed: $failed"
    echo ""
    echo "Failed examples:$failed_examples"
    echo ""
    echo "Logs available in /tmp/build-<example>.log"
    exit 1
fi

echo "All examples built successfully!"
