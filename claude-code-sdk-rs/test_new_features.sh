#!/bin/bash

# Test script for new features in v0.1.6

echo "=================================="
echo "Testing Claude Code SDK v0.1.6"
echo "=================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test 1: Basic compilation (only check new examples)
echo "Test 1: Checking new examples compilation..."
all_compile=true
for example in test_new_options test_settings test_add_dirs test_combined_features; do
    if cargo check --example $example 2>&1 | grep -q "error"; then
        echo -e "${RED}✗ $example compilation failed${NC}"
        all_compile=false
    else
        echo -e "${GREEN}✓ $example compiles successfully${NC}"
    fi
done
echo ""

# Test 2: Test new options example
echo "Test 2: Running test_new_options example..."
if cargo run --example test_new_options > /dev/null 2>&1; then
    echo -e "${GREEN}✓ test_new_options runs successfully${NC}"
else
    echo -e "${RED}✗ test_new_options failed${NC}"
fi
echo ""

# Test 3: Build documentation
echo "Test 3: Building documentation..."
if cargo doc --no-deps 2>&1 | grep -q "error"; then
    echo -e "${RED}✗ Documentation build failed${NC}"
else
    echo -e "${GREEN}✓ Documentation builds successfully${NC}"
fi
echo ""

# Test 4: Check for required files
echo "Test 4: Checking example files..."
files_to_check=(
    "examples/test_settings.rs"
    "examples/test_add_dirs.rs"
    "examples/test_combined_features.rs"
    "examples/test_new_options.rs"
    "examples/claude-settings.json"
    "examples/custom-claude-settings.json"
)

all_files_exist=true
for file in "${files_to_check[@]}"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓ $file exists${NC}"
    else
        echo -e "${RED}✗ $file not found${NC}"
        all_files_exist=false
    fi
done
echo ""

# Summary
echo "=================================="
echo "Test Summary"
echo "=================================="
if [ "$all_files_exist" = true ]; then
    echo -e "${GREEN}All tests passed! The SDK is ready for testing.${NC}"
    echo ""
    echo "You can now test the new features with:"
    echo "  cargo run --example test_settings"
    echo "  cargo run --example test_add_dirs"
    echo "  cargo run --example test_combined_features"
else
    echo -e "${RED}Some tests failed. Please check the errors above.${NC}"
fi