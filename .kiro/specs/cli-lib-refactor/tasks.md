# Implementation Plan

## Phase 1: Utility Function Extraction

- [x] 1. Phase 1 Pre-Refactoring Audit and Setup
- [x] 1.1 (P) Create test migration matrix
  - Audit all inline tests in `rust/cli/src/lib.rs` `#[cfg(test)]` module
  - Document test name, line range, and target module for each test
  - Create migration matrix table mapping tests to Phase 1-5 destinations
  - Identify tests for formatters, validation, I/O utilities, and commands
  - Document matrix in Phase 1 PR description template
  - _Requirements: 6_

- [x] 1.2 (P) Create git branch for Phase 1
  - Checkout main branch and pull latest changes
  - Create new branch `cli-refactor-phase-1-utilities` from main
  - Verify clean working directory before starting extraction
  - _Requirements: 8_

- [x] 2. Create utility modules with documentation
- [x] 2.1 (P) Create formatters module
  - Create `rust/cli/src/formatters.rs` with module-level doc comment
  - Add module-level documentation explaining card/board/action formatting purpose
  - Document Unicode vs ASCII fallback behavior
  - Extract `supports_unicode()`, `format_suit()`, `format_rank()`, `format_card()`, `format_board()`, `format_action()` functions from lib.rs
  - Preserve all existing function doc comments and signatures
  - Add inline tests to `#[cfg(test)]` module for Unicode and ASCII modes
  - _Requirements: 1, 7, 9_

- [x] 2.2 (P) Create I/O utilities module
  - Create `rust/cli/src/io_utils.rs` with module-level doc comment
  - Add module-level documentation explaining file I/O helper purpose
  - Extract `read_stdin_line()`, `read_text_auto()`, `ensure_parent_dir()` functions from lib.rs
  - Preserve exact error handling patterns and `Result` types
  - Add inline tests for compressed (.zst) and plain text file handling
  - _Requirements: 1, 7, 10_

- [x] 2.3 (P) Create validation module
  - Create `rust/cli/src/validation.rs` with module-level doc comment
  - Add module-level documentation explaining input validation purpose
  - Extract `validate_speed()`, `validate_dealing_meta()`, `parse_player_action()` functions from lib.rs
  - Define `ParseResult` enum with `Action`, `Quit`, `Invalid` variants
  - Preserve exact validation logic and error messages for test compatibility
  - Add inline tests for edge cases (empty input, invalid actions, quit commands)
  - _Requirements: 1, 7, 10_

- [x] 2.4 (P) Add BatchValidationError to error module
  - Open `rust/cli/src/error.rs` and add module-level doc comment update
  - Define generic `BatchValidationError<T>` struct with `item_context: T` and `message: String` fields
  - Implement `std::fmt::Display` trait for `BatchValidationError<T>` where `T: std::fmt::Display`
  - Add doc comment explaining batch validation pattern and usage in verify/dataset/sim commands
  - Verify existing `CliError` enum remains unchanged
  - _Requirements: 1, 10_

- [x] 3. Update lib.rs imports and remove extracted code
- [x] 3.1 Update lib.rs module declarations
  - Add `mod formatters;`, `mod io_utils;`, `mod validation;` declarations to lib.rs
  - Add `use` statements for extracted functions used in lib.rs command handlers
  - Organize imports following standard library → external crates → internal crates → current crate pattern
  - Remove extracted function definitions from lib.rs
  - Preserve all command handler code (no command extraction in Phase 1)
  - _Requirements: 1, 7_
  - **Note**: Also fixed incomplete `validate_dealing_meta` function from task 2 by adding missing deal_sequence and burn_positions validation logic

- [x] 4. Migrate Phase 1 tests to new modules
- [x] 4.1 Migrate formatter tests
  - Identify all `test_format_*` tests in lib.rs test module
  - Move tests to `formatters.rs` `#[cfg(test)]` module
  - Update test imports to use `use super::*;`
  - Verify tests access both public and private formatter functions
  - Remove migrated tests from lib.rs
  - _Requirements: 6, 7_
  - **Note**: Tests were already migrated to formatters.rs during task 2. No formatter tests found in lib.rs to migrate.

- [x] 4.2 Migrate validation tests
  - Identify all `test_parse_*` and `test_validate_*` tests in lib.rs test module
  - Move tests to `validation.rs` `#[cfg(test)]` module
  - Preserve test assertions for `ParseResult` variants
  - Remove migrated tests from lib.rs
  - _Requirements: 6, 7_
  - **Note**: Tests were already migrated to validation.rs during task 2. Removed duplicate tests (17 tests) from lib.rs.

- [x] 4.3 Migrate I/O utility tests
  - Identify all `test_read_*` and file I/O tests in lib.rs test module
  - Move tests to `io_utils.rs` `#[cfg(test)]` module
  - Ensure tests cover both compressed and plain text file handling
  - Remove migrated tests from lib.rs
  - _Requirements: 6, 7_
  - **Note**: Tests were already migrated to io_utils.rs during task 2. Removed duplicate tests (5 tests) from lib.rs.
  - **Result**: lib.rs now contains only 6 integration/CLI validation tests (as per test migration matrix). Total test count: 48 tests (distributed across modules).

- [ ] 5. Phase 1 validation and PR creation
- [ ] 5.1 Run comprehensive validation suite
  - Execute `cargo build --package axiomind_cli --release` and verify zero errors
  - Execute `cargo test --package axiomind_cli` and verify zero test failures
  - Execute `cargo clippy --package axiomind_cli -- -D warnings` and verify zero warnings
  - Execute `cargo fmt --package axiomind_cli -- --check` and verify formatting compliance
  - Run manual smoke test with `axiomind deal` command to verify formatter integration
  - _Requirements: 6, 8_

- [ ] 5.2 Create Phase 1 pull request
  - Commit all changes with message "refactor(cli): Phase 1 - Extract utility functions"
  - Push branch to remote repository
  - Create PR with title "refactor(cli): Phase 1 - Utility Function Extraction"
  - Include PR description with files created, line count reduction, test migration matrix, and validation results
  - Add "Part of #59" reference in PR description
  - Apply labels: `refactor`, `cli`, `phase-1`
  - _Requirements: 8, 9_

## Phase 2: Simple Command Extraction

- [ ] 6. Phase 2 setup and commands directory creation
- [ ] 6.1 Create git branch for Phase 2
  - Checkout main branch and pull latest merged Phase 1 changes
  - Create new branch `cli-refactor-phase-2-simple-commands` from main
  - Verify Phase 1 utility modules are present
  - _Requirements: 8_

- [ ] 6.2 Create commands directory structure
  - Create `rust/cli/src/commands/` directory
  - Create `rust/cli/src/commands/mod.rs` with module-level doc comment
  - Add doc comment explaining command handler organization pattern
  - Initialize empty module (no re-exports yet)
  - _Requirements: 2, 7_

- [ ] 7. Extract simple command handlers
- [ ] 7.1 (P) Extract cfg command
  - Create `rust/cli/src/commands/cfg.rs` with module-level doc comment
  - Extract cfg command handler from lib.rs (lines ~3240-3275)
  - Define `pub fn handle_cfg_command(out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError>`
  - Preserve configuration display logic and output format
  - Add `pub use cfg::handle_cfg_command;` to commands/mod.rs
  - _Requirements: 2, 6, 9, 10_

- [ ] 7.2 (P) Extract doctor command
  - Create `rust/cli/src/commands/doctor.rs` with module-level doc comment
  - Extract doctor command handler from lib.rs
  - Extract inline `run_doctor()` helper as module-private function
  - Define `pub fn handle_doctor_command(out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError>`
  - Preserve all diagnostic check logic (RNG, file I/O, config validation)
  - Add `pub use doctor::handle_doctor_command;` to commands/mod.rs
  - _Requirements: 2, 6, 9, 10_

- [ ] 7.3 (P) Extract rng command
  - Create `rust/cli/src/commands/rng.rs` with module-level doc comment
  - Extract rng command handler from lib.rs (lines ~3425-3435)
  - Define `pub fn handle_rng_command(seed: Option<u64>, out: &mut dyn Write) -> Result<(), CliError>`
  - Preserve RNG seeding and distribution verification logic
  - Add `pub use rng::handle_rng_command;` to commands/mod.rs
  - _Requirements: 2, 6, 9, 10_

- [ ] 7.4 Extract deal command
  - Create `rust/cli/src/commands/deal.rs` with module-level doc comment
  - Extract deal command handler from lib.rs (lines ~3389-3423)
  - Define `pub fn handle_deal_command(seed: Option<u64>, out: &mut dyn Write) -> Result<(), CliError>`
  - Add import for `formatters::format_card` from Phase 1 utilities
  - Preserve card dealing and formatting output
  - Add `pub use deal::handle_deal_command;` to commands/mod.rs
  - _Requirements: 2, 6, 9, 10_

- [ ] 7.5 (P) Extract bench command
  - Create `rust/cli/src/commands/bench.rs` with module-level doc comment
  - Extract bench command handler from lib.rs (lines ~3363-3387)
  - Define `pub fn handle_bench_command(out: &mut dyn Write) -> Result<(), CliError>`
  - Preserve benchmark timing logic and performance metrics output
  - Add `pub use bench::handle_bench_command;` to commands/mod.rs
  - _Requirements: 2, 6, 9, 10_

- [ ] 8. Update lib.rs command dispatch for Phase 2
- [ ] 8.1 Update lib.rs to use command modules
  - Add `mod commands;` declaration to lib.rs
  - Add `use commands::*;` for all Phase 2 command handlers
  - Update `Commands::Cfg` match arm to call `handle_cfg_command(out, err)?`
  - Update `Commands::Doctor` match arm to call `handle_doctor_command(out, err)?`
  - Update `Commands::Rng` match arm to call `handle_rng_command(seed, out)?`
  - Update `Commands::Deal` match arm to call `handle_deal_command(seed, out)?`
  - Update `Commands::Bench` match arm to call `handle_bench_command(out)?`
  - Remove extracted command handler code from lib.rs
  - _Requirements: 2, 7_

- [ ] 9. Phase 2 validation and PR creation
- [ ] 9.1 Run comprehensive validation suite
  - Execute `cargo build --package axiomind_cli --release` and verify zero errors
  - Execute `cargo test --package axiomind_cli` and verify zero test failures
  - Execute `cargo clippy --package axiomind_cli -- -D warnings` and verify zero warnings
  - Execute `cargo fmt --package axiomind_cli -- --check` and verify formatting compliance
  - Run manual smoke tests: `axiomind cfg`, `axiomind doctor`, `axiomind bench`
  - Verify CLI help output unchanged for all extracted commands
  - _Requirements: 6, 8_

- [ ] 9.2 Create Phase 2 pull request
  - Commit all changes with message "refactor(cli): Phase 2 - Extract simple command handlers"
  - Push branch to remote repository
  - Create PR with title "refactor(cli): Phase 2 - Simple Command Extraction"
  - Include PR description with files created, line count reduction (~400 lines), and validation results
  - Add "Part of #59" reference in PR description
  - Apply labels: `refactor`, `cli`, `phase-2`
  - _Requirements: 8, 9_

## Phase 3: Moderate Command Extraction

- [ ] 10. Phase 3 setup
- [ ] 10.1 Create git branch for Phase 3
  - Checkout main branch and pull latest merged Phase 2 changes
  - Create new branch `cli-refactor-phase-3-moderate-commands` from main
  - Verify Phase 1 utilities and Phase 2 commands are present
  - _Requirements: 8_

- [ ] 11. Extract moderate complexity commands
- [ ] 11.1 Extract play command
  - Create `rust/cli/src/commands/play.rs` with module-level doc comment
  - Extract play command handler from lib.rs (lines ~1848-1869)
  - Extract nested helpers: `execute_play_command()`, `play_hand_with_two_ais()` as module-private functions
  - Define `Vs` enum with `Ai`, `Human` variants (moved from cli.rs, used by play only)
  - Define `pub fn handle_play_command(vs: Vs, hands: Option<u32>, speed: Option<u64>, out: &mut dyn Write, err: &mut dyn Write, stdin: &mut dyn BufRead) -> Result<(), CliError>`
  - Add imports for `formatters`, `validation::parse_player_action`, `io_utils::read_stdin_line`
  - Preserve interactive gameplay flow, AI integration, and input validation
  - Add `pub use play::handle_play_command;` to commands/mod.rs
  - _Requirements: 3, 6, 9, 10_

- [ ] 11.2 (P) Extract stats command
  - Create `rust/cli/src/commands/stats.rs` with module-level doc comment
  - Extract stats command handler from lib.rs (lines ~2240-2243)
  - Extract inline `run_stats()` helper as module-private function
  - Define `pub fn handle_stats_command(input: String, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError>`
  - Add import for `io_utils::read_text_auto` for JSONL file reading
  - Preserve statistics aggregation and SQLite parsing logic
  - Add `pub use stats::handle_stats_command;` to commands/mod.rs
  - _Requirements: 3, 6, 9, 10_

- [ ] 11.3 (P) Extract eval command
  - Create `rust/cli/src/commands/eval.rs` with module-level doc comment
  - Extract eval command handler from lib.rs (lines ~2681-2694)
  - Extract helpers: `handle_eval_command()`, `print_eval_results()` as module-private functions
  - Define module-private `EvalStats` struct with fields: hands, p0_wins, p1_wins, splits, p0_chips, p1_chips
  - Define `pub fn handle_eval_command(hands: u32, seed: Option<u64>, out: &mut dyn Write) -> Result<(), CliError>`
  - Preserve AI policy head-to-head evaluation and statistical analysis logic
  - Add `pub use eval::handle_eval_command;` to commands/mod.rs
  - _Requirements: 3, 6, 9, 10_

- [ ] 11.4 (P) Extract export command
  - Create `rust/cli/src/commands/export.rs` with module-level doc comment
  - Extract export command handler from lib.rs (lines ~2896-2908)
  - Extract `export_sqlite()` helper as module-private function
  - Define `pub fn handle_export_command(input: String, output: String, format: ExportFormat, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError>`
  - Add imports for `io_utils::read_text_auto`, `io_utils::ensure_parent_dir`
  - Preserve format conversion logic for CSV, JSON, SQLite
  - Add `pub use export::handle_export_command;` to commands/mod.rs
  - _Requirements: 3, 6, 9, 10_

- [ ] 12. Update lib.rs command dispatch for Phase 3
- [ ] 12.1 Update lib.rs to use Phase 3 command modules
  - Update `use commands::*;` to include Phase 3 handlers
  - Update `Commands::Play` match arm to call `handle_play_command(vs, hands, speed, out, err, stdin)?`
  - Update `Commands::Stats` match arm to call `handle_stats_command(input, out, err)?`
  - Update `Commands::Eval` match arm to call `handle_eval_command(hands, seed, out)?`
  - Update `Commands::Export` match arm to call `handle_export_command(input, output, format, out, err)?`
  - Remove extracted command handler code from lib.rs
  - _Requirements: 3, 7_

- [ ] 13. Migrate Phase 3 tests (if present)
- [ ] 13.1 Migrate helper function tests
  - Audit lib.rs for inline tests of play/stats/eval/export helpers
  - Move tests to respective command module `#[cfg(test)]` sections if found
  - Verify integration tests in `rust/cli/tests/` continue to pass
  - Document test migration in Phase 3 PR description
  - _Requirements: 6, 7_

- [ ] 14. Phase 3 validation and PR creation
- [ ] 14.1 Run comprehensive validation suite
  - Execute `cargo build --package axiomind_cli --release` and verify zero errors
  - Execute `cargo test --package axiomind_cli` and verify zero test failures (especially `test_play.rs`, `test_play_session.rs`, `test_stats.rs`, `test_eval.rs`, `test_export.rs`)
  - Execute `cargo clippy --package axiomind_cli -- -D warnings` and verify zero warnings
  - Execute `cargo fmt --package axiomind_cli -- --check` and verify formatting compliance
  - Run manual smoke tests: `axiomind play --vs ai --hands 1`, `axiomind stats <file>`, `axiomind eval --hands 100`
  - _Requirements: 6, 8_

- [ ] 14.2 Create Phase 3 pull request
  - Commit all changes with message "refactor(cli): Phase 3 - Extract moderate complexity commands"
  - Push branch to remote repository
  - Create PR with title "refactor(cli): Phase 3 - Moderate Command Extraction"
  - Include PR description with files created, line count reduction (~800 lines), and validation results
  - Add "Part of #59" reference in PR description
  - Apply labels: `refactor`, `cli`, `phase-3`
  - _Requirements: 8, 9_

## Phase 4: Large Inline Command Extraction

- [ ] 15. Phase 4 setup
- [ ] 15.1 Create git branch for Phase 4
  - Checkout main branch and pull latest merged Phase 3 changes
  - Create new branch `cli-refactor-phase-4-complex-commands` from main
  - Verify all previous phase modules are present and integrated
  - _Requirements: 8_

- [ ] 16. Extract complex commands with large inline handlers
- [ ] 16.1 Extract replay command
  - Create `rust/cli/src/commands/replay.rs` with module-level doc comment
  - Extract replay command handler from lib.rs (lines ~1870-2239)
  - Define `pub fn handle_replay_command(input: String, speed: Option<u64>, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError>`
  - Add imports for `formatters::*`, `validation::validate_speed`, `io_utils::read_text_auto`
  - Preserve replay timing logic, JSONL parsing, and board state display
  - Add `pub use replay::handle_replay_command;` to commands/mod.rs
  - _Requirements: 4, 6, 9, 10_

- [ ] 16.2 Extract verify command with batch validation
  - Create `rust/cli/src/commands/verify.rs` with module-level doc comment
  - Extract verify command handler from lib.rs (lines ~2244-2676)
  - Define type alias `type VerifyError = BatchValidationError<usize>;` for hand index context
  - Extract validation helpers as module-private functions using `VerifyError` type
  - Define `pub fn handle_verify_command(input: String, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError>`
  - Add imports for `error::BatchValidationError`, `io_utils::read_text_auto`
  - Preserve game rule validation logic and batch error reporting
  - Add `pub use verify::handle_verify_command;` to commands/mod.rs
  - _Requirements: 4, 6, 9, 10_

- [ ] 16.3 Extract sim command with environment variables
  - Create `rust/cli/src/commands/sim.rs` with module-level doc comment
  - Extract sim command handler from lib.rs (lines ~2722-2895)
  - Extract nested helpers: `play_hand_to_completion()`, `sim_run_fast()` as module-private functions
  - Define `pub fn handle_sim_command(hands: u32, output: String, seed: Option<u64>, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError>`
  - Add import for `io_utils::ensure_parent_dir`
  - Preserve environment variable handling for `axiomind_SIM_FAST` (fast simulation mode detection)
  - Preserve simulation loop, hand completion logic, and JSONL output
  - Add `pub use sim::handle_sim_command;` to commands/mod.rs
  - _Requirements: 4, 6, 9, 10_

- [ ] 16.4 Extract dataset command with streaming logic
  - Create `rust/cli/src/commands/dataset.rs` with module-level doc comment
  - Extract dataset command handler from lib.rs (lines ~2909-3039)
  - Extract helpers: `compute_splits()`, `dataset_stream_if_needed()` as module-private functions
  - Define `pub fn handle_dataset_command(input: String, output_dir: String, train_ratio: f64, val_ratio: f64, seed: Option<u64>, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError>`
  - Add imports for `io_utils::read_text_auto`, `io_utils::ensure_parent_dir`
  - Preserve environment variable handling for `axiomind_DATASET_STREAM_THRESHOLD` (file size threshold)
  - Preserve dataset split computation and streaming mode logic
  - Add `pub use dataset::handle_dataset_command;` to commands/mod.rs
  - _Requirements: 4, 6, 9, 10_

- [ ] 17. Update lib.rs command dispatch for Phase 4
- [ ] 17.1 Update lib.rs to use Phase 4 command modules
  - Update `use commands::*;` to include Phase 4 handlers
  - Update `Commands::Replay` match arm to call `handle_replay_command(input, speed, out, err)?`
  - Update `Commands::Verify` match arm to call `handle_verify_command(input, out, err)?`
  - Update `Commands::Sim` match arm to call `handle_sim_command(hands, output, seed, out, err)?`
  - Update `Commands::Dataset` match arm to call `handle_dataset_command(input, output_dir, train_ratio, val_ratio, seed, out, err)?`
  - Remove extracted command handler code from lib.rs (largest line reduction: ~2,500 lines)
  - _Requirements: 4, 7_

- [ ] 18. Migrate Phase 4 tests
- [ ] 18.1 Migrate complex command helper tests
  - Audit lib.rs for inline tests of verify validation helpers
  - Move validation tests to `verify.rs` `#[cfg(test)]` module if found
  - Move sim helper tests to `sim.rs` `#[cfg(test)]` module if found
  - Document test migration decisions in Phase 4 PR description
  - _Requirements: 6, 7_

- [ ] 19. Phase 4 validation and PR creation
- [ ] 19.1 Run comprehensive validation suite
  - Execute `cargo build --package axiomind_cli --release` and verify zero errors
  - Execute `cargo test --package axiomind_cli` and verify zero test failures (especially `test_replay.rs`, `test_validation.rs`, `test_sim.rs`, `test_sim_resume.rs`, `test_dataset.rs`)
  - Execute `cargo clippy --package axiomind_cli -- -D warnings` and verify zero warnings
  - Execute `cargo fmt --package axiomind_cli -- --check` and verify formatting compliance
  - Run manual smoke tests with environment variables: `axiomind_SIM_FAST=1 axiomind sim --hands 10 --output test.jsonl`
  - Verify `axiomind verify`, `axiomind replay`, `axiomind dataset` commands work correctly
  - _Requirements: 6, 8_

- [ ] 19.2 Create Phase 4 pull request
  - Commit changes in 4 sequential commits (one per command: replay, verify, sim, dataset)
  - Push branch to remote repository
  - Create PR with title "refactor(cli): Phase 4 - Large Inline Command Extraction"
  - Include PR description with files created, line count reduction (~2,500 lines), commit structure for review, and validation results
  - Add "Part of #59" reference in PR description
  - Apply labels: `refactor`, `cli`, `phase-4`
  - _Requirements: 8, 9_

## Phase 5: Run Function Refactoring and Cleanup

- [ ] 20. Phase 5 setup
- [ ] 20.1 Create git branch for Phase 5
  - Checkout main branch and pull latest merged Phase 4 changes
  - Create new branch `cli-refactor-phase-5-cleanup` from main
  - Verify lib.rs now contains primarily module declarations and run function
  - _Requirements: 8_

- [ ] 21. Extract CLI types to dedicated module
- [ ] 21.1 Create CLI types module
  - Create `rust/cli/src/cli.rs` with module-level doc comment
  - Add doc comment explaining Clap structure definitions purpose
  - Move `AxiomindCli` struct with `#[derive(Parser)]` from lib.rs to cli.rs
  - Move `Commands` enum with `#[derive(Subcommand)]` from lib.rs to cli.rs
  - Ensure all 13 subcommand variants preserved with exact argument structures
  - Make both types public (`pub struct AxiomindCli`, `pub enum Commands`)
  - _Requirements: 5, 7_

- [ ] 22. Simplify run function to dispatch only
- [ ] 22.1 Refactor run function in lib.rs
  - Add `mod cli;` declaration to lib.rs
  - Update imports to use `cli::AxiomindCli` and `cli::Commands`
  - Verify run function contains only: argument parsing via `AxiomindCli::parse_from(args)`, command enum matching, delegation to `commands::handle_*_command()`, and exit code conversion
  - Remove all business logic from run function (should be delegated to command modules)
  - Ensure exit code behavior preserved: 0 (success), 2 (error), 130 (interruption)
  - Target lib.rs line count: under 100 lines total
  - _Requirements: 5, 7, 10_

- [ ] 22.2 Update lib.rs module-level documentation
  - Update top-level `//!` doc comment to reflect new module organization
  - Document module hierarchy: cli types, utilities, commands, and support modules
  - Add brief description of each module's responsibility
  - Remove outdated comments referencing monolithic structure
  - _Requirements: 5, 9_

- [ ] 23. Final validation and documentation
- [ ] 23.1 Verify final module structure
  - Confirm lib.rs contains only module declarations, re-exports, and run function
  - Count lib.rs lines and verify under 100 lines (target: ~80 lines)
  - Verify all 13 command handlers exported from commands/mod.rs
  - Verify CLI types exported from cli.rs
  - Check that no code duplication exists between modules
  - _Requirements: 5, 7_

- [ ] 23.2 Run comprehensive final validation suite
  - Execute `cargo build --package axiomind_cli --release` and verify zero errors
  - Execute `cargo test --package axiomind_cli` and verify zero test failures across all 26 integration tests
  - Execute `cargo clippy --package axiomind_cli -- -D warnings` and verify zero warnings
  - Execute `cargo fmt --package axiomind_cli -- --check` and verify formatting compliance
  - Run `axiomind --help` and verify help output unchanged from original
  - Run representative commands from each phase to verify end-to-end functionality
  - _Requirements: 5, 6, 8_

- [ ] 23.3 Verify no unwrap/expect/panic introduced
  - Grep codebase for `unwrap()`, `expect()`, `panic!()` calls in refactored modules
  - Verify all error handling uses `?` operator or explicit `Result` returns
  - Confirm no error suppression or silent failures introduced
  - _Requirements: 10_

- [ ] 24. Phase 5 PR creation and issue closure
- [ ] 24.1 Create final pull request
  - Commit all changes with message "refactor(cli): Phase 5 - Run function cleanup and CLI types extraction"
  - Push branch to remote repository
  - Create PR with title "refactor(cli): Phase 5 - Run Function Refactoring and Cleanup"
  - Include PR description with final module structure, lib.rs line count (~80 lines, reduced from 4,434), complete refactoring summary, and validation results
  - Add "Closes #59" reference in PR description (final PR closes the issue)
  - Apply labels: `refactor`, `cli`, `phase-5`
  - _Requirements: 5, 8, 9_

- [ ] 24.2 Final documentation update
  - Verify all module-level doc comments accurately describe refactored structure
  - Confirm inline comments preserved for complex logic
  - Ensure all doc comment examples remain valid and compilable
  - Update lib.rs doc comment to reflect final module organization
  - _Requirements: 9_

## Cross-Phase Quality Assurance

- [ ] 25. Continuous verification across all phases
- [ ] 25.1 Maintain backward compatibility
  - Verify no breaking changes to public API after each phase
  - Ensure CLI interface (arguments, flags, help output) unchanged
  - Confirm error messages remain identical for test compatibility
  - Validate exit code behavior preserved across all commands
  - _Requirements: 6, 8, 10_

- [ ] 25.2 Ensure consistent module organization
  - Follow `snake_case` naming for all module files
  - Use module-level `//!` doc comments for all new modules
  - Organize imports consistently: std → external → internal → current crate
  - Use `pub use` re-exports in commands/mod.rs for clean imports
  - Minimize public API surface: only export items needed by lib.rs or external consumers
  - _Requirements: 7, 9_

- [ ] 25.3 Preserve error handling patterns
  - Maintain `Result<(), CliError>` return type for all command handlers
  - Use `?` operator for error propagation with From trait implementations
  - Ensure errors written to appropriate stream (stdout vs stderr)
  - Preserve `ui::write_error()` and `ui::display_warning()` usage
  - Verify no error silencing or unwrap calls introduced
  - _Requirements: 10_
