# Incident Report: Human Mode Stub Implementation (2025-11-11)

## Executive Summary

**Severity**: Critical
**Impact**: Core user-facing feature (`axm play --vs human`) non-functional
**Root Cause**: Test-driven development process focused on superficial validation rather than behavioral verification
**Status**: Identified, documented, awaiting fix

---

## Incident Description

### What Happened

The `axm play --vs human` command, which should allow interactive poker gameplay against an AI opponent, immediately exits after displaying a single prompt without waiting for or processing any user input.

**Expected Behavior**:
1. Display game state (blinds, cards, pot)
2. Prompt user for action
3. **Wait for stdin input**
4. Parse and validate action
5. Apply action to game engine
6. Continue game until completion

**Actual Behavior**:
1. Display game state ✓
2. Prompt user for action ✓
3. **Immediately proceed to next hand** ✗
4. Report all hands as "completed" despite no gameplay ✗

### User Impact

- **100% of interactive CLI gameplay is non-functional**
- Users cannot test AI opponents interactively via command line
- The feature appears to work (no error messages) but does nothing
- Silent failure mode - users may think they're doing something wrong

---

## Technical Analysis

### Root Cause: Incomplete Implementation

**File**: `rust/cli/src/lib.rs:1566-1580`

```rust
Vs::Human => {
    // prompt once; in tests, read from AXM_TEST_INPUT
    let action = scripted.as_deref().unwrap_or("");
    if action.is_empty() {
        let _ = writeln!(out, "Enter action (check/call/bet/raise/fold/q): ");
    }
    // ← NO stdin reading logic here
}
played += 1;  // Immediately continues to next iteration
```

**Missing Implementation**:
- No `std::io::stdin().read_line()` call
- No input parsing logic
- No action application to game engine
- No game state progression

This is a **stub implementation** that was never completed.

---

## Why Tests Didn't Catch This

### Test Philosophy Failure

The `comprehensive-cli-testing` spec (178 tests, all passing) validated:

✅ **What was tested**:
- Exit codes are zero
- Output contains expected strings ("Hand 1", "completed")
- Files are created
- Non-TTY environments are blocked

❌ **What was NOT tested**:
- **Stdin is actually read**
- **Command waits for user input**
- **Game engine processes actions**
- **Game state changes occur**
- **Interactive loop functions**

### Specific Test Example

```rust
// test_play.rs
#[test]
fn human_quick_quit_via_test_input() {
    env::set_var("AXM_TEST_INPUT", "q\n");  // Bypasses real stdin
    let result = run_cli(&["play", "--vs", "human", "--hands", "1"]);

    assert!(result.stdout.contains("Hand 1"));       // ✓ Passes
    assert!(result.stdout.contains("completed"));    // ✓ Passes
    assert_eq!(result.exit_code, 0);                 // ✓ Passes

    // But never checks:
    // - Was stdin polled?          ✗
    // - Did game execute?          ✗
    // - Were actions processed?    ✗
}
```

The test uses `AXM_TEST_INPUT` environment variable as a "backdoor" that bypasses the real stdin code path entirely.

### Requirements Specification Gap

**From** `.kiro/specs/comprehensive-cli-testing/requirements.md`:

**Requirement that EXISTS**:
> "WHEN `--vs human` is specified in non-TTY environment THEN the system SHALL warn and refuse execution"

**Requirement that DOES NOT EXIST**:
> "WHEN `--vs human` is specified in TTY environment THEN the system SHALL read and process user input"

**Only error cases were defined. Success cases were implicit and untested.**

---

## Process Failures

### 1. Requirements Definition Incompleteness
- **Problem**: Only error cases specified
- **Missing**: Positive behavioral requirements
- **Result**: No test obligation for core functionality

### 2. Test Strategy Superficiality
- **Problem**: Tested "what it prints" not "what it does"
- **Missing**: Behavioral verification (blocking, state changes, interactions)
- **Result**: Stub implementations pass all tests

### 3. Lack of Behavioral Assertions
- **Problem**: Black-box I/O testing only
- **Missing**: Introspection of runtime behavior
- **Result**: Cannot detect "does nothing" vs "does something"

### 4. No Manual Verification in Definition of Done
- **Problem**: Spec marked complete when tests pass
- **Missing**: "Actually use the feature" step
- **Result**: Nobody discovered the feature doesn't work

### 5. E2E Test Scope Limited to Web
- **Problem**: Comprehensive E2E tests exist for Web UI (Playwright)
- **Missing**: Equivalent interactive testing for CLI
- **Result**: Web UI quality is high, CLI quality is low

---

## Comparison: Web UI vs CLI Testing

| Aspect | Web UI Testing | CLI Testing |
|--------|----------------|-------------|
| **Actual behavior** | ✅ Browser E2E with Playwright | ❌ Output string checking only |
| **User interaction** | ✅ Real clicks, inputs, form submissions | ❌ Environment variable substitution |
| **Integration** | ✅ Full stack (htmx → server → response) | ❌ Isolated command execution |
| **Failure detection** | ✅ JavaScript runtime errors caught | ❌ Stub implementations pass |
| **Quality outcome** | ✅ All features functional | ❌ Core feature non-functional |

**Key difference**: Web UI tests verify the feature works in a real browser. CLI tests verify the command exits with code 0.

---

## Timeline

- **2025-11-02**: comprehensive-cli-testing spec completed, all 178 tests passing
- **2025-11-11**: User reports `play --vs human` doesn't work
- **2025-11-11**: Investigation reveals stub implementation, never completed
- **2025-11-11**: Incident documented

**Duration undetected**: ~9 days (though likely existed much longer)

---

## Lessons Learned

### 1. Test Count ≠ Test Quality
- 178 passing tests provided false confidence
- Quantity of assertions does not equal quality of verification

### 2. Always Define Success Cases
```diff
❌ "Non-TTY environment must error"
✅ "TTY environment must read stdin, parse input, execute actions, and progress game state"
```

### 3. Behavioral Verification is Essential
```diff
❌ assert!(output.contains("Enter action"));
✅ assert!(command_is_blocking_on_stdin());
✅ assert!(stdin_was_read());
✅ assert!(game_state_changed());
```

### 4. Manual Testing is Part of Definition of Done
```markdown
Completion criteria:
- [x] All automated tests pass
- [x] Zero compiler warnings
- [x] Documentation updated
+ [ ] Feature manually tested in target environment
+ [ ] Core user workflows verified end-to-end
```

### 5. E2E Tests Should Match Interface Type
- **Web UI** → Browser automation (Playwright) ✓
- **CLI interactive** → PTY simulation or real terminal testing ✗ (missing)
- **CLI batch** → Command execution and output checking ✓

---

## Recommended Actions

### Immediate (Fix the Bug)
1. Implement stdin reading loop in `rust/cli/src/lib.rs`
2. Add input parsing and validation
3. Integrate with game engine action processing
4. Add proper game state progression

### Short-term (Prevent Recurrence)
1. Add PTY-based integration tests for interactive CLI features
2. Update comprehensive-cli-testing spec with positive behavioral requirements
3. Add manual testing checklist to all spec completion criteria
4. Create "behavioral assertion" test helpers

### Long-term (Process Improvement)
1. Establish "Definition of Done" that includes manual verification
2. Require behavioral tests for all interactive features
3. Expand E2E testing philosophy from Web to all interfaces
4. Implement test quality reviews, not just test coverage reviews

---

## Related Documents

- `docs/TESTING.md` - Testing strategy and guidelines
- `docs/CLI.md` - CLI command reference
- `.kiro/specs/comprehensive-cli-testing/` - Test specification
- `rust/cli/tests/test_play.rs` - Existing play command tests

---

## Conclusion

This incident reveals a fundamental flaw in the test-driven development process: **optimizing for "tests passing" rather than "features working"**.

The comprehensive-cli-testing spec achieved 100% of its stated goals (178 tests, zero warnings, all assertions passing) while delivering 0% of the user-facing functionality for interactive play.

**The process succeeded, but the product failed.**

This is a critical learning moment. Moving forward, all feature specifications must include:
1. Positive behavioral requirements (not just error cases)
2. Tests that verify actual functionality (not just output strings)
3. Manual verification as part of completion criteria
4. E2E testing appropriate to the interface type

---

## Post-Incident Audit

Following this incident, a comprehensive audit of ALL CLI commands was conducted (2025-11-11).

### Additional Issues Discovered

**Critical (User-Facing Failures)**:
- `serve` command: Advertised in help and docs but **does not exist**
- `train` command: Advertised in help and docs but **does not exist**

**Significant (Misleading Functionality)**:
- `play --vs ai`: AI always "checks", no real gameplay
- `eval`: Uses random coin flip instead of actual AI comparison
- `replay`: Counts lines instead of replaying hands; `--speed` parameter unused

**Summary**: 6 out of 16 commands have significant implementation issues:
- 2 commands completely missing (12.5%)
- 4 commands with stub/placeholder implementations (25%)
- **Total: 37.5% of advertised CLI surface area non-functional or misleading**

### Full Audit Report

See `docs/CLI_IMPLEMENTATION_STATUS.md` for:
- Complete status matrix of all commands
- Detailed analysis of each issue
- Recommended fixes and priorities
- Updated documentation requirements
- Test improvement recommendations

### Key Finding

This was **not an isolated incident**. The same test philosophy failures allowed multiple stub implementations and non-existent commands to be marked as "complete" and shipped to users.

**Root cause remains**: Optimizing for "tests passing" rather than "features working".

---

_Documented: 2025-11-11_
_Author: Claude (AI Development Assistant)_
_Status: Open - Awaiting implementation of fixes_
_Related: docs/CLI_IMPLEMENTATION_STATUS.md_
