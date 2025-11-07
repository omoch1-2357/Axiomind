# Validation Tests

This directory contains validation tests and checklists for end-to-end feature validation.

## Task 8.2: GitHub Pages Deployment Test

### Purpose
Validate that the rustdoc documentation is correctly deployed to GitHub Pages as part of the rustdoc-automation feature (Task 8.2).

### Files

- **`task-8.2-github-pages-checklist.md`**: Comprehensive manual validation checklist
- **`run-github-pages-test.sh`**: Automated test execution script
- **`_evidence/`**: Directory for storing test evidence (screenshots, logs, reports)

### Quick Start

#### Automated Testing

Run the automated test suite:

```bash
# From project root
bash tests/validation/run-github-pages-test.sh
```

This script will:
1. Verify workflow configuration
2. Check GitHub Pages accessibility
3. Validate index generation script
4. Run automated E2E tests
5. Generate a test report

#### Manual Testing

Follow the comprehensive checklist:

```bash
# Open the checklist
cat tests/validation/task-8.2-github-pages-checklist.md
```

The checklist covers:
- Workflow trigger verification
- gh-pages branch validation
- Documentation accessibility testing
- Navigation link testing
- Search functionality validation
- Cross-browser compatibility
- Performance metrics

### Requirements

- **Node.js**: >=18.0.0
- **npm**: >=9.0.0
- **Playwright**: Installed via `npm install`
- **bash**: For running shell scripts (Git Bash on Windows)

### Environment Variables

- `GITHUB_PAGES_URL`: Override the default GitHub Pages URL
  ```bash
  export GITHUB_PAGES_URL=https://your-org.github.io/your-repo/
  bash tests/validation/run-github-pages-test.sh
  ```

### Test Results

After running tests, check:

- **Automated results**: `tests/validation/_evidence/test-report.md`
- **E2E test logs**: `tests/validation/_evidence/e2e-test-output.log`
- **Manual checklist**: Fill out `task-8.2-github-pages-checklist.md`

### Evidence Collection

When performing manual validation:

1. Create `_evidence/` directory if it doesn't exist
2. Capture screenshots:
   - Workflow execution
   - gh-pages branch structure
   - Documentation pages
   - Navigation and search
3. Save evidence files with descriptive names:
   - `workflow-trigger.png`
   - `gh-pages-structure.png`
   - `pages-index.png`
   - `search-results.png`

### Integration with CI

These validation tests can be integrated into CI/CD pipelines:

```yaml
# .github/workflows/ci.yml (example)
- name: Validate GitHub Pages Deployment
  run: |
    export GITHUB_PAGES_URL=https://omoch1-2357.github.io/Axiomind/
    bash tests/validation/run-github-pages-test.sh
```

### Troubleshooting

#### Issue: "GitHub Pages URL is not accessible"

**Possible causes:**
- Deployment hasn't completed yet
- GitHub Pages is not enabled in repository settings
- URL is incorrect

**Solution:**
1. Check GitHub Actions → Deploy Documentation workflow status
2. Verify Settings → Pages → Source is set correctly
3. Wait a few minutes for deployment to propagate

#### Issue: "E2E tests fail with timeout"

**Possible causes:**
- GitHub Pages is slow to respond
- Network connectivity issues

**Solution:**
1. Increase timeout in `tests/e2e/github-pages-deploy.spec.js`
2. Check network connectivity
3. Try running tests with headed browser: `npm run test:e2e:headed`

#### Issue: "js-yaml module not found"

**Solution:**
```bash
npm install
```

### Related Documentation

- Main task specification: `.kiro/specs/rustdoc-automation/tasks.md`
- E2E test implementation: `tests/e2e/github-pages-deploy.spec.js`
- Deployment workflow: `.github/workflows/deploy-docs.yml`
- Index generation script: `scripts/generate-doc-index.sh`

---

**Last Updated**: 2025-11-07
**Maintained by**: Axiomind Development Team
