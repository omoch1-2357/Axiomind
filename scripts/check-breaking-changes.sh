#!/bin/bash
# Breaking Change Detection Script
# Detects potential breaking changes in public APIs and warns about documentation updates
# Usage: ./scripts/check-breaking-changes.sh [base_branch]

set -euo pipefail

BASE_BRANCH="${1:-main}"
WARNINGS_FOUND=0

echo "=== Breaking Change Detection ==="
echo "Comparing against: $BASE_BRANCH"
echo ""

# Check if base branch exists
if ! git rev-parse --verify "$BASE_BRANCH" >/dev/null 2>&1; then
    echo "Warning: Base branch '$BASE_BRANCH' not found. Skipping breaking change detection."
    exit 0
fi

# Detect removed or modified public items in Rust code
detect_rust_breaking_changes() {
    echo "Checking Rust public API changes..."

    # Find all Rust files with public items
    RUST_FILES=$(git diff --name-only "$BASE_BRANCH"...HEAD | grep '\.rs$' || true)

    if [ -z "$RUST_FILES" ]; then
        echo "  No Rust files modified."
        return
    fi

    for file in $RUST_FILES; do
        # Skip if file doesn't exist in HEAD (deleted files)
        if [ ! -f "$file" ]; then
            continue
        fi

        # Check for removed pub items
        REMOVED_PUB=$(git diff "$BASE_BRANCH"...HEAD -- "$file" | grep '^-.*pub ' || true)
        if [ -n "$REMOVED_PUB" ]; then
            echo "  ⚠️  Potential breaking change in $file:"
            echo "$REMOVED_PUB" | head -5
            WARNINGS_FOUND=$((WARNINGS_FOUND + 1))
        fi

        # Check for modified function signatures
        MODIFIED_PUB_FN=$(git diff "$BASE_BRANCH"...HEAD -- "$file" | grep -E '^[-+].*pub (fn|struct|enum|trait|type)' || true)
        if [ -n "$MODIFIED_PUB_FN" ]; then
            echo "  ⚠️  Modified public API in $file:"
            echo "$MODIFIED_PUB_FN" | head -5
            WARNINGS_FOUND=$((WARNINGS_FOUND + 1))
        fi
    done
}

# Check for documentation updates in modified files
check_documentation_updates() {
    echo ""
    echo "Checking documentation updates..."

    RUST_FILES=$(git diff --name-only "$BASE_BRANCH"...HEAD | grep '\.rs$' || true)

    if [ -z "$RUST_FILES" ]; then
        echo "  No Rust files to check."
        return
    fi

    for file in $RUST_FILES; do
        if [ ! -f "$file" ]; then
            continue
        fi

        # Check if pub items changed but no doc comments added
        PUB_CHANGES=$(git diff "$BASE_BRANCH"...HEAD -- "$file" | grep -E '^[+-].*pub (fn|struct|enum|trait|type)' | wc -l)
        DOC_CHANGES=$(git diff "$BASE_BRANCH"...HEAD -- "$file" | grep -E '^[+-].*//(/|!)' | wc -l)

        if [ "$PUB_CHANGES" -gt 0 ] && [ "$DOC_CHANGES" -eq 0 ]; then
            echo "  ⚠️  $file: Public API modified but no documentation comments updated"
            WARNINGS_FOUND=$((WARNINGS_FOUND + 1))
        fi
    done
}

# Check for version changes that might indicate breaking changes
check_version_changes() {
    echo ""
    echo "Checking Cargo.toml version changes..."

    CARGO_DIFF=$(git diff "$BASE_BRANCH"...HEAD -- '**/Cargo.toml' | grep '^[+-]version = ' || true)

    if [ -n "$CARGO_DIFF" ]; then
        echo "  Version changes detected:"
        echo "$CARGO_DIFF"
        echo "  ⚠️  Ensure documentation reflects version-specific behavior"
        WARNINGS_FOUND=$((WARNINGS_FOUND + 1))
    else
        echo "  No version changes."
    fi
}

# Main execution
detect_rust_breaking_changes
check_documentation_updates
check_version_changes

echo ""
echo "=== Summary ==="
if [ "$WARNINGS_FOUND" -gt 0 ]; then
    echo "⚠️  Found $WARNINGS_FOUND potential issues requiring attention"
    echo ""
    echo "Action Required:"
    echo "1. Review all modified public APIs"
    echo "2. Update affected documentation comments (///, //!)"
    echo "3. Add migration notes for breaking changes"
    echo "4. Update RUNBOOK.md if procedures changed"
    echo "5. Run 'cargo test --doc' to verify code examples"
    echo ""
    echo "This is a warning only. CI will not fail."
else
    echo "✓ No breaking changes detected or all APIs properly documented"
fi

exit 0
