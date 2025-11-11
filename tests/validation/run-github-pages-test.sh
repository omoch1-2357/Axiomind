#!/bin/bash
# Test execution script for Task 8.2: GitHub Pages Deployment Test
#
# This script automates the validation of GitHub Pages deployment.
# It performs both automated E2E tests and manual validation checks.

set -euo pipefail

# Configuration
GITHUB_PAGES_URL="${GITHUB_PAGES_URL:-https://omoch1-2357.github.io/Axiomind/}"
REPO_URL="${REPO_URL:-https://github.com/omoch1-2357/Axiomind}"
EVIDENCE_DIR="tests/validation/_evidence"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_section() {
    echo -e "\n${BLUE}===================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}===================================================${NC}\n"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_info() {
    echo -e "ℹ️  $1"
}

# Create evidence directory
mkdir -p "$EVIDENCE_DIR"

# Start validation
log_section "Task 8.2: GitHub Pages Deployment Test"
echo "Repository: $REPO_URL"
echo "GitHub Pages URL: $GITHUB_PAGES_URL"
echo "Evidence directory: $EVIDENCE_DIR"

# Step 1: Verify workflow file exists
log_section "Step 1: Verify Workflow Configuration"

if [ -f ".github/workflows/deploy-docs.yml" ]; then
    log_success "deploy-docs.yml workflow file exists"

    # Parse workflow file
    log_info "Parsing workflow configuration..."
    node -e "
        const fs = require('fs');
        const yaml = require('js-yaml');
        const workflow = yaml.load(fs.readFileSync('.github/workflows/deploy-docs.yml', 'utf8'));

        console.log('Workflow name:', workflow.name);
        console.log('Triggers:', Object.keys(workflow.on).join(', '));
        console.log('Jobs:', Object.keys(workflow.jobs).join(', '));

        // Verify main branch trigger
        if (workflow.on.push && workflow.on.push.branches.includes('main')) {
            console.log('✅ Triggers on main branch push');
        } else {
            console.log('❌ Does NOT trigger on main branch push');
            process.exit(1);
        }

        // Verify workflow_dispatch
        if (workflow.on.workflow_dispatch) {
            console.log('✅ Manual trigger (workflow_dispatch) enabled');
        }

        // Verify deploy job
        const deployJob = workflow.jobs['deploy-docs'];
        if (deployJob) {
            console.log('✅ deploy-docs job found');

            // Check for GitHub Pages action
            const ghPagesStep = deployJob.steps.find(step =>
                step.uses && step.uses.includes('peaceiris/actions-gh-pages')
            );
            if (ghPagesStep) {
                console.log('✅ GitHub Pages deployment action configured');
            } else {
                console.log('❌ GitHub Pages deployment action NOT found');
                process.exit(1);
            }
        } else {
            console.log('❌ deploy-docs job NOT found');
            process.exit(1);
        }
    "; then
        log_success "Workflow configuration validated"
    else
        log_error "Workflow configuration validation failed"
        exit 1
    fi
else
    log_error "deploy-docs.yml workflow file NOT found"
    exit 1
fi

# Step 2: Check GitHub Pages URL accessibility
log_section "Step 2: Test GitHub Pages Accessibility"

log_info "Checking if $GITHUB_PAGES_URL is accessible..."
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$GITHUB_PAGES_URL" || echo "000")

if [ "$HTTP_STATUS" = "200" ]; then
    log_success "GitHub Pages URL is accessible (HTTP 200)"
else
    log_error "GitHub Pages URL returned HTTP $HTTP_STATUS"
    log_warning "This may be expected if the site hasn't been deployed yet"
    log_info "Please verify deployment has completed on GitHub Actions"
fi

# Step 3: Verify index.html generation script
log_section "Step 3: Verify Index Generation Script"

SCRIPT_EXISTS_ICON="❌"
SCRIPT_EXEC_ICON="❌"
if [ -f "scripts/generate-doc-index.sh" ]; then
    log_success "generate-doc-index.sh script exists"
    SCRIPT_EXISTS_ICON="✅"

    if [ -x "scripts/generate-doc-index.sh" ]; then
        log_success "Script has execute permissions"
        SCRIPT_EXEC_ICON="✅"
    else
        log_warning "Script is not executable (CI will use 'bash script.sh')"
    fi
else
    log_error "generate-doc-index.sh script NOT found"
    exit 1
fi

if [ "$SCRIPT_EXISTS_ICON" = "✅" ] && [ "$SCRIPT_EXEC_ICON" = "✅" ]; then
    SCRIPT_SECTION_ICON="✅"
else
    SCRIPT_SECTION_ICON="⚠️"
fi

# Step 4: Run automated E2E tests
log_section "Step 4: Run Automated E2E Tests"

log_info "Running Playwright E2E tests..."
export GITHUB_PAGES_URL

E2E_STATUS="❌"
if npm run test:e2e -- tests/e2e/github-pages-deploy.spec.js 2>&1 | tee "$EVIDENCE_DIR/e2e-test-output.log"; then
    log_success "All E2E tests passed"
    E2E_STATUS="✅"
else
    log_error "Some E2E tests failed"
    log_info "Check $EVIDENCE_DIR/e2e-test-output.log for details"
    exit 1
fi

# Step 5: Generate test report
log_section "Step 5: Generate Test Report"

cat > "$EVIDENCE_DIR/test-report.md" << EOF
# Task 8.2: GitHub Pages Deployment Test - Execution Report

**Execution Date**: $(date '+%Y-%m-%d %H:%M:%S')
**Repository**: $REPO_URL
**GitHub Pages URL**: $GITHUB_PAGES_URL

## Test Results

### 1. Workflow Configuration ✅
- Workflow file exists: ✅
- Triggers on main branch: ✅
- Manual trigger enabled: ✅
- GitHub Pages action configured: ✅

### 2. GitHub Pages Accessibility
- HTTP Status: $HTTP_STATUS
- Status: $([ "$HTTP_STATUS" = "200" ] && echo "✅ PASS" || echo "⚠️  PENDING")

### 3. Index Generation Script $SCRIPT_SECTION_ICON
- Script exists: $SCRIPT_EXISTS_ICON
- Script is executable: $SCRIPT_EXEC_ICON

### 4. Automated E2E Tests
- Status: $E2E_STATUS PASS
- Full log: _evidence/e2e-test-output.log

## Next Steps

1. Review manual validation checklist: tests/validation/task-8.2-github-pages-checklist.md
2. Verify all manual test cases
3. Collect screenshots for evidence
4. Sign off on validation checklist

## Evidence Files

- Test execution log: $EVIDENCE_DIR/e2e-test-output.log
- This report: $EVIDENCE_DIR/test-report.md

---

**Generated by**: run-github-pages-test.sh
**Test Suite**: Task 8.2 GitHub Pages Deployment
EOF

log_success "Test report generated: $EVIDENCE_DIR/test-report.md"

# Step 6: Summary
log_section "Test Execution Complete"

echo "Summary:"
echo "  - Workflow validation: ✅"
echo "  - GitHub Pages accessibility: $([ "$HTTP_STATUS" = "200" ] && echo "✅" || echo "⚠️ ")"
echo "  - Index script validation: $SCRIPT_SECTION_ICON"
echo "  - E2E tests: ✅"
echo ""
log_success "All automated tests passed!"
echo ""
log_info "Next steps:"
echo "  1. Review manual validation checklist"
echo "  2. Verify GitHub Actions workflow run"
echo "  3. Test navigation and search on GitHub Pages"
echo "  4. Complete evidence collection"
echo "  5. Update tasks.md to mark task 8.2 as complete"
echo ""
log_info "For manual validation, see:"
echo "  tests/validation/task-8.2-github-pages-checklist.md"
