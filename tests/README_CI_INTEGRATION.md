# CI/CD Integration Test Suite

## Overview

This test suite validates the rustdoc automation CI/CD pipeline implementation for the Axiomind project. It ensures that documentation generation, validation, and deployment workflows function correctly.

## Test File

**Location**: `tests/test_ci_integration.sh`

## Test Coverage

The test suite includes 21 comprehensive tests across 10 categories:

### 1. Workflow File Existence (2 tests)
- Verifies `ci.yml` exists
- Verifies `deploy-docs.yml` exists

### 2. Rustdoc Job Configuration (3 tests)
- Confirms rustdoc job is defined in ci.yml
- Validates `cargo doc --workspace --no-deps` command
- Checks index.html generation step

### 3. Documentation Validation Job (2 tests)
- Confirms validate-docs job is defined
- Validates broken link detection with `-D warnings`

### 4. Parallel Job Execution (2 tests)
- Ensures rustdoc and validate-docs jobs run independently
- Confirms no dependency chain between the two jobs

### 5. Artifact Management (2 tests)
- Verifies artifact upload in rustdoc job
- Confirms artifact is named "documentation"

### 6. Local Documentation Build (4 tests)
- Tests `cargo doc` builds successfully
- Validates axm_engine documentation generation
- Validates axm_cli documentation generation
- Validates axm_web documentation generation

### 7. Index Generation (2 tests)
- Tests index.html generation script execution
- Confirms index.html file creation

### 8. Broken Link Detection (1 test)
- Creates temporary module with broken documentation link
- Verifies rustdoc detects and fails on broken links with `-D warnings`

### 9. Doctest Validation (1 test)
- Creates temporary module with failing doctest
- Confirms test job detects doctest failures

### 10. Deployment Configuration (2 tests)
- Verifies deploy-docs triggers on main branch
- Confirms manual workflow_dispatch support

## Running the Tests

### Quick Run
```bash
# From project root
bash tests/test_ci_integration.sh
```

### Expected Output
```
===========================================
CI/CD Integration Tests (Task 8.1)
===========================================

Test 1: Workflow files exist
-----------------------------------
✓ ci.yml exists
✓ deploy-docs.yml exists

... (21 tests) ...

===========================================
Test Summary
===========================================
Passed: 21
Failed: 0

All tests passed!
```

## Test Implementation Details

### Broken Link Detection
The test creates a temporary Rust module with an intentionally broken documentation link:
```rust
/// This link is definitely broken: [`super::super::ThisTypeDoesNotExist123456`]
pub struct TestBrokenLink;
```

The test verifies that `cargo rustdoc -- -D warnings` catches this error.

### Failing Doctest Detection
The test creates a temporary module with a failing doctest:
```rust
/// ```
/// assert_eq!(2 + 2, 5, "Math is broken!");
/// ```
pub struct TestFailingDoctest;
```

The test confirms that `cargo test --doc` detects and reports the failure.

### Cleanup
All temporary files are automatically cleaned up after each test:
- `rust/engine/src/test_broken_link_temp.rs`
- `rust/engine/src/test_failing_doctest_temp.rs`
- Temporary modifications to `rust/engine/src/lib.rs`

## Integration with CI

This test suite validates the behavior of:
- `.github/workflows/ci.yml` (rustdoc and validate-docs jobs)
- `.github/workflows/deploy-docs.yml` (deployment to GitHub Pages)
- `scripts/generate-doc-index.sh` (index.html generation)

## Requirements Validation

This test suite validates all requirements from the rustdoc-automation specification:
- **Requirement 2**: Rustdoc build automation
- **Requirement 3**: GitHub Pages deployment configuration
- **Requirement 4**: Documentation quality validation
- **Requirement 5**: Local documentation generation

## Exit Codes

- `0`: All tests passed
- `1`: One or more tests failed

## Troubleshooting

### Test Hangs on Broken Link Detection
- Ensure Cargo cache is clean: `cargo clean`
- Run the test in isolation to verify functionality

### Artifact Upload Not Detected
- Check the awk pattern matches the job boundary
- Verify `validate-docs:` job name hasn't changed

### Doctest Detection Fails
- Ensure temporary files are properly cleaned up
- Check that `rust/engine/src/lib.rs` doesn't have leftover module declarations

## Future Enhancements

Potential improvements for this test suite:
1. Add timeout guards for long-running cargo commands
2. Implement parallel test execution for faster runs
3. Add performance benchmarking for documentation build times
4. Include HTML validation for generated index.html
