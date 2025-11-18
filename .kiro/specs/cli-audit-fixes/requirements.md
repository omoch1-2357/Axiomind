# Requirements Document: CLI Audit Fixes

## Project Description

Fix all issues identified in PR #22 CLI implementation audit. Address broken implementations (play --vs human), partial/stub implementations (play --vs ai, replay, eval), and explicitly mark unimplemented commands (serve, train) as such.

## Introduction

This specification addresses the comprehensive CLI implementation audit documented in `docs/CLI_IMPLEMENTATION_STATUS.md`. The audit revealed:
- 1 BROKEN command (play --vs human)
- 4 PARTIAL commands with stub/placeholder behavior (play --vs ai, replay, eval, and related issues)
- 2 PLANNED but non-existent commands documented in help text (serve, train)

The goal is to either fully implement these features or explicitly mark them as unimplemented/placeholder to prevent user confusion.

## Requirements

### Requirement 1: Fix Broken Human Play Mode
**Objective:** As a poker researcher, I want to play interactively against the AI via CLI, so that I can manually test AI behavior and game mechanics.

#### Acceptance Criteria
1. When user runs `axiomind play --vs human`, the CLI shall wait for stdin input before proceeding with each action
2. When user enters a valid action (check/call/bet/raise/fold), the CLI shall parse the input and apply it to the game engine
3. When user enters an invalid action, the CLI shall display an error message and re-prompt for input
4. When user enters 'q' or 'quit', the CLI shall terminate the game session and display summary statistics
5. When a betting action is required, the CLI shall accept and validate bet/raise amounts from user input
6. The CLI shall display current game state (cards, pot, stack sizes) before each action prompt
7. The CLI shall integrate with the game engine to progress through all game phases (preflop, flop, turn, river, showdown)
8. If the game reaches showdown, the CLI shall display final hands and declare the winner

### Requirement 2: Implement or Clearly Mark AI Play Mode
**Objective:** As a poker researcher, I want to either play against a functional AI opponent or understand that the current implementation is a placeholder, so that I don't misinterpret test results.

#### Acceptance Criteria
1. When user runs `axiomind play --vs ai`, the CLI shall either execute real AI decision-making OR display a clear warning that this is a placeholder implementation
2. If implementing real AI functionality, when the AI makes a decision, the CLI shall integrate with the game engine to apply legal poker actions based on game state
3. If implementing real AI functionality, the AI shall make context-aware decisions (not always "check") based on cards, pot odds, and position
4. If marking as placeholder, when the command starts, the CLI shall display: "WARNING: AI opponent is a placeholder that always checks. Use for demo purposes only."
5. If marking as placeholder, the CLI shall append "[DEMO MODE]" to all AI action output messages
6. The CLI shall complete full poker hands with proper game state progression regardless of implementation choice

### Requirement 3: Fix or Clarify Replay Functionality
**Objective:** As a poker researcher, I want to understand what the replay command actually does, so that I can use it appropriately or wait for full implementation.

#### Acceptance Criteria
1. When user runs `axiomind replay <file>`, the CLI shall accurately describe what operation is performed in the output message
2. If only counting lines, the CLI shall output "Counted: N hands in file" instead of "Replayed: N hands"
3. If the `--speed` parameter is not used by the implementation, the CLI shall remove it from the command signature
4. The CLI shall add a warning message: "Note: Full visual replay not yet implemented. This command only counts hands in the file."
5. When the file contains invalid JSONL, the CLI shall report parsing errors with line numbers
6. The CLI shall correctly count only valid hand records, excluding empty lines and malformed entries

### Requirement 4: Fix or Replace Eval Command
**Objective:** As a poker researcher, I want to either compare AI policies head-to-head or understand that eval returns random results, so that I can make informed decisions about evaluation methodology.

#### Acceptance Criteria
1. When user runs `axiomind eval --ai-a <name> --ai-b <name>`, the CLI shall either perform real AI comparison OR display a prominent warning about placeholder behavior
2. If keeping placeholder implementation, when the command starts, the CLI shall display: "WARNING: This is a placeholder returning random results. AI parameters are not used. For real simulations, use 'axiomind sim' command."
3. If keeping placeholder implementation, the final output shall include "[RANDOM RESULTS - NOT REAL AI COMPARISON]" in the summary
4. If implementing real functionality, when AIs are compared, the CLI shall load AI policy configurations and execute actual poker simulations
5. If implementing real functionality, the CLI shall use both `--ai-a` and `--ai-b` parameters to determine which strategies to compare
6. The CLI shall display clear distinction between placeholder mode and real evaluation mode in all output

### Requirement 5: Remove or Implement Serve Command
**Objective:** As a CLI user, I want the help text to accurately reflect available commands, so that I don't attempt to use non-existent features.

#### Acceptance Criteria
1. When user runs `axiomind --help`, the CLI shall NOT list "serve" in the available commands unless the command is implemented
2. The CLI shall remove "serve" from the COMMANDS constant in `rust/cli/src/lib.rs`
3. When user attempts `axiomind serve`, the CLI shall return an error with a helpful message: "Web server not integrated. Use: cargo run -p axiomind_web --bin axiomind-web-server"
4. If implementing the serve command, when user runs `axiomind serve`, the CLI shall spawn the axiomind-web-server binary process
5. If implementing the serve command, when server starts successfully, the CLI shall display the server URL and port
6. If implementing the serve command, when server fails to start, the CLI shall display the error reason and exit with non-zero status

### Requirement 6: Remove or Implement Train Command
**Objective:** As a CLI user, I want the help text to accurately reflect available commands, so that I don't attempt to use non-existent features.

#### Acceptance Criteria
1. When user runs `axiomind --help`, the CLI shall NOT list "train" in the available commands unless the command is implemented
2. The CLI shall remove "train" from the COMMANDS constant in `rust/cli/src/lib.rs`
3. When user attempts `axiomind train`, the CLI shall return an error message: "Training not yet implemented. This is a planned feature."
4. The CLI shall retain "train" in documentation files with clear "(planned)" markers
5. If implementing the train command in the future, the implementation shall be complete before adding back to COMMANDS list

### Requirement 7: Update Documentation for Accuracy
**Objective:** As a developer or user, I want documentation to accurately reflect implementation status, so that I have correct expectations about CLI functionality.

#### Acceptance Criteria
1. The CLI shall update `docs/CLI.md` to include an "Implementation Status" column for all commands
2. When a command is marked as PARTIAL, the documentation shall clearly describe the limitations
3. When a command is marked as PLANNED, the documentation shall state "Not yet available" in the description
4. The CLI shall update `CLAUDE.md` to include a "Known Issues" section referencing `docs/CLI_IMPLEMENTATION_STATUS.md`
5. When documentation is updated, all references to "serve" and "train" commands shall include accurate availability status
6. The documentation shall provide workarounds for non-integrated features (e.g., how to run web server manually)

### Requirement 8: Add Behavioral Tests
**Objective:** As a developer, I want tests that verify actual command behavior, so that future changes don't introduce regressions in functionality.

#### Acceptance Criteria
1. When testing `play --vs human`, the test suite shall verify that the command blocks waiting for stdin input
2. When testing `play --vs ai`, the test suite shall verify that AI decisions vary based on game state (if real implementation) or that placeholder warning is displayed
3. When testing `eval`, the test suite shall verify that AI parameters are actually used in computation (if real implementation) or that placeholder warning is displayed
4. When testing `replay`, the test suite shall verify that output message accurately describes the operation performed
5. When testing non-existent commands, the test suite shall verify that helpful error messages are displayed
6. The test suite shall include integration tests that validate game engine integration for interactive play modes
7. When behavioral tests fail, the test output shall clearly indicate which expected behavior was not observed

### Requirement 9: Prevent Future Incomplete Implementations
**Objective:** As a project maintainer, I want a checklist and process to prevent shipping incomplete command implementations, so that users always receive accurate information about feature availability.

#### Acceptance Criteria
1. When a new CLI command is proposed, the development process shall require completion of a "New Command Checklist" before merging
2. The checklist shall include: Command enum variant exists, implementation is complete (not stub), behavioral tests verify correctness, manual testing completed, documentation updated
3. When a command is marked as PLANNED, the CLI shall enforce that it does NOT appear in the COMMANDS constant
4. When a command accepts parameters, the test suite shall verify all parameters are actually used in the implementation
5. When documentation claims a feature exists, the CI pipeline shall include a test that verifies the command is in the Commands enum
6. The project shall maintain `docs/CLI_IMPLEMENTATION_STATUS.md` as the source of truth for all command implementation status

### Requirement 10: CLI User Experience Consistency
**Objective:** As a CLI user, I want consistent behavior across all commands regarding errors, warnings, and status indicators, so that I can easily understand what's happening.

#### Acceptance Criteria
1. When any command operates in placeholder/demo mode, the CLI shall display warnings using a consistent format: "WARNING: [explanation]"
2. When any command fails due to missing implementation, the CLI shall suggest alternatives or next steps
3. When any command displays status information, the CLI shall use consistent terminology (e.g., "DEMO MODE", "PLACEHOLDER", "NOT IMPLEMENTED")
4. The CLI shall use exit code 0 for successful operations and non-zero for errors consistently across all commands
5. When verbose mode is enabled, all commands shall display implementation status warnings even if they would normally be suppressed
6. The CLI shall maintain consistent output formatting for similar operations across different commands (e.g., hand counting, statistics display)
