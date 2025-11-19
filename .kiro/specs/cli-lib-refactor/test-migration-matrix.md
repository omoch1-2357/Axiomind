# Test Migration Matrix - Phase 1 Pre-Refactoring Audit

**Generated**: 2025-11-19
**Source**: `rust/cli/src/lib.rs` lines 4169-4434 (#[cfg(test)] module)
**Total Tests Identified**: 25 tests

## Test Migration Plan

| Test Name | Line Range | Target Module | Migration Phase | Test Category | Notes |
|-----------|------------|---------------|-----------------|---------------|-------|
| `test_read_stdin_line_valid_input` | 4174-4180 | `io_utils.rs` | Phase 1 | I/O Utilities | Tests stdin reading with valid input |
| `test_read_stdin_line_with_whitespace` | 4182-4188 | `io_utils.rs` | Phase 1 | I/O Utilities | Tests whitespace trimming |
| `test_read_stdin_line_empty_after_trim` | 4190-4196 | `io_utils.rs` | Phase 1 | I/O Utilities | Tests empty input after trimming |
| `test_read_stdin_line_eof` | 4198-4204 | `io_utils.rs` | Phase 1 | I/O Utilities | Tests EOF handling |
| `test_read_stdin_line_bet_with_amount` | 4206-4212 | `io_utils.rs` | Phase 1 | I/O Utilities | Tests complex input with spaces |
| `test_parse_fold` | 4215-4222 | `validation.rs` | Phase 1 | Validation | Tests fold action parsing |
| `test_parse_check_case_insensitive` | 4224-4231 | `validation.rs` | Phase 1 | Validation | Tests case-insensitive parsing |
| `test_parse_call` | 4233-4240 | `validation.rs` | Phase 1 | Validation | Tests call action parsing |
| `test_parse_bet_with_amount` | 4242-4251 | `validation.rs` | Phase 1 | Validation | Tests bet with amount parsing |
| `test_parse_raise_with_amount` | 4253-4262 | `validation.rs` | Phase 1 | Validation | Tests raise with amount parsing |
| `test_parse_quit_lowercase` | 4264-4268 | `validation.rs` | Phase 1 | Validation | Tests quit command (lowercase) |
| `test_parse_quit_full` | 4270-4274 | `validation.rs` | Phase 1 | Validation | Tests quit command (full word) |
| `test_parse_quit_uppercase` | 4276-4280 | `validation.rs` | Phase 1 | Validation | Tests quit command (uppercase) |
| `test_parse_invalid_action` | 4282-4291 | `validation.rs` | Phase 1 | Validation | Tests invalid action handling |
| `test_parse_bet_no_amount` | 4293-4302 | `validation.rs` | Phase 1 | Validation | Tests bet without amount error |
| `test_parse_bet_negative_amount` | 4304-4313 | `validation.rs` | Phase 1 | Validation | Tests negative bet error |
| `test_parse_bet_invalid_amount` | 4315-4324 | `validation.rs` | Phase 1 | Validation | Tests non-numeric bet amount error |
| `test_execute_play_reads_stdin` | 4327-4347 | Keep in lib.rs | N/A | Integration | Tests play command integration; calls multiple modules |
| `test_execute_play_handles_quit` | 4349-4369 | Keep in lib.rs | N/A | Integration | Tests play command quit handling; integration-level |
| `test_execute_play_handles_invalid_input_then_valid` | 4371-4391 | Keep in lib.rs | N/A | Integration | Tests play command error recovery; integration-level |
| `test_execute_play_ai_mode` | 4393-4407 | Keep in lib.rs | N/A | Integration | Tests AI mode; integration-level |
| `test_play_level_validation_rejects_out_of_range` | 4409-4420 | Keep in lib.rs | N/A | CLI Validation | Tests clap validation; depends on AxiomindCli |
| `test_play_level_validation_accepts_valid_range` | 4422-4433 | Keep in lib.rs | N/A | CLI Validation | Tests clap validation; depends on AxiomindCli |

## Summary by Target Module

### Phase 1 Migrations

**io_utils.rs** (5 tests):
- All tests for `read_stdin_line()` function
- Tests cover: valid input, whitespace handling, empty input, EOF, complex input
- Action: Move to `io_utils.rs` `#[cfg(test)]` module

**validation.rs** (12 tests):
- All tests for `parse_player_action()` function
- Tests cover: all action types, case insensitivity, quit commands, error cases
- Action: Move to `validation.rs` `#[cfg(test)]` module

**formatters.rs** (0 tests):
- No inline tests found for formatter functions in lib.rs
- Note: Formatter functions may be tested via integration tests or have no existing coverage
- Action: No tests to migrate; add new tests if needed during extraction

### Keep in lib.rs (8 tests)

**Integration-level tests** (4 tests):
- Tests calling `execute_play_command()` which orchestrates multiple modules
- Decision: Keep in lib.rs as they test integration between components
- Alternative: Move to `rust/cli/tests/` integration test directory in future cleanup

**CLI Validation tests** (2 tests):
- Tests for `AxiomindCli` clap validation (level range checking)
- Decision: Keep in lib.rs until Phase 5 when CLI types are extracted
- Future: Consider moving to `cli.rs` test module in Phase 5

## Migration Decision Criteria

**MOVE to extracted module** if:
- ✅ Test calls ONLY functions extracted to that module
- ✅ Test has no dependencies on lib.rs-specific setup
- ✅ Test is unit-level (tests single function or small group)

**KEEP in lib.rs** if:
- ❌ Test calls functions from multiple modules (integration-style)
- ❌ Test validates public API through `run()` function or clap structures
- ❌ Test requires lib.rs-level orchestration

## Formatter Coverage Gap

**Observation**: No inline tests found for formatter functions (`format_suit`, `format_rank`, `format_card`, `format_board`, `format_action`, `supports_unicode`)

**Possible Reasons**:
1. Formatters are simple string conversions, considered low-risk
2. Coverage provided by integration tests that use formatted output
3. Manual testing during development deemed sufficient

**Recommendation**: Consider adding basic formatter tests during Phase 1 extraction for:
- Unicode vs ASCII mode switching
- Card/board formatting edge cases
- Action formatting for all PlayerAction variants

## Phase-by-Phase Test Migration Schedule

### Phase 1: Utility Extraction
- **Migrate**: 17 tests (5 io_utils + 12 validation)
- **Keep**: 8 tests (4 integration + 4 CLI validation)
- **Add**: Consider basic formatter tests (optional)

### Phase 2-4: Command Extraction
- **Migrate**: None expected (command logic tested via integration tests in `rust/cli/tests/`)
- **Keep**: All integration-level tests remain in lib.rs

### Phase 5: Run Function Cleanup
- **Migrate**: 2 CLI validation tests to `cli.rs` (if appropriate)
- **Final State**: lib.rs may retain integration tests or they could be moved to `rust/cli/tests/`

## Validation Checklist

After Phase 1 test migration:
- [ ] All 17 migrated tests pass in their new modules
- [ ] 8 remaining tests in lib.rs still pass
- [ ] `cargo test --package axiomind_cli` shows same total test count
- [ ] No duplicate tests between lib.rs and new modules
- [ ] Test coverage maintained or improved

---

**Matrix Status**: Complete
**Tests Audited**: 25 of 25
**Migration Targets Identified**: 17 tests to 2 modules (io_utils, validation)
**Integration Tests Preserved**: 8 tests remaining in lib.rs
