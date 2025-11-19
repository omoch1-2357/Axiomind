# Requirements Document

## Project Description (Input)
issue#59への対応を5つのPhaseに分けて行う。

This specification addresses GitHub issue #59: refactoring the monolithic `rust/cli/src/lib.rs` file (4,434 lines) into a proper module structure. The current file contains utilities, command handlers, validation logic, and the main run function all in one place, making it difficult to maintain and test. The refactoring will be executed in 5 incremental phases, each with its own pull request for review and validation.

## Requirements

### Requirement 1: Utility Function Extraction (Phase 1)
**Objective:** As a CLI developer, I want utility functions separated into dedicated modules, so that formatting, I/O, and validation logic is reusable and testable in isolation.

#### Acceptance Criteria
1. When Phase 1 refactoring is initiated, the CLI refactoring system shall extract all formatter functions (`format_suit`, `format_rank`, `format_card`, `format_board`, `format_action`) into a new `rust/cli/src/formatters.rs` module
2. When Phase 1 refactoring is initiated, the CLI refactoring system shall extract all I/O utility functions (`read_stdin_line`, `read_text_auto`, `ensure_parent_dir`) into a new `rust/cli/src/io_utils.rs` module
3. When Phase 1 refactoring is initiated, the CLI refactoring system shall extract all validation functions (`validate_speed`, `validate_dealing_meta`, `parse_player_action`) into a new `rust/cli/src/validation.rs` module
4. When utility functions are extracted, the CLI refactoring system shall update `lib.rs` to import and use the new modules via `mod` declarations and `use` statements
5. When Phase 1 is complete, the CLI refactoring system shall verify that all existing tests pass without modification
6. The CLI refactoring system shall maintain exact function signatures and behavior during extraction to preserve backward compatibility
7. The CLI refactoring system shall preserve all existing documentation comments in the extracted modules

### Requirement 2: Simple Command Extraction (Phase 2)
**Objective:** As a CLI developer, I want simple, self-contained commands separated into individual modules, so that command logic is organized and easier to maintain.

#### Acceptance Criteria
1. When Phase 2 refactoring is initiated, the CLI refactoring system shall create a new `rust/cli/src/commands/` directory for command handlers
2. When Phase 2 refactoring is initiated, the CLI refactoring system shall extract the `cfg` command handler into `rust/cli/src/commands/cfg.rs`
3. When Phase 2 refactoring is initiated, the CLI refactoring system shall extract the `doctor` command handler into `rust/cli/src/commands/doctor.rs`
4. When Phase 2 refactoring is initiated, the CLI refactoring system shall extract the `rng` command handler into `rust/cli/src/commands/rng.rs`
5. When Phase 2 refactoring is initiated, the CLI refactoring system shall extract the `deal` command handler into `rust/cli/src/commands/deal.rs`
6. When Phase 2 refactoring is initiated, the CLI refactoring system shall extract the `bench` command handler into `rust/cli/src/commands/bench.rs`
7. When command modules are created, the CLI refactoring system shall create a `rust/cli/src/commands/mod.rs` file that re-exports all command handlers
8. When Phase 2 is complete, the CLI refactoring system shall update the main `run()` function to delegate to the new command modules
9. When Phase 2 is complete, the CLI refactoring system shall verify that all existing tests pass without modification
10. The CLI refactoring system shall maintain all existing command-line argument handling and error reporting behavior

### Requirement 3: Moderate Command Extraction (Phase 3)
**Objective:** As a CLI developer, I want moderately complex commands with dependencies on utilities separated into modules, so that command logic is properly encapsulated.

#### Acceptance Criteria
1. When Phase 3 refactoring is initiated, the CLI refactoring system shall extract the `play` command handler into `rust/cli/src/commands/play.rs`
2. When Phase 3 refactoring is initiated, the CLI refactoring system shall extract the `stats` command handler into `rust/cli/src/commands/stats.rs`
3. When Phase 3 refactoring is initiated, the CLI refactoring system shall extract the `eval` command handler into `rust/cli/src/commands/eval.rs`
4. When Phase 3 refactoring is initiated, the CLI refactoring system shall extract the `export` command handler into `rust/cli/src/commands/export.rs`
5. When Phase 3 command modules are created, the CLI refactoring system shall ensure proper imports of utility modules (formatters, io_utils, validation)
6. When Phase 3 command modules are created, the CLI refactoring system shall extract shared helper functions (e.g., `export_sqlite`, `run_stats`) into appropriate locations
7. When Phase 3 is complete, the CLI refactoring system shall verify that all existing tests pass without modification
8. The CLI refactoring system shall maintain all existing error handling and output formatting behavior

### Requirement 4: Large Inline Command Extraction (Phase 4)
**Objective:** As a CLI developer, I want large commands with significant inline logic separated into dedicated modules with proper helper function organization, so that complex command implementations are maintainable.

#### Acceptance Criteria
1. When Phase 4 refactoring is initiated, the CLI refactoring system shall extract the `replay` command handler into `rust/cli/src/commands/replay.rs`
2. When Phase 4 refactoring is initiated, the CLI refactoring system shall extract the `verify` command handler into `rust/cli/src/commands/verify.rs`
3. When Phase 4 refactoring is initiated, the CLI refactoring system shall extract the `sim` command handler into `rust/cli/src/commands/sim.rs`
4. When Phase 4 refactoring is initiated, the CLI refactoring system shall extract the `dataset` command handler into `rust/cli/src/commands/dataset.rs`
5. When `verify` command is extracted, the CLI refactoring system shall extract the `ValidationError` struct and related validation logic into the verify module
6. When `sim` command is extracted, the CLI refactoring system shall extract helper functions (`play_hand_to_completion`, `sim_run_fast`) into the sim module
7. When `dataset` command is extracted, the CLI refactoring system shall extract helper functions (`compute_splits`, `dataset_stream_if_needed`) into the dataset module
8. When Phase 4 is complete, the CLI refactoring system shall verify that all existing tests pass without modification
9. The CLI refactoring system shall maintain all existing environment variable handling (e.g., `axiomind_SIM_FAST`, `axiomind_DATASET_STREAM_THRESHOLD`)
10. The CLI refactoring system shall preserve all existing error handling, logging, and progress reporting behavior

### Requirement 5: Run Function Refactoring (Phase 5)
**Objective:** As a CLI developer, I want the main `run()` function simplified to only handle argument parsing and command dispatch, so that the entry point is clean and maintainable.

#### Acceptance Criteria
1. When Phase 5 refactoring is initiated, the CLI refactoring system shall reduce the `run()` function to only contain argument parsing, command matching, and dispatch logic
2. When Phase 5 refactoring is initiated, the CLI refactoring system shall move all command execution logic to their respective command modules
3. When Phase 5 refactoring is initiated, the CLI refactoring system shall extract the `Commands` enum and `AxiomindCli` struct into a separate `rust/cli/src/cli.rs` module
4. When Phase 5 refactoring is initiated, the CLI refactoring system shall extract the `Vs` enum into `rust/cli/src/commands/play.rs` as it is only used by the play command
5. When Phase 5 is complete, the main `lib.rs` file shall contain only module declarations, re-exports, and the simplified `run()` function (target: under 100 lines)
6. When Phase 5 is complete, the CLI refactoring system shall verify that all existing tests pass without modification
7. The CLI refactoring system shall maintain all existing exit code behavior (0 for success, 2 for errors, 130 for interruptions)
8. The CLI refactoring system shall preserve all existing help and version display behavior

### Requirement 6: Test Preservation and Validation
**Objective:** As a QA engineer, I want all existing tests to continue passing after each phase, so that refactoring does not introduce regressions.

#### Acceptance Criteria
1. When any phase refactoring is completed, the CLI refactoring system shall run `cargo test --package axiomind_cli` and verify zero test failures
2. When any phase refactoring is completed, the CLI refactoring system shall run `cargo clippy --package axiomind_cli -- -D warnings` and verify zero warnings
3. When any phase refactoring is completed, the CLI refactoring system shall run `cargo fmt --package axiomind_cli -- --check` and verify formatting compliance
4. When tests are located in `lib.rs` and are closely related to extracted functionality, the CLI refactoring system shall move those tests to the appropriate module's test section
5. When tests are moved, the CLI refactoring system shall preserve all test names, assertions, and behavior
6. The CLI refactoring system shall ensure that all existing integration tests in `rust/cli/tests/` continue to pass without modification

### Requirement 7: Module Structure and Organization
**Objective:** As a CLI developer, I want a clear and consistent module structure, so that the codebase follows Rust best practices and project conventions.

#### Acceptance Criteria
1. The CLI refactoring system shall organize all modules according to the project structure defined in `.kiro/steering/structure.md`
2. The CLI refactoring system shall follow Rust naming conventions: `snake_case` for modules and files, `PascalCase` for types
3. When creating new modules, the CLI refactoring system shall include appropriate module-level documentation comments describing the module's purpose
4. When creating command modules, the CLI refactoring system shall use a consistent pattern: each command handler shall be a public function that takes output streams and returns `Result<(), CliError>`
5. When creating module hierarchies, the CLI refactoring system shall use `mod.rs` files to re-export public items and organize submodules
6. The CLI refactoring system shall maintain consistent import organization: standard library first, external crates second, internal crates third, current crate modules last (all alphabetically sorted)
7. The CLI refactoring system shall ensure that public API surface area remains minimal: only re-export items that are needed by `main.rs` or external consumers

### Requirement 8: Incremental Review and Validation
**Objective:** As a project maintainer, I want each phase delivered as a separate pull request with validation, so that changes can be reviewed incrementally and issues can be caught early.

#### Acceptance Criteria
1. When each phase is completed, the CLI refactoring system shall create a separate git branch following the naming pattern `cli-refactor-phase-N` (where N is 1-5)
2. When each phase is completed, the CLI refactoring system shall create a pull request with a descriptive title following the pattern `refactor(cli): Phase N - [Description]`
3. When creating a pull request, the CLI refactoring system shall include a PR description that lists all files moved/created, line count reduction in lib.rs, and test validation results
4. When each phase PR is created, the CLI refactoring system shall reference GitHub issue #59 in the PR description using "Part of #59" or "Addresses #59"
5. When Phase 5 is completed and merged, the CLI refactoring system shall create a final PR that closes issue #59 using "Closes #59" in the description
6. The CLI refactoring system shall ensure each phase builds successfully with `cargo build --package axiomind_cli --release` before PR creation
7. The CLI refactoring system shall ensure each phase maintains backward compatibility: no breaking changes to the public API or CLI interface

### Requirement 9: Documentation and Comments
**Objective:** As a future maintainer, I want clear documentation and comments explaining the refactored structure, so that the codebase remains approachable.

#### Acceptance Criteria
1. When any module is created, the CLI refactoring system shall include a module-level doc comment (`//!`) explaining the module's purpose and responsibilities
2. When public functions are extracted, the CLI refactoring system shall preserve all existing doc comments with correct references to types and modules
3. When helper functions are made module-private, the CLI refactoring system shall add or preserve regular comments (`///`) explaining their purpose
4. When the refactoring is complete, the CLI refactoring system shall update the top-level `lib.rs` doc comment to reflect the new module organization
5. The CLI refactoring system shall ensure all doc comment examples remain valid and compilable after refactoring
6. The CLI refactoring system shall maintain all existing inline comments that explain complex logic or non-obvious decisions
7. The CLI refactoring system shall follow the documentation style defined in `.kiro/steering/tech.md`: English language, clear descriptions, code examples where appropriate

### Requirement 10: Error Handling and Type Safety
**Objective:** As a CLI developer, I want error handling patterns to remain consistent and type-safe across all refactored modules, so that error reporting remains reliable.

#### Acceptance Criteria
1. When extracting command handlers, the CLI refactoring system shall maintain the existing `Result<(), CliError>` return type pattern
2. When extracting utility functions, the CLI refactoring system shall preserve exact error types and error messages
3. When creating new module boundaries, the CLI refactoring system shall avoid silencing errors: all errors shall propagate or be explicitly handled with user-visible output
4. The CLI refactoring system shall maintain the existing error conversion patterns using the `?` operator and `From` trait implementations
5. The CLI refactoring system shall ensure that all error paths continue to write to the appropriate output stream (stdout vs stderr)
6. The CLI refactoring system shall preserve all existing uses of the `ui::write_error()` and `ui::display_warning()` functions
7. When refactoring is complete, the CLI refactoring system shall verify that no `unwrap()`, `expect()`, or `panic!()` calls have been introduced in the refactored code

