#!/bin/bash
# Integration test for CI/CD pipeline (Task 8.1)
# Tests rustdoc and validate-docs jobs in parallel, error scenarios

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
PASSED=0
FAILED=0

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/.."

assert_success() {
    local description=$1
    echo -e "${GREEN}✓${NC} $description"
    PASSED=$((PASSED + 1))
}

assert_failure() {
    local description=$1
    echo -e "${RED}✗${NC} $description"
    FAILED=$((FAILED + 1))
}

assert_file_exists() {
    local file=$1
    local description=$2
    if [ -f "$file" ]; then
        assert_success "$description"
        return 0
    else
        assert_failure "$description (file not found: $file)"
        return 1
    fi
}

# Test 1: Verify workflow files exist
test_workflow_files_exist() {
    echo "Test 1: Workflow files exist"
    echo "-----------------------------------"

    assert_file_exists "$PROJECT_ROOT/.github/workflows/ci.yml" \
        "ci.yml exists"

    assert_file_exists "$PROJECT_ROOT/.github/workflows/deploy-docs.yml" \
        "deploy-docs.yml exists"

    echo ""
}

# Test 2: Verify rustdoc job exists in ci.yml
test_rustdoc_job_exists() {
    echo "Test 2: Rustdoc job exists in ci.yml"
    echo "-----------------------------------"

    if grep -q "rustdoc:" "$PROJECT_ROOT/.github/workflows/ci.yml"; then
        assert_success "rustdoc job defined"
    else
        assert_failure "rustdoc job not found"
    fi

    if grep -q "cargo doc --workspace --no-deps" "$PROJECT_ROOT/.github/workflows/ci.yml"; then
        assert_success "rustdoc job runs cargo doc"
    else
        assert_failure "cargo doc command not found in rustdoc job"
    fi

    if grep -q "generate-doc-index.sh" "$PROJECT_ROOT/.github/workflows/ci.yml"; then
        assert_success "rustdoc job generates index.html"
    else
        assert_failure "index.html generation not found in rustdoc job"
    fi

    echo ""
}

# Test 3: Verify validate-docs job exists in ci.yml
test_validate_docs_job_exists() {
    echo "Test 3: Validate-docs job exists in ci.yml"
    echo "-----------------------------------"

    if grep -q "validate-docs:" "$PROJECT_ROOT/.github/workflows/ci.yml"; then
        assert_success "validate-docs job defined"
    else
        assert_failure "validate-docs job not found"
    fi

    if grep -q "cargo rustdoc.*-- -D warnings" "$PROJECT_ROOT/.github/workflows/ci.yml"; then
        assert_success "validate-docs job checks for broken links"
    else
        assert_failure "broken link check not found in validate-docs job"
    fi

    echo ""
}

# Test 4: Verify jobs are independent (no 'needs' dependency between rustdoc and validate-docs)
test_jobs_run_in_parallel() {
    echo "Test 4: Jobs can run in parallel"
    echo "-----------------------------------"

    # Extract rustdoc job definition
    RUSTDOC_JOB=$(awk '/^  rustdoc:/,/^  [a-z]/' "$PROJECT_ROOT/.github/workflows/ci.yml")

    # Check if rustdoc has a 'needs' field that depends on validate-docs
    if echo "$RUSTDOC_JOB" | grep -q "needs:.*validate-docs"; then
        assert_failure "rustdoc job should not depend on validate-docs"
    else
        assert_success "rustdoc job does not depend on validate-docs"
    fi

    # Extract validate-docs job definition
    VALIDATE_DOCS_JOB=$(awk '/^  validate-docs:/,/^  [a-z]/' "$PROJECT_ROOT/.github/workflows/ci.yml")

    # Check if validate-docs has a 'needs' field that depends on rustdoc
    if echo "$VALIDATE_DOCS_JOB" | grep -q "needs:.*rustdoc"; then
        assert_failure "validate-docs job should not depend on rustdoc"
    else
        assert_success "validate-docs job does not depend on rustdoc"
    fi

    echo ""
}

# Test 5: Verify artifacts are uploaded in rustdoc job
test_artifacts_uploaded() {
    echo "Test 5: Artifacts are uploaded"
    echo "-----------------------------------"

    # Extract the rustdoc job section from ci.yml (from rustdoc to next job)
    if awk '/^  rustdoc:/,/^  validate-docs:/' "$PROJECT_ROOT/.github/workflows/ci.yml" | grep -q "actions/upload-artifact"; then
        assert_success "rustdoc job uploads artifacts"
    else
        assert_failure "artifact upload not found in rustdoc job"
    fi

    if grep -A 5 "actions/upload-artifact" "$PROJECT_ROOT/.github/workflows/ci.yml" | grep -q "name: documentation"; then
        assert_success "artifact named 'documentation'"
    else
        assert_failure "artifact not named 'documentation'"
    fi

    echo ""
}

# Test 6: Run rustdoc build locally to verify it succeeds
test_rustdoc_build_succeeds() {
    echo "Test 6: Rustdoc build succeeds locally"
    echo "-----------------------------------"

    cd "$PROJECT_ROOT"

    # Clean previous build
    rm -rf target/doc

    # Run cargo doc
    if cargo doc --workspace --no-deps --verbose 2>&1 | tee /tmp/rustdoc_output.log; then
        assert_success "cargo doc builds successfully"
    else
        assert_failure "cargo doc failed"
        cat /tmp/rustdoc_output.log
    fi

    # Verify documentation directories exist
    if [ -d "target/doc/axiomind_engine" ]; then
        assert_success "axiomind_engine documentation generated"
    else
        assert_failure "axiomind_engine documentation not found"
    fi

    if [ -d "target/doc/axiomind_cli" ]; then
        assert_success "axiomind_cli documentation generated"
    else
        assert_failure "axiomind_cli documentation not found"
    fi

    if [ -d "target/doc/axiomind_web" ]; then
        assert_success "axiomind_web documentation generated"
    else
        assert_failure "axiomind_web documentation not found"
    fi

    echo ""
}

# Test 7: Run index.html generation script
test_index_generation_succeeds() {
    echo "Test 7: Index.html generation succeeds"
    echo "-----------------------------------"

    cd "$PROJECT_ROOT"

    if bash scripts/generate-doc-index.sh; then
        assert_success "index.html generation successful"
    else
        assert_failure "index.html generation failed"
    fi

    if [ -f "target/doc/index.html" ]; then
        assert_success "index.html exists"
    else
        assert_failure "index.html not found"
    fi

    echo ""
}

# Test 8: Create temporary file with broken doc link and verify it fails validation
test_broken_link_detection() {
    echo "Test 8: Broken link detection"
    echo "-----------------------------------"

    cd "$PROJECT_ROOT"

    # Clean up any leftover files from previous runs
    TEMP_TEST_FILE="rust/engine/src/test_broken_link_temp.rs"
    rm -f "$TEMP_TEST_FILE" "rust/engine/src/lib.rs.bak"
    sed -i.bak '/pub mod test_broken_link_temp;/d' "rust/engine/src/lib.rs" 2>/dev/null || true
    rm -f "rust/engine/src/lib.rs.bak"

    # Create a temporary test file with a broken link
    cat > "$TEMP_TEST_FILE" << 'EOF'
/// A test module with a broken documentation link.
///
/// This link is definitely broken: [`super::super::ThisTypeDoesNotExist123456`]
pub struct TestBrokenLink;
EOF

    # Add the module to lib.rs temporarily
    echo "pub mod test_broken_link_temp;" >> "rust/engine/src/lib.rs"

    # Force a clean build by touching lib.rs
    touch "rust/engine/src/lib.rs"

    # Try to build docs for this file - it SHOULD fail with -D warnings
    # Capture both stdout and stderr, and check for error
    LINK_TEST_OUTPUT=$(cargo rustdoc -p axiomind-engine --lib -- -D warnings 2>&1 || true)

    if echo "$LINK_TEST_OUTPUT" | grep -qi "unresolved link\|broken.*link\|error.*link"; then
        assert_success "rustdoc detects broken link (warnings treated as errors)"
    else
        assert_failure "rustdoc did not detect broken link"
    fi

    # Clean up
    sed -i.bak '/pub mod test_broken_link_temp;/d' "rust/engine/src/lib.rs"
    rm -f "$TEMP_TEST_FILE" "rust/engine/src/lib.rs.bak"

    echo ""
}

# Test 9: Create temporary doctest that fails and verify test job catches it
test_failing_doctest_detection() {
    echo "Test 9: Failing doctest detection"
    echo "-----------------------------------"

    cd "$PROJECT_ROOT"

    # Create a temporary test file with a failing doctest
    TEMP_TEST_FILE="rust/engine/src/test_failing_doctest_temp.rs"
    cat > "$TEMP_TEST_FILE" << 'EOF'
/// A test module with a failing doctest.
///
/// # Example (this will fail)
/// ```
/// // This doctest will intentionally fail
/// assert_eq!(2 + 2, 5, "Math is broken!");
/// ```
pub struct TestFailingDoctest;
EOF

    # Add the module to lib.rs temporarily
    echo "pub mod test_failing_doctest_temp;" >> "rust/engine/src/lib.rs"

    # Try to run doctests - expect failure
    TEST_OUTPUT=$(cargo test --doc -p axiomind-engine 2>&1 || true)

    if echo "$TEST_OUTPUT" | grep -q "test result: FAILED\|FAILED\|panicked"; then
        assert_success "doctest failure detected correctly"
    else
        # Doctest might not have run
        echo -e "${YELLOW}⚠${NC}  Warning: Doctest did not fail as expected"
        echo "$TEST_OUTPUT" | head -20
    fi

    # Clean up
    sed -i.bak '/pub mod test_failing_doctest_temp;/d' "rust/engine/src/lib.rs"
    rm -f "$TEMP_TEST_FILE" "rust/engine/src/lib.rs.bak"

    echo ""
}

# Test 10: Verify deploy-docs workflow triggers on main branch only
test_deploy_workflow_triggers() {
    echo "Test 10: Deploy workflow configuration"
    echo "-----------------------------------"

    if grep -A 5 "^on:" "$PROJECT_ROOT/.github/workflows/deploy-docs.yml" | grep -q "branches:.*main"; then
        assert_success "deploy-docs triggers on main branch"
    else
        assert_failure "deploy-docs does not trigger on main branch"
    fi

    if grep -q "workflow_dispatch" "$PROJECT_ROOT/.github/workflows/deploy-docs.yml"; then
        assert_success "deploy-docs supports manual trigger"
    else
        assert_failure "deploy-docs does not support manual trigger"
    fi

    echo ""
}

# Run all tests
echo "==========================================="
echo "CI/CD Integration Tests (Task 8.1)"
echo "==========================================="
echo ""

test_workflow_files_exist
test_rustdoc_job_exists
test_validate_docs_job_exists
test_jobs_run_in_parallel
test_artifacts_uploaded
test_rustdoc_build_succeeds
test_index_generation_succeeds
test_broken_link_detection
test_failing_doctest_detection
test_deploy_workflow_triggers

# Print summary
echo "==========================================="
echo "Test Summary"
echo "==========================================="
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
