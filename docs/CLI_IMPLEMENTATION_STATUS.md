# CLI Implementation Status

**Last Updated**: 2025-11-11
**Purpose**: Track actual implementation status of all CLI commands to prevent user confusion

---

## ⚠️ Critical Warning

This document was created following the discovery that multiple CLI commands advertised in `docs/CLI.md` and help text are either non-existent or provide only stub/placeholder implementations.

**See**: `docs/incidents/2025-11-11-human-mode-stub-implementation.md` for the incident that triggered this audit.

---

## Status Legend

- ✅ **COMPLETE**: Fully implemented and tested
- ⚠️ **PARTIAL**: Exists but with significant limitations or stub behavior
- 🚧 **PLANNED**: Documented but not yet implemented
- ❌ **BROKEN**: Advertised but non-functional

---

## Command Status Matrix

| Command | Status | Functionality | Limitations | Action Required |
|---------|--------|---------------|-------------|-----------------|
| `play --vs ai` | ⚠️ PARTIAL | AI always "checks", no real gameplay | No engine integration, no actual AI decisions | **HIGH PRIORITY** - Implement or document as demo |
| `play --vs human` | ❌ BROKEN | Prompts for input but never reads stdin | Cannot actually play; loops through hands immediately | **CRITICAL** - Implement stdin reading or remove mode |
| `replay` | ⚠️ PARTIAL | Counts lines in JSONL file | No visual replay, `--speed` parameter unused | Document as line counter; implement full replay later |
| `sim` | ✅ COMPLETE | Full simulation with resume capability | None | - |
| `eval` | ⚠️ PARTIAL | Random coin flip instead of AI comparison | Accepts AI parameters but ignores them | **MEDIUM PRIORITY** - Implement or document as placeholder |
| `stats` | ✅ COMPLETE | Aggregates statistics from JSONL | None | - |
| `verify` | ✅ COMPLETE | Comprehensive game rule verification | None | - |
| `serve` | 🚧 PLANNED | **Command does not exist** | Listed in help but not in Commands enum | **CRITICAL** - Remove from docs or implement integration |
| `deal` | ✅ COMPLETE | Deals single hand, displays cards | None | - |
| `bench` | ✅ COMPLETE | Benchmarks hand evaluation | None | - |
| `rng` | ✅ COMPLETE | Tests RNG output | None | - |
| `cfg` | ✅ COMPLETE | Displays effective configuration | None | - |
| `doctor` | ✅ COMPLETE | Environment diagnostics | None | - |
| `export` | ✅ COMPLETE | Format conversion (CSV/JSON/SQLite) | None | - |
| `dataset` | ✅ COMPLETE | Train/val/test splitting | None | - |
| `train` | 🚧 PLANNED | **Command does not exist** | Listed in help but not in Commands enum | **CRITICAL** - Remove from docs until implemented |

---

## Detailed Analysis

### ❌ BROKEN: `play --vs human`

**File**: `rust/cli/src/lib.rs:1567-1579`

**Current Behavior**:
```rust
Vs::Human => {
    // prompt once; in tests, read from axiomind_TEST_INPUT
    let action = scripted.as_deref().unwrap_or("");
    if action.is_empty() {
        let _ = writeln!(out, "Enter action (check/call/bet/raise/fold/q): ");
    }
    // ← NO stdin reading here
}
played += 1;  // Immediately proceeds
```

**User Experience**:
```bash
$ axiomind play --vs human
Level: 1
Blinds: SB=50 BB=100
Hand 1
Enter action (check/call/bet/raise/fold/q):
Session hands=1
Hands played: 1 (completed)
# ← Exits immediately without waiting for input
```

**What's Missing**:
- `std::io::stdin().read_line()` call
- Input parsing (check/call/raise/fold/bet amounts)
- Action validation and application to game engine
- Game state progression and display

**Tests**:
- `test_play.rs::human_quick_quit_via_test_input()` uses `axiomind_TEST_INPUT` env var (bypasses real stdin)
- Test validates prompt appears, not that input is read

**Recommendation**: See incident report for full analysis and fix plan.

---

### ⚠️ PARTIAL: `play --vs ai`

**File**: `rust/cli/src/lib.rs:1576-1578`

**Current Behavior**:
```rust
Vs::Ai => {
    let _ = writeln!(out, "ai: check");
}
```

**Issues**:
- AI always "checks" regardless of game state
- No integration with game engine
- No actual poker gameplay occurs
- Just prints message and increments counter

**What's Missing**:
- AI decision-making logic
- Game engine integration
- Bet sizing, folding, raising logic
- Hand progression and showdown

**Documentation Claims**: "Play against AI or human opponent"

**Recommendation**:
1. Implement actual AI opponent with game engine integration
2. Or rename to `--demo` mode and document limitations
3. Connect to existing AI infrastructure (if available)

---

### ⚠️ PARTIAL: `eval`

**File**: `rust/cli/src/lib.rs:1852-1874`

**Current Behavior**:
```rust
Commands::Eval { ai_a, ai_b, hands, seed } => {
    // Ignores ai_a and ai_b parameters!
    for _ in 0..hands {
        if (rng.next_u32() & 1) == 0 {
            a_wins += 1;
        } else {
            b_wins += 1;
        }
    }
}
```

**Issues**:
- Accepts `--ai-a` and `--ai-b` parameters but completely ignores them
- Uses random coin flip instead of poker simulation
- Misleads users into thinking they're comparing actual AI strategies
- Warning only shows if both AI names are identical

**Documentation Claims**: "Evaluate AI policies head-to-head by running N simulated games"

**Test Coverage**: `test_eval.rs` validates output format, not actual AI logic

**Recommendation**:
1. **Option A**: Implement real AI-vs-AI poker simulation
2. **Option B**: Add prominent warning: `eval: This is a placeholder returning random results. Real AI comparison not yet implemented.`
3. **Option C**: Return error stating "AI evaluation not implemented; use sim command"

---

### ⚠️ PARTIAL: `replay`

**File**: `rust/cli/src/lib.rs:1586-1603`

**Current Behavior**:
```rust
Commands::Replay { input, speed } => {
    validate_speed(speed)?;  // Validates but never uses speed
    let count = content.lines().filter(|l| !l.trim().is_empty()).count();
    writeln!(out, "Replayed: {} hands", count);
}
```

**Issues**:
- Accepts `--speed` parameter but doesn't use it
- Only counts non-empty lines, no actual replay
- No visualization of cards, actions, or game progression
- Misleading output: "Replayed: N hands" (should be "Counted: N lines")

**Documentation Claims**: "Load and replay hand histories, optionally controlling playback speed"

**Recommendation**:
1. Remove `--speed` parameter until replay is implemented
2. Change output to `Counted: N hands in file` (more accurate)
3. Implement actual replay with:
   - Parse HandRecord from JSONL
   - Display game state, actions, and outcomes
   - Use `--speed` for pacing between actions

---

### 🚧 PLANNED: `serve`

**Status**: **COMMAND DOES NOT EXIST**

**Evidence**:
```bash
$ axiomind serve
error: unrecognized subcommand 'serve'

Did you mean?
    sim
```

**Where It's Advertised**:
- `rust/cli/src/lib.rs:1443` - COMMANDS list includes "serve"
- `docs/CLI.md:17` - Listed as `serve - ローカル UI サーバを起動`
- `rust/cli/tests/integration/cli_basic.rs:26` - Expected in help output

**Infrastructure Exists**:
- `rust/web/src/bin/server.rs` - Web server binary exists
- Not integrated into CLI Commands enum

**Workaround**:
```bash
# Use this instead:
cargo run -p axiomind_web --bin axiomind-web-server
```

**Recommendation**:
1. **URGENT**: Remove "serve" from COMMANDS list and docs/CLI.md
2. Add note to docs: "To start the web server, use: `cargo run -p axiomind_web --bin axiomind-web-server`"
3. Or implement proper integration:
   ```rust
   Commands::Serve { port } => {
       // Spawn axiomind-web-server binary
   }
   ```

---

### 🚧 PLANNED: `train`

**Status**: **COMMAND DOES NOT EXIST**

**Evidence**:
```bash
$ axiomind train
error: unrecognized subcommand 'train'
```

**Where It's Advertised**:
- `rust/cli/src/lib.rs:1443` - COMMANDS list includes "train"
- `docs/CLI.md:25` - Listed as `train - 学習を起動 (planned)`
- `CLAUDE.md:263` - Listed as "Launch training (planned)"

**Recommendation**:
1. **URGENT**: Remove "train" from COMMANDS list
2. Keep in documentation with clear "(planned)" marker
3. When implemented, add Commands::Train variant

---

## Test Coverage Gaps

### Commands Without Proper Behavioral Tests

1. **play --vs human**
   - Test uses `axiomind_TEST_INPUT` workaround
   - Doesn't verify stdin is read
   - Doesn't verify blocking behavior

2. **play --vs ai**
   - No test validates AI makes actual decisions
   - Only checks output contains "ai: check"

3. **eval**
   - No test validates AI parameters are used
   - Only checks output format, not actual comparison

4. **replay**
   - No test validates `--speed` parameter is used
   - Only checks line count is correct

### Recommended Test Improvements

```rust
// Example: Proper behavioral test for play --vs human
#[test]
fn play_human_actually_reads_stdin() {
    let mut cmd = Command::new("axiomind");
    cmd.args(&["play", "--vs", "human", "--hands", "1"]);

    let mut child = cmd.stdin(Stdio::piped()).spawn().unwrap();
    let stdin = child.stdin.as_mut().unwrap();

    // Verify command is waiting (doesn't exit immediately)
    thread::sleep(Duration::from_millis(100));
    assert!(child.try_wait().unwrap().is_none(), "Command exited without reading input");

    // Provide input
    writeln!(stdin, "fold").unwrap();

    // Now it should complete
    let status = child.wait().unwrap();
    assert!(status.success());
}
```

---

## Documentation Updates Required

### 1. Update `docs/CLI.md`

**Remove**:
- `serve` command (not implemented)
- `train` command (mark as "planned, not yet available")

**Add Implementation Status Column**:
```markdown
| Command | Status | Description |
|---------|--------|-------------|
| play --vs ai | ⚠️ Demo | AI always checks (placeholder) |
| play --vs human | ❌ Broken | Prompts but doesn't read input |
| eval | ⚠️ Random | Returns random results (placeholder) |
```

### 2. Update `CLAUDE.md`

**Add Warning**:
```markdown
## Known Issues
- `play --vs human`: Non-functional, see docs/incidents/
- `eval`: Returns random results, not actual AI comparison
- Several commands have placeholder implementations
- See docs/CLI_IMPLEMENTATION_STATUS.md for full list
```

### 3. Update Help Text

**File**: `rust/cli/src/lib.rs:1441-1444`

**Current**:
```rust
const COMMANDS: &[&str] = &[
    "play", "replay", "sim", "eval", "stats", "verify",
    "serve", "deal", "bench", "rng", "cfg", "doctor",
    "export", "dataset", "train",
];
```

**Recommended**:
```rust
const COMMANDS: &[&str] = &[
    "play", "replay", "sim", "eval", "stats", "verify",
    "deal", "bench", "rng", "cfg", "doctor",
    "export", "dataset",
    // Not yet integrated: "serve" (use cargo run -p axiomind_web)
    // Planned: "train"
];
```

---

## Completion Criteria for Fixing

### For Each Broken/Partial Command

- [ ] **Implementation**: Feature works as documented
- [ ] **Tests**: Behavioral tests verify actual functionality
- [ ] **Documentation**: Updated to reflect actual capabilities
- [ ] **Manual Testing**: Feature verified in real usage scenarios
- [ ] **Incident Review**: Root cause addressed in test strategy

### Definition of "Complete"

A command is considered complete when:
1. ✅ All documented features are implemented
2. ✅ Tests verify actual behavior (not just output)
3. ✅ Manual testing confirms user workflows work
4. ✅ No known limitations or placeholders
5. ✅ Documentation matches implementation

### Definition of "Partial"

A command is partial when:
1. Core functionality exists but with limitations
2. Placeholder/stub behavior present
3. Parameters accepted but not used
4. Output suggests completeness but behavior is minimal

### Definition of "Planned"

A command is planned when:
1. Documented for future implementation
2. Infrastructure may exist but not integrated
3. Command variant does not exist in Commands enum
4. Help text clearly marks as "planned" or "future"

---

## Incident Prevention

### Process Improvements

1. **New Command Checklist**:
   - [ ] Command enum variant exists
   - [ ] Implementation is complete (not stub)
   - [ ] Tests verify behavioral correctness
   - [ ] Manual testing completed
   - [ ] Documentation updated
   - [ ] Implementation status documented

2. **Test Requirements**:
   - Must verify actual behavior, not just output
   - Must test with real inputs (not just mocks)
   - Must validate state changes occur
   - Must check integration with subsystems

3. **Documentation Requirements**:
   - Implementation status must be tracked
   - Limitations must be clearly stated
   - Placeholders must be marked as such
   - Planned features must be clearly labeled

---

## References

- **Incident Report**: `docs/incidents/2025-11-11-human-mode-stub-implementation.md`
- **Testing Guidelines**: `docs/TESTING.md`
- **CLI Reference**: `docs/CLI.md`
- **Main Project Guide**: `CLAUDE.md`

---

_Documented: 2025-11-11_
_Author: Claude (AI Development Assistant)_
_Next Review: When new commands are added or existing commands are updated_
