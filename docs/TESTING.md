# Testing Strategy

This document outlines the comprehensive testing strategy for the Axiomind project.

## The Problem: Tests Pass But Code Doesn't Work

### Incident Report (2025-11-02)

**Issue**: rust-web-server had 178 passing tests (100%), but the application failed in a real browser.

**Root Causes**:
1. JavaScript syntax error in `game.js:66-69` (template literal misuse)
2. htmx Content-Type mismatch (`application/x-www-form-urlencoded` vs. `application/json`)
3. Zero browser-based E2E tests
4. No JavaScript linting or static analysis

**Impact**: Complete UI failure despite passing all Rust tests.

**Lesson**: Testing only the backend does NOT validate the frontend works.

---

## Testing Pyramid

```
         /\
        /E2\     Browser E2E (Playwright) - USER EXPERIENCE
       /----\
      / API  \   Integration (HTTP requests) - API CONTRACTS
     /--------\
    /   UNIT   \ Unit Tests (Rust) - LOGIC CORRECTNESS
   /------------\
```

### 1. Unit Tests (Rust)
- **Location**: Inline with modules (`#[cfg(test)]`)
- **Coverage**: Business logic, algorithms, data structures
- **Run**: `cargo test --workspace`
- **Target**: 80%+ code coverage

### 2. Integration Tests (HTTP API)
- **Location**: `rust/*/tests/`
- **Coverage**: HTTP endpoints, request/response validation
- **Run**: `cargo test --test integration`
- **Limitation**: ⚠️ Does NOT test JavaScript, browser behavior, or UI

### 3. Browser E2E Tests (Playwright)
- **Location**: `tests/e2e/`
- **Coverage**: Complete user flows in real browser
- **Run**: `npm run test:e2e`
- **CRITICAL**: This is the ONLY test that validates the app works for users

---

## Mandatory Test Requirements

### For Backend Changes (Rust)
- ✅ Run `cargo test --workspace` - All tests must pass
- ✅ Run `cargo clippy` - Zero warnings
- ✅ Run `cargo fmt --check` - Formatting correct

### For Frontend Changes (JavaScript/HTML/CSS)
- ✅ Run `npm run lint` - ESLint must pass
- ✅ Run `node --check <file>` - Syntax validation
- ✅ Run `npm run test:e2e` - Browser E2E must pass
- ✅ **Manual verification in browser** - Visual inspection

### For Full-Stack Features
- ✅ All of the above
- ✅ Test integration points (browser → htmx → server)
- ✅ Verify Content-Type headers
- ✅ Check console for JavaScript errors

---

## Test Commands

```bash
# Rust Tests
cargo test --workspace               # All Rust tests
cargo test -p axm_web                # Web server tests only
cargo test --test integration        # Integration tests only

# JavaScript Tests
npm run lint                         # ESLint
npm run lint:fix                     # Auto-fix linting issues
npm run test:e2e                     # Playwright E2E tests
npm run test:e2e:headed              # E2E with visible browser

# Combined
npm test                             # All frontend tests
cargo test --all && npm test         # Everything

# Pre-commit validation
./.githooks/pre-commit               # Runs before every commit
```

---

## CI/CD Pipeline

Every push triggers:

1. **Rust Check** - `cargo check`
2. **Rust Tests** - `cargo test` on Linux/macOS/Windows
3. **Rust Fmt** - `cargo fmt --check`
4. **Clippy** - `cargo clippy`
5. **JavaScript Lint** - `npm run lint` + syntax check
6. **Browser E2E** - Playwright tests in real browser

**All must pass before merging.**

---

## Writing E2E Tests

### Example: Game Start Flow

```javascript
// tests/e2e/game-flow.spec.js
import { test, expect } from '@playwright/test';

test('complete game flow', async ({ page }) => {
  // Start server (handled by test setup)
  await page.goto('http://localhost:8080');

  // Verify lobby loads
  await expect(page.locator('h2')).toContainText('Game Lobby');

  // Start game
  await page.click('button:has-text("START GAME")');

  // Verify game table appears
  await expect(page.locator('.poker-table')).toBeVisible();

  // Check player cards are displayed
  await expect(page.locator('.hole-cards')).toBeVisible();

  // Verify no JavaScript errors
  const errors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(msg.text());
  });

  // Take action
  await page.click('button:has-text("CHECK")');

  // Verify state updates
  await expect(page.locator('.pot-display')).toBeVisible();

  // No errors should occur
  expect(errors).toHaveLength(0);
});
```

---

## Common Testing Pitfalls

### ❌ Anti-Pattern: Testing Implementation, Not Behavior
```rust
// BAD: Mocking everything
let response = mock_http_client()
    .post("/api/sessions")
    .with_json(json!({"level": 1}))
    .send();
```

### ✅ Best Practice: Test Real Behavior
```javascript
// GOOD: Real browser, real clicks
await page.click('button:has-text("START GAME")');
await expect(page.locator('.poker-table')).toBeVisible();
```

### ❌ Anti-Pattern: Assuming Tests Cover Everything
```rust
// This passes, but doesn't test JavaScript:
assert_eq!(response.status(), 201);
```

### ✅ Best Practice: Test Integration Points
```javascript
// This catches Content-Type issues:
const response = await page.request.post('/api/sessions', {
  data: { level: 1, opponent_type: 'ai:baseline' }
});
expect(response.headers()['content-type']).toContain('application/json');
```

---

## When to Skip Tests

**Never skip**:
- Unit tests for business logic
- E2E tests for user-facing features
- Integration tests for API endpoints

**Can skip** (with justification):
- Performance tests in local development
- Cross-browser tests (stick to Chromium in CI)
- Exhaustive edge case coverage (focus on happy path + critical errors)

---

## Test Failure Protocol

### If Rust Tests Fail
1. Fix the issue
2. Add a test case for the bug
3. Verify fix with `cargo test`

### If E2E Tests Fail
1. **Check the screenshot** (`playwright-report/`)
2. Run headed mode: `npm run test:e2e:headed`
3. Check browser console for errors
4. Verify API responses in Network tab
5. Fix and re-run

### If Tests Pass But App Doesn't Work
**🚨 THIS IS A TEST GAP 🚨**

1. Write an E2E test that reproduces the issue
2. Verify the test fails
3. Fix the bug
4. Verify the test passes
5. Commit both the fix AND the test

---

## Coverage Goals

| Layer | Target | Measurement |
|-------|--------|-------------|
| Rust Unit | 80%+ | `cargo tarpaulin` |
| Rust Integration | 100% of endpoints | Manual audit |
| JavaScript | 70%+ | `nyc` (optional) |
| E2E Critical Flows | 100% | User journey checklist |

---

## References

- [Playwright Documentation](https://playwright.dev)
- [Testing Trophy (Kent C. Dodds)](https://kentcdodds.com/blog/the-testing-trophy-and-testing-classifications)
- Incident: rust-web-server 2025-11-02 (this project)
