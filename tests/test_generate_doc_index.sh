#!/bin/bash
# Test script for generate-doc-index.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Test counter
PASSED=0
FAILED=0

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT_PATH="$SCRIPT_DIR/../scripts/generate-doc-index.sh"

# Setup test environment
setup_test() {
    TEST_DIR=$(mktemp -d)
    mkdir -p "$TEST_DIR/target/doc"
    mkdir -p "$TEST_DIR/rust/engine"
    mkdir -p "$TEST_DIR/rust/cli"
    mkdir -p "$TEST_DIR/rust/web"
    mkdir -p "$TEST_DIR/scripts"

    # Copy script to test directory
    cp "$SCRIPT_PATH" "$TEST_DIR/scripts/"

    # Create mock Cargo.toml
    cat > "$TEST_DIR/Cargo.toml" << 'EOF'
[workspace]
members = [
    "rust/engine",
    "rust/cli",
    "rust/web"
]
resolver = "2"
EOF

    # Create mock rustdoc directories
    mkdir -p "$TEST_DIR/target/doc/axiomind_engine"
    mkdir -p "$TEST_DIR/target/doc/axiomind_cli"
    mkdir -p "$TEST_DIR/target/doc/axiomind_web"

    echo "$TEST_DIR"
}

cleanup_test() {
    rm -rf "$1"
}

assert_file_exists() {
    local file=$1
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} File exists: $file"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}✗${NC} File not found: $file"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

assert_contains() {
    local file=$1
    local pattern=$2
    local description=$3

    if grep -q "$pattern" "$file"; then
        echo -e "${GREEN}✓${NC} $description"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}✗${NC} $description"
        echo "  Expected pattern: $pattern"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

assert_valid_html() {
    local file=$1

    # Check basic HTML structure
    if grep -q "<!DOCTYPE html>" "$file" && \
       grep -q "<html" "$file" && \
       grep -q "</html>" "$file" && \
       grep -q "<head>" "$file" && \
       grep -q "<body>" "$file"; then
        echo -e "${GREEN}✓${NC} Valid HTML structure"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}✗${NC} Invalid HTML structure"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Test 1: Script generates index.html
test_generates_index_html() {
    echo "Test 1: Generates index.html"

    TEST_DIR=$(setup_test)
    cd "$TEST_DIR"

    # Run the script
    bash "$TEST_DIR/scripts/generate-doc-index.sh" || true

    assert_file_exists "$TEST_DIR/target/doc/index.html"

    cleanup_test "$TEST_DIR"
    echo ""
}

# Test 2: index.html contains valid HTML structure
test_valid_html_structure() {
    echo "Test 2: Valid HTML structure"

    TEST_DIR=$(setup_test)
    cd "$TEST_DIR"

    bash "$TEST_DIR/scripts/generate-doc-index.sh" || true

    if [ -f "$TEST_DIR/target/doc/index.html" ]; then
        assert_valid_html "$TEST_DIR/target/doc/index.html"
    else
        echo -e "${RED}✗${NC} index.html not generated"
        FAILED=$((FAILED + 1))
    fi

    cleanup_test "$TEST_DIR"
    echo ""
}

# Test 3: index.html contains crate links
test_contains_crate_links() {
    echo "Test 3: Contains crate links"

    TEST_DIR=$(setup_test)
    cd "$TEST_DIR"

    bash "$TEST_DIR/scripts/generate-doc-index.sh" || true

    if [ -f "$TEST_DIR/target/doc/index.html" ]; then
        assert_contains "$TEST_DIR/target/doc/index.html" "axiomind_engine" "Contains axiomind_engine link"
        assert_contains "$TEST_DIR/target/doc/index.html" "axiomind_cli" "Contains axiomind_cli link"
        assert_contains "$TEST_DIR/target/doc/index.html" "axiomind_web" "Contains axiomind_web link"
    else
        echo -e "${RED}✗${NC} index.html not generated"
        FAILED=$((FAILED + 1))
    fi

    cleanup_test "$TEST_DIR"
    echo ""
}

# Test 4: index.html contains project overview
test_contains_project_overview() {
    echo "Test 4: Contains project overview"

    TEST_DIR=$(setup_test)
    cd "$TEST_DIR"

    bash "$TEST_DIR/scripts/generate-doc-index.sh" || true

    if [ -f "$TEST_DIR/target/doc/index.html" ]; then
        assert_contains "$TEST_DIR/target/doc/index.html" "Axiomind" "Contains project name"
        assert_contains "$TEST_DIR/target/doc/index.html" "Documentation" "Contains documentation heading"
    else
        echo -e "${RED}✗${NC} index.html not generated"
        FAILED=$((FAILED + 1))
    fi

    cleanup_test "$TEST_DIR"
    echo ""
}

# Test 5: index.html is responsive (contains viewport meta)
test_responsive_design() {
    echo "Test 5: Responsive design"

    TEST_DIR=$(setup_test)
    cd "$TEST_DIR"

    bash "$TEST_DIR/scripts/generate-doc-index.sh" || true

    if [ -f "$TEST_DIR/target/doc/index.html" ]; then
        assert_contains "$TEST_DIR/target/doc/index.html" "viewport" "Contains viewport meta tag"
    else
        echo -e "${RED}✗${NC} index.html not generated"
        FAILED=$((FAILED + 1))
    fi

    cleanup_test "$TEST_DIR"
    echo ""
}

# Run all tests
echo "==================================="
echo "Running tests for generate-doc-index.sh"
echo "==================================="
echo ""

test_generates_index_html
test_valid_html_structure
test_contains_crate_links
test_contains_project_overview
test_responsive_design

# Print summary
echo "==================================="
echo "Test Summary"
echo "==================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
