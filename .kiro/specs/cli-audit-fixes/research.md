# Research & Design Decisions: CLI Audit Fixes

---
**Purpose**: Capture discovery findings, architectural investigations, and rationale that inform the technical design.

**Usage**:
- Log research activities and outcomes during the discovery phase.
- Document design decision trade-offs that are too detailed for `design.md`.
- Provide references and evidence for future audits or reuse.
---

## Summary
- **Feature**: `cli-audit-fixes`
- **Discovery Scope**: Extension (modifying existing CLI commands and test infrastructure)
- **Key Findings**:
  - Current implementation has blocking stdin issues in human play mode
  - Multiple commands have placeholder implementations without proper warnings
  - Testing infrastructure needs behavioral validation, not just output format checks
  - Game engine provides complete API but CLI doesn't integrate properly

## Research Log

### Rust stdin Reading and Blocking Behavior

- **Context**: The `play --vs human` command displays a prompt but never actually reads stdin, causing immediate termination instead of waiting for user input.
- **Sources Consulted**:
  - Stack Overflow: "How can I read non-blocking from stdin?" (Rust patterns)
  - Rust CLI Book: Testing chapter
  - Rust std::io documentation
- **Findings**:
  - Standard `std::io::stdin().read_line()` provides blocking behavior by default
  - For interactive CLI applications, blocking stdin is appropriate and expected
  - The current code prompts but never calls any stdin reading function
  - Test mode uses `AXM_TEST_INPUT` environment variable bypass which masked the bug
- **Implications**:
  - Implementation needs `BufRead::read_line()` or `Stdin::lines()` iterator
  - Must handle EOF, invalid input, and quit commands
  - Blocking behavior is correct for interactive mode; no async needed

### CLI Testing Strategies for Interactive Input

- **Context**: Current tests use environment variable workaround (`AXM_TEST_INPUT`) which doesn't validate actual stdin reading behavior.
- **Sources Consulted**:
  - Rust CLI Book: https://rust-cli.github.io/book/tutorial/testing.html
  - assert_cmd documentation
  - rexpect library for interactive testing
  - Stack Overflow: "How can I test stdin and stdout?"
- **Findings**:
  - **Dependency Injection Pattern**: Pass generic `impl BufRead` and `impl Write` to functions for unit testing
  - **assert_cmd**: Good for non-interactive command testing, limited for stdin interaction
  - **rexpect**: Recommended for true interactive testing with expect-like patterns
  - **Integration Testing**: Use `Command::spawn()` with `Stdio::piped()` to verify blocking behavior
- **Implications**:
  - Short-term: Add integration tests with piped stdin to verify blocking behavior
  - Long-term: Consider rexpect for comprehensive interactive testing
  - Current `AXM_TEST_INPUT` approach should remain for quick smoke tests
  - Need behavioral tests that verify: (1) command waits for input, (2) input is parsed, (3) actions affect game state

### Game Engine Integration Points

- **Context**: Understanding how to properly integrate CLI input with the existing game engine.
- **Sources Consulted**:
  - `rust/engine/src/engine.rs` - Engine struct and methods
  - `rust/engine/src/player.rs` - PlayerAction enum and validation
  - `rust/engine/src/rules.rs` - Action validation logic
- **Findings**:
  - Engine provides `Engine::deal_hand()` for dealing cards
  - Engine exposes `Engine::players()` and `Engine::board()` for state queries
  - `PlayerAction` enum defines: Fold, Check, Call, Bet(u32), Raise(u32), AllIn
  - Engine is designed for batch processing, not step-by-step interactive gameplay
  - **Gap**: No public API for progressive betting rounds (preflop → flop → turn → river)
  - Current `sim` command generates complete hands without interaction
- **Implications**:
  - **Phase 1 Fix (Human Play)**: Must build game loop around engine's current API
  - Need to track betting round state externally (preflop/flop/turn/river)
  - Need pot calculation and action validation logic in CLI layer
  - May require extending engine API for true interactive play (future enhancement)
  - For now, focus on parsing user input and validating against game rules

### Command Implementation Status Pattern

- **Context**: Multiple commands listed in help text don't exist or are placeholders.
- **Sources Consulted**:
  - `docs/CLI_IMPLEMENTATION_STATUS.md` - Comprehensive audit results
  - `rust/cli/src/lib.rs:1441-1444` - COMMANDS constant array
- **Findings**:
  - COMMANDS array includes "serve" and "train" which don't exist in Commands enum
  - Help text generation uses COMMANDS array, creating false advertising
  - Several commands accept parameters they ignore (eval's --ai-a/--ai-b, replay's --speed)
  - No consistent pattern for marking placeholder implementations
- **Implications**:
  - COMMANDS array must be synchronized with actual Commands enum variants
  - Placeholder implementations need prominent runtime warnings
  - Documentation must distinguish between "working", "partial", and "planned"
  - CI/tests should verify COMMANDS array matches enum variants

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Direct Fix (Phase 1) | Add stdin reading loops to existing command handlers | Minimal changes, quick to implement | Doesn't solve engine integration gap | Recommended for immediate fix |
| Engine Extension | Extend game engine with progressive betting API | Clean separation, proper game state | Requires engine API changes, larger scope | Future enhancement, out of scope for audit fixes |
| CLI Game Loop | Build complete game state machine in CLI layer | Full control, can ship quickly | Duplicates some engine logic, technical debt | Acceptable for Phase 2 (optional) |
| Hybrid Approach | Fix critical issues now, plan refactor later | Pragmatic, unblocks users | Need clear migration path | Selected approach (phased implementation) |

## Design Decisions

### Decision: Phased Implementation Strategy

- **Context**: Requirements span from simple documentation fixes to complex interactive gameplay implementation. Audit identified 10 distinct issues across multiple commands.
- **Alternatives Considered**:
  1. Full Implementation - Fix everything at once (serve, train, human play, AI mode, eval, replay)
  2. Critical Only - Fix only broken `play --vs human`, mark others as known issues
  3. Phased Approach - Prioritize by impact and complexity
- **Selected Approach**: Phased implementation aligned with gap analysis recommendations
  - **Phase 1**: Quick wins (warnings, documentation, remove non-existent commands)
  - **Phase 2**: Fix human play mode (critical, blocks testing)
  - **Phase 3**: AI implementation (optional, depends on AI infrastructure availability)
  - **Phase 4**: Enhanced testing infrastructure (prevents future regressions)
- **Rationale**:
  - Phase 1 provides immediate value with minimal risk
  - Phase 2 unblocks manual testing workflows (high priority for researchers)
  - Phase 3 and 4 can be deferred based on resource availability
  - Allows incremental delivery and early user feedback
- **Trade-offs**:
  - **Benefits**: Reduced risk, faster initial delivery, flexible scope
  - **Compromises**: Multiple PRs, interim state with known limitations, need clear phase tracking
- **Follow-up**: Each phase should be independently testable and shippable

### Decision: stdin Reading Pattern for Human Play

- **Context**: Current implementation prompts but never reads stdin, causing immediate exit. Need reliable interactive input handling.
- **Alternatives Considered**:
  1. Simple `stdin().read_line()` in loop - Direct, blocking, easy to test
  2. Async tokio stdin - Non-blocking, requires async runtime changes
  3. crossterm/termion for raw mode - Character-by-character input, better UX but complex
- **Selected Approach**: Simple blocking `stdin().read_line()` in game loop
- **Rationale**:
  - Aligns with existing synchronous CLI architecture
  - Standard pattern for line-based interactive tools
  - Easy to test with piped stdin in integration tests
  - Sufficient for poker gameplay (turn-based, not real-time)
- **Trade-offs**:
  - **Benefits**: Simple, testable, low risk, standard Rust pattern
  - **Compromises**: Line-buffered (need Enter key), no arrow key history, basic UX
- **Follow-up**: Future enhancement could add rustyline/crossterm for better readline experience

### Decision: Placeholder Warning Strategy

- **Context**: Commands like `eval` and `play --vs ai` have stub implementations that mislead users. Need clear communication without breaking existing workflows.
- **Alternatives Considered**:
  1. Return errors (exit code 1) - Forces users to acknowledge limitations
  2. Silent placeholders - Maintain status quo, document in help text only
  3. Prominent warnings - Display warning banners but allow execution
- **Selected Approach**: Prominent runtime warnings with consistent format
- **Rationale**:
  - Doesn't break existing scripts or tests
  - Users get immediate feedback when running commands
  - Allows demo/testing use cases while preventing misinterpretation of results
  - Consistent "WARNING:" prefix enables grep filtering in scripts
- **Trade-offs**:
  - **Benefits**: Backward compatible, clear communication, maintains demo utility
  - **Compromises**: Users can still ignore warnings, requires documentation updates
- **Follow-up**: Consider adding `--acknowledge-placeholder` flag for scripting use cases

### Decision: Test Infrastructure Enhancement Strategy

- **Context**: Current tests validate output format but not actual behavior (stdin blocking, parameter usage, state changes).
- **Alternatives Considered**:
  1. Add behavioral tests to existing test suite - Inline with current structure
  2. Separate behavioral test crate - Clear separation, easier to run subset
  3. Use rexpect for all interactive tests - Comprehensive but adds dependency
- **Selected Approach**: Add behavioral tests to existing test suite, document rexpect as future enhancement
- **Rationale**:
  - Behavioral tests should be mandatory for all commands
  - Existing test structure can accommodate new test types
  - Avoid new dependencies for initial implementation
  - rexpect can be added later for more sophisticated interaction testing
- **Trade-offs**:
  - **Benefits**: No new dependencies, fits current workflow, incremental improvement
  - **Compromises**: More verbose test code, limited interaction patterns
- **Follow-up**: Create testing guidelines document with behavioral test examples

## Risks & Mitigations

- **Risk 1**: Engine API limitations prevent true interactive gameplay
  - **Mitigation**: Phase 1 focuses on stdin reading and input parsing; game engine integration can be simplified version that demonstrates concept without full betting rounds

- **Risk 2**: Tests may not catch future regressions in interactive behavior
  - **Mitigation**: Add CI check that verifies Commands enum and COMMANDS array are synchronized; require behavioral tests for new commands

- **Risk 3**: Users may ignore placeholder warnings and misinterpret results
  - **Mitigation**: Display warnings on every invocation; add warnings to output (e.g., "[DEMO MODE]" suffix); update documentation with clear status indicators

- **Risk 4**: Scope creep - requirements span from trivial (doc fixes) to complex (full AI implementation)
  - **Mitigation**: Strict phase boundaries with independent approval; Phase 1 and 2 are committed, Phase 3-4 are optional based on resources

## References

- [Rust CLI Book - Testing](https://rust-cli.github.io/book/tutorial/testing.html) - Official testing patterns
- [Stack Overflow - Rust stdin blocking](https://stackoverflow.com/questions/30012995/how-can-i-read-non-blocking-from-stdin) - stdin reading patterns
- [CLI Implementation Status Audit](C:\Users\kouda\VSCode\Axiomind\docs\CLI_IMPLEMENTATION_STATUS.md) - Comprehensive command audit
- [Incident Report 2025-11-11](C:\Users\kouda\VSCode\Axiomind\docs\incidents\2025-11-11-human-mode-stub-implementation.md) - Root cause analysis
- [axm-engine API docs](C:\Users\kouda\VSCode\Axiomind\rust\engine\src\lib.rs) - Game engine interface documentation
