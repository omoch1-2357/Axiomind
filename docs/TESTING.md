# Testing Strategy

This document outlines the comprehensive testing strategy for the Axiomind project.

## The Problem: Tests Pass But Code Doesn't Work

### Incident Report (2025-11-02)

**Issue**: rust-web-server had 178 passing tests (100%), but the application failed in a real browser.

**Root Causes**:
1. **JavaScript syntax error** in `game.js:66-69` (template literal misuse)
   - Mixing quote types inside template literals: `` `${condition ? value : '...'} ` ``
   - ESLint couldn't parse the file due to "Unterminated string constant" error
   - Existing `no-template-curly-in-string` rule was present but never ran due to parse failure
2. **htmx Content-Type mismatch** (`application/x-www-form-urlencoded` vs. `application/json`)
   - `hx-ext="json-enc"` declared in HTML but extension script not loaded
   - Rust API expected `application/json`, received form-encoded data
   - Resulted in 415 Unsupported Media Type errors
3. **Zero browser-based E2E tests**
   - Only Rust unit/integration tests existed (178 tests)
   - No validation of JavaScript execution, htmx integration, or DOM updates
   - Critical gap: backend tests cannot validate frontend behavior
4. **No JavaScript linting or static analysis enforcement**
   - ESLint configuration existed (`.eslintrc.json`)
   - Pre-commit hook existed (`.githooks/pre-commit`)
   - CI job existed (`.github/workflows/ci.yml` - `frontend-lint`)
   - **But**: Syntax error prevented ESLint from running, creating false sense of security

**Impact**: Complete UI failure despite passing all Rust tests. Users could not start games, submit forms, or interact with the application.

**Lesson**: Testing only the backend does NOT validate the frontend works. You MUST test in a real browser with E2E tests.

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
- ✅ Run `npm run lint` - ESLint must pass (zero errors)
- ✅ Run `node --check <file>` - Syntax validation (no parse errors)
- ✅ Run `npm run test:e2e` - Browser E2E must pass (all test cases)
- ✅ **Manual verification in browser** - Visual inspection
- ✅ Check browser console (F12) - No JavaScript errors
- ✅ Verify Network tab - Correct Content-Type headers (`application/json` for API calls)
- ✅ Test form submissions - Data sent in expected format (JSON, not form-encoded)

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
   - Playwright captures screenshots on test failure
   - Look for visual clues: missing elements, error messages, blank screens
2. **Check the trace logs** (`playwright-report/trace.zip`)
   - Open with `npx playwright show-trace trace.zip`
   - Provides step-by-step execution timeline with DOM snapshots
3. **Run in headed mode** for live debugging:
   ```bash
   npm run test:e2e:headed
   # Or run a specific test file
   npx playwright test tests/e2e/game-flow.spec.js --headed
   ```
4. **Check browser console errors**:
   - Tests automatically capture console messages via `page.on('console')`
   - Look for JavaScript runtime errors, syntax errors, or API failures
5. **Verify API responses in Network tab**:
   - Use Playwright's network interception:
     ```javascript
     page.on('request', request => console.log('>>', request.method(), request.url()));
     page.on('response', response => console.log('<<', response.status(), response.url()));
     ```
   - Check for 415 (Content-Type mismatch), 400 (Bad Request), or 500 (Server Error)
6. **Inspect request payloads**:
   ```javascript
   const request = await page.waitForRequest('/api/sessions');
   console.log('Headers:', request.headers());
   console.log('Body:', request.postData());
   ```
7. **Fix and re-run**:
   - Make minimal changes to fix the root cause
   - Re-run the specific test: `npx playwright test tests/e2e/<file>.spec.js`
   - Verify all tests pass: `npm run test:e2e`

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

## Continuous Quality Assurance Process

### Quality Gates (Must Pass Before Merge)

**For ALL Pull Requests**:
- [ ] ✅ All Rust tests pass (`cargo test --workspace`)
- [ ] ✅ Clippy passes with zero warnings (`cargo clippy -- -D warnings`)
- [ ] ✅ Code is formatted (`cargo fmt --check`)
- [ ] ✅ ESLint passes with zero errors (`npm run lint`)
- [ ] ✅ All E2E tests pass (`npm run test:e2e`)
- [ ] ✅ No console errors in browser (manual check)
- [ ] ✅ CI pipeline passes all jobs (7/7 jobs green)

**For Frontend Changes** (additional requirements):
- [ ] ✅ E2E test added for new feature (test-first development)
- [ ] ✅ Content-Type headers verified in E2E test
- [ ] ✅ API payload structure validated in E2E test
- [ ] ✅ Manual browser testing completed (all browsers: Chrome/Firefox/Safari)

**For Backend Changes** (additional requirements):
- [ ] ✅ Unit tests added for new logic
- [ ] ✅ Integration test added for new API endpoint
- [ ] ✅ API contract documented (request/response types)

### Code Review Checklist

**Reviewer Responsibilities**:
1. **Test Coverage Verification**
   - [ ] New features have corresponding tests
   - [ ] E2E tests cover critical user flows
   - [ ] Edge cases are tested (error scenarios, boundary conditions)
   - [ ] No untested code paths in diff

2. **E2E Test Quality**
   - [ ] Tests verify Content-Type headers (if API calls)
   - [ ] Tests check for JavaScript console errors
   - [ ] Tests validate DOM updates (not just HTTP responses)
   - [ ] Tests include meaningful assertions (not just "page loads")

3. **Documentation**
   - [ ] New features documented in relevant docs
   - [ ] API changes reflected in `docs/` or inline comments
   - [ ] Breaking changes clearly marked

4. **CI/CD**
   - [ ] All CI jobs passed (check GitHub Actions tab)
   - [ ] No flaky tests introduced (re-run if necessary)
   - [ ] Artifacts uploaded successfully (if applicable)

### Test-Driven Development Workflow

**For New Features** (follow this order):

```
1. Write E2E test (RED)
   ├─ Describe expected user behavior
   ├─ Run test (should FAIL)
   └─ Commit test file

2. Write unit/integration tests (RED)
   ├─ Test backend logic
   ├─ Run tests (should FAIL)
   └─ Commit test files

3. Implement feature (GREEN)
   ├─ Write minimal code to pass tests
   ├─ Run all tests (should PASS)
   └─ Commit implementation

4. Refactor (REFACTOR)
   ├─ Clean up code
   ├─ Run all tests (should still PASS)
   └─ Commit refactoring

5. Validate (VERIFY)
   ├─ Run full CI locally: cargo test && npm run lint && npm run test:e2e
   ├─ Manual browser test
   └─ Create pull request
```

**Benefits**:
- Tests document expected behavior
- Prevents over-engineering
- Catches regressions immediately
- Provides confidence for refactoring

### Test Gap Detection Process

**When to Add Tests**:
- [ ] New API endpoint added → Add integration test + E2E test
- [ ] New UI component added → Add E2E test
- [ ] Bug fixed → Add regression test (unit or E2E)
- [ ] Edge case discovered → Add test case

**How to Identify Gaps**:
1. **Code Review**: Reviewer checks for untested code
2. **Bug Reports**: If bug slipped through, test was missing
3. **Coverage Report**: `cargo tarpaulin` for Rust (target: 80%+)
4. **E2E Audit**: Manually verify all user flows have E2E tests

**Remediation**:
- Add missing test immediately (before new features)
- Document the gap in issue tracker
- Schedule test improvement in next sprint

### Continuous Improvement

**Weekly/Monthly Reviews**:
- [ ] Review test failure rate (target: <5% flaky tests)
- [ ] Audit E2E test coverage (all critical flows covered?)
- [ ] Check CI pipeline duration (target: <15 minutes)
- [ ] Review test maintenance burden (flaky tests, slow tests)

**Metrics to Track**:
- Test count (unit, integration, E2E)
- Test execution time
- Test failure rate (flaky vs. real failures)
- Bug escape rate (bugs found in production vs. caught by tests)

**Improvement Actions**:
- Remove or fix flaky tests
- Split slow E2E tests into focused tests
- Add tests for common bug patterns
- Update documentation when patterns emerge

## Testing Interactive CLI Commands

### Overview

Interactive commands that read from stdin require special testing approaches. This section documents patterns for testing blocking behavior, input parsing, and state changes in CLI commands.

### Pattern 1: Testing Blocking Behavior with Piped Stdin

**Purpose**: Verify that interactive commands actually wait for user input instead of immediately completing.

**Implementation**:
```rust
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn test_command_blocks_waiting_for_stdin() {
    let binary = find_axm_binary();

    // Spawn command with piped stdin but don't write to it
    let mut child = Command::new(&binary)
        .args(&["play", "--vs", "human", "--hands", "1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    // Command should block waiting for input
    let start = Instant::now();
    let timeout = Duration::from_millis(500);

    loop {
        if let Ok(Some(_status)) = child.try_wait() {
            panic!("Command completed unexpectedly (should block)");
        }

        if start.elapsed() >= timeout {
            // Success - command is blocking
            let _ = child.kill();
            let _ = child.wait();
            return;
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}
```

**Key Points**:
- Use `Stdio::piped()` to control stdin
- Use `try_wait()` to check if process is still running
- Don't write to stdin - test that command blocks
- Kill the process after verifying blocking behavior

### Pattern 2: Testing Input Parsing and State Changes

**Purpose**: Verify that user input is correctly parsed and affects command behavior.

**Implementation**:
```rust
use std::io::Write;

#[test]
fn test_command_parses_input_and_updates_state() {
    let binary = find_axm_binary();

    let mut child = Command::new(&binary)
        .args(&["play", "--vs", "human", "--hands", "1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"fold\n").expect("Failed to write");
        stdin.write_all(b"q\n").expect("Failed to write quit");
    }

    // Wait for completion
    let output = child.wait_with_output().expect("Failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify input was processed
    assert!(stdout.contains("Action: fold"),
            "Expected fold action in output");
    assert_eq!(output.status.code().unwrap_or(1), 0,
              "Expected successful exit");
}
```

**Key Points**:
- Take ownership of `stdin` to write input
- Write newline-terminated strings (`\n`)
- Verify output contains evidence of input processing
- Check exit code to ensure success

### Pattern 3: Testing Error Handling and Recovery

**Purpose**: Verify that invalid input triggers appropriate errors without crashing.

**Implementation**:
```rust
#[test]
fn test_command_handles_invalid_input_gracefully() {
    let binary = find_axm_binary();

    let mut child = Command::new(&binary)
        .args(&["play", "--vs", "human", "--hands", "1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    // Send invalid input followed by valid quit
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"invalid_action\n").expect("Failed to write");
        stdin.write_all(b"q\n").expect("Failed to write quit");
    }

    let output = child.wait_with_output().expect("Failed to wait");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify error message appears
    assert!(stderr.contains("Invalid") || stderr.contains("Unrecognized"),
            "Expected error message for invalid input");

    // Verify command recovers and completes successfully
    assert_eq!(output.status.code().unwrap_or(1), 0,
              "Command should recover from invalid input");
}
```

**Key Points**:
- Test invalid input scenarios
- Verify error messages go to stderr
- Ensure command doesn't crash or hang
- Confirm successful exit after recovery

### Pattern 4: Testing Warning Display

**Purpose**: Verify that placeholder implementations display appropriate warnings.

**Implementation**:
```rust
#[test]
fn test_placeholder_warning_displayed() {
    let binary = find_axm_binary();

    let output = Command::new(&binary)
        .args(&["eval", "--ai-a", "test", "--ai-b", "baseline", "--hands", "10"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify warning appears in stderr
    assert!(stderr.contains("WARNING:"),
            "Expected WARNING prefix in stderr");
    assert!(stderr.contains("placeholder") || stderr.contains("random"),
            "Expected warning about placeholder behavior");

    // Verify parameters unused warnings
    assert!(stderr.contains("ai-a") || stderr.contains("ai-b"),
            "Expected warning about unused parameters");
}
```

**Key Points**:
- Check stderr for warnings
- Verify "WARNING:" prefix for grep-ability
- Confirm placeholder behavior is clearly indicated
- Check that parameter warnings are present

### Pattern 5: Separating stdout and stderr in Tests

**Purpose**: Validate that data goes to stdout and diagnostics go to stderr.

**Implementation**:
```rust
#[test]
fn test_output_separation() {
    let binary = find_axm_binary();

    let output = Command::new(&binary)
        .args(&["stats", "--input", "test.jsonl"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify data output goes to stdout
    assert!(stdout.contains("Hands:") || stdout.contains("Win rate:"),
            "Expected data in stdout");

    // Verify warnings/errors go to stderr (if any)
    if !stderr.is_empty() {
        assert!(stderr.contains("WARNING:") || stderr.contains("Error"),
                "stderr should contain warnings or errors only");
    }
}
```

**Key Points**:
- Capture both stdout and stderr separately
- Verify data output is on stdout
- Verify diagnostics are on stderr
- Allow empty stderr for clean runs

### Helper Functions

**Finding the Binary**:
```rust
fn find_axm_binary() -> std::path::PathBuf {
    // Check CARGO_BIN_EXE_axm first (set by cargo test)
    if let Ok(explicit) = std::env::var("CARGO_BIN_EXE_axm") {
        return std::path::PathBuf::from(explicit);
    }

    // Fall back to searching target directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root");

    let executable = if cfg!(windows) { "axm.exe" } else { "axm" };

    // Search debug and release profiles
    for profile in ["debug", "release"] {
        let candidate = workspace_root
            .join("target")
            .join(profile)
            .join(executable);
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!("Could not find axm binary in target directory");
}
```

### Common Pitfalls

**Don't**: Use environment variable bypasses
```rust
// BAD - bypasses actual stdin reading
std::env::set_var("AXM_TEST_INPUT", "fold");
```

**Do**: Use real piped stdin
```rust
// GOOD - tests actual blocking behavior
let mut child = Command::new(binary)
    .stdin(Stdio::piped())
    .spawn()?;
child.stdin.as_mut().unwrap().write_all(b"fold\n")?;
```

**Don't**: Assume command will complete instantly
```rust
// BAD - race condition
let child = Command::new(binary).spawn()?;
let output = child.wait_with_output()?; // May hang!
```

**Do**: Write input before waiting
```rust
// GOOD - write input then wait
if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(b"q\n")?;
}
let output = child.wait_with_output()?;
```

### Test Organization

Place behavioral tests in `rust/cli/tests/test_behavioral.rs` following this structure:

```
rust/cli/tests/
├── test_behavioral.rs        # Interactive command tests
├── test_command_registry.rs  # Command synchronization tests
├── test_warning_system.rs    # Warning display tests
└── helpers/
    ├── mod.rs                # Helper functions
    └── cli_runner.rs         # Binary finding logic
```

### Checklist for Interactive Command Tests

When testing an interactive CLI command, verify:

- [ ] Command blocks waiting for stdin (doesn't complete immediately)
- [ ] Valid input is parsed correctly and affects behavior
- [ ] Invalid input displays error and re-prompts (doesn't crash)
- [ ] Quit commands ('q', 'quit') work from any state
- [ ] Empty input is handled gracefully
- [ ] Multiple invalid inputs don't cause crashes
- [ ] Output goes to stdout, warnings/errors to stderr
- [ ] Exit codes are correct (0 = success, 2 = error)
- [ ] Warnings displayed for placeholder implementations
- [ ] All command parameters are verified to be used

## References

- [Playwright Documentation](https://playwright.dev)
- [Testing Trophy (Kent C. Dodds)](https://kentcdodds.com/blog/the-testing-trophy-and-testing-classifications)
- [Test-Driven Development by Kent Beck](https://www.amazon.com/Test-Driven-Development-Kent-Beck/dp/0321146530)
- Incident: rust-web-server 2025-11-02 (this project)
- [std::process::Command Documentation](https://doc.rust-lang.org/std/process/struct.Command.html)
