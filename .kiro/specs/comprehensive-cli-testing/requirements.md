# Requirements Document

## Introduction

This feature implements a comprehensive test suite for the axiomind CLI poker application to ensure all functionality works correctly, handles edge cases properly, and maintains code quality with zero warnings. The test suite covers CLI argument validation, game logic correctness, file I/O operations, configuration management, and error handling across all commands and scenarios.

## Requirements

### Requirement 1: Basic CLI Functionality

**User Story:** As a developer, I want the basic CLI commands to work correctly, so that users can access help and version information reliably.

#### Acceptance Criteria

1. WHEN `axiomind --version` is executed THEN the system SHALL exit with code 0 and display version information
2. WHEN `axiomind --help` is executed THEN the system SHALL exit with code 0 and display help text containing all commands (play/replay/sim/eval/stats/verify/deal/bench/rng/cfg/doctor/export/dataset/serve/train)
3. WHEN `axiomind unknown` is executed with an invalid subcommand THEN the system SHALL exit with non-zero code and display help excerpt

### Requirement 2: Configuration and Options Management

**User Story:** As a user, I want CLI options to have correct default values and proper precedence, so that the application behaves predictably.

#### Acceptance Criteria

1. WHEN `--seed` is not specified THEN the system SHALL use non-deterministic behavior (no default seed)
2. WHEN `--adaptive` is not specified THEN the system SHALL default to "on"
3. WHEN `--ai-version` is not specified THEN the system SHALL default to "latest"
4. WHEN configuration conflicts exist THEN the system SHALL follow precedence: CLI arguments > environment variables > config file > defaults

### Requirement 3: Input Validation and Error Handling

**User Story:** As a user, I want proper error messages for invalid inputs, so that I can correct my usage quickly.

#### Acceptance Criteria

1. WHEN required arguments are missing THEN the system SHALL exit with code 2 and display usage information to stderr
2. WHEN invalid argument types are provided (negative numbers, non-numbers) THEN the system SHALL exit with code 2 and display usage information to stderr
3. WHEN `--vs human` is specified in non-TTY environment THEN the system SHALL warn and refuse execution

### Requirement 4: Game Logic Validation

**User Story:** As a poker player, I want the game rules to be implemented correctly, so that gameplay is fair and accurate.

#### Acceptance Criteria

1. WHEN any player's stack becomes zero THEN the system SHALL immediately end the game without processing additional hands
2. WHEN a game ends THEN the final JSONL record SHALL satisfy chip conservation (total chips remain constant)
3. WHEN invalid bet amounts are provided (e.g., --bet 26 with minimum chip unit violations) THEN the system SHALL reject the input appropriately
4. WHEN a raise is attempted with amount less than minimum raise delta THEN the system SHALL reject the raise
5. WHEN a player goes all-in with amount less than previous raise delta THEN the system SHALL NOT reopen betting action
6. WHEN dealing cards THEN the system SHALL follow correct order: BTN=SB, OOP=BB, BTN deals first, burn cards each street

### Requirement 5: Deterministic Behavior

**User Story:** As a developer, I want deterministic behavior with seeds, so that games can be reproduced for testing and debugging.

#### Acceptance Criteria

1. WHEN the same `--seed` and `--level` are used THEN the system SHALL produce identical JSONL output and results
2. WHEN `--seed` is specified for evaluation THEN the system SHALL produce reproducible win rates and EV calculations

### Requirement 6: File I/O and Data Handling

**User Story:** As a user, I want robust file handling, so that the application works with various input formats and recovers from data corruption.

#### Acceptance Criteria

1. WHEN `--input` is not specified for replay THEN the system SHALL exit with non-zero code
2. WHEN `--speed` is out of range (0 or negative) THEN the system SHALL exit with non-zero code
3. WHEN JSONL files have incomplete final lines THEN the system SHALL detect and discard the incomplete line and continue processing
4. WHEN input contains non-UTF-8 or CRLF line endings THEN the system SHALL handle gracefully or provide clear error messages
5. WHEN compressed JSONL files (*.jsonl.zst) are provided THEN the system SHALL decompress and process them correctly
6. WHEN `--input` specifies a directory THEN the system SHALL recursively scan for JSONL files
7. WHEN corrupted records are encountered THEN the system SHALL skip them and report the count with warnings

### Requirement 7: Simulation and Evaluation

**User Story:** As a poker analyst, I want accurate simulation and evaluation capabilities, so that I can analyze poker strategies effectively.

#### Acceptance Criteria

1. WHEN running simulations THEN `--hands <N>` SHALL be required with validation for N=0 and extremely large values
2. WHEN using `--resume` THEN the system SHALL detect duplicate hand_ids and handle them according to specification (skip duplicates or resume from interruption)
3. WHEN writing simulation data THEN the system SHALL use batched writes to SQLite/JSONL for performance
4. WHEN running evaluations THEN `--ai-a <name>`, `--ai-b <name>`, and `--hands <N>` SHALL be required
5. WHEN using identical AI models THEN the system SHALL warn according to design decisions

### Requirement 8: Statistics and Export

**User Story:** As a data analyst, I want reliable statistics and export functionality, so that I can analyze poker data in external tools.

#### Acceptance Criteria

1. WHEN calculating statistics THEN win/loss totals SHALL sum to zero and chip conservation SHALL be maintained
2. WHEN exporting to SQLite THEN the system SHALL generate proper schema with required columns and enforce NOT NULL constraints where appropriate

### Requirement 9: Dataset Management

**User Story:** As a machine learning engineer, I want precise dataset splitting, so that I can train models with proper data separation.

#### Acceptance Criteria

1. WHEN `--split` percentages don't sum to 100% THEN the system SHALL exit with non-zero code
2. WHEN `--seed` is specified for dataset splitting THEN the system SHALL produce identical splits across runs

### Requirement 10: Configuration Display

**User Story:** As a user, I want to see effective configuration values, so that I can understand how my settings are being applied.

#### Acceptance Criteria

1. WHEN displaying configuration THEN the system SHALL show final effective values from all sources (defaults/file/environment/CLI)
2. WHEN possible THEN the system SHALL indicate which source provided each configuration value

### Requirement 11: Verification and Diagnostics

**User Story:** As a developer, I want built-in verification and diagnostic tools, so that I can ensure the system is working correctly.

#### Acceptance Criteria

1. WHEN running verification THEN the system SHALL test hand evaluation antisymmetry: compare(a,b) = -compare(b,a)
2. WHEN running verification THEN the system SHALL test idempotency: evaluating the same hand twice produces identical results
3. WHEN running verification THEN the system SHALL validate side pot distribution conservation
4. WHEN running doctor diagnostics THEN the system SHALL test SQLite write permissions, data directory access, and UTF-8 locale
5. WHEN running RNG tests with `--seed` THEN the system SHALL produce deterministic output; without seed SHALL be non-deterministic
6. WHEN dealing cards THEN the system SHALL validate 52 unique cards, proper burn positions, and correct board card count

### Requirement 12: Data Format Validation

**User Story:** As a data consumer, I want strict data format validation, so that I can rely on data integrity.

#### Acceptance Criteria

1. WHEN processing JSONL THEN the system SHALL validate required fields: hand_id, seed, level, blinds, button, players, actions, board, showdown, net_result, end_reason, timestamp
2. WHEN validating hand_id THEN the system SHALL enforce YYYYMMDD-NNNNNN format and uniqueness
3. WHEN accessing SQLite THEN the system SHALL handle single-process write locks with appropriate retry/failure behavior
4. WHEN processing large files THEN the system SHALL avoid excessive memory usage and prevent OOM conditions

### Requirement 13: Game State Validation

**User Story:** As a poker player, I want correct game state management, so that the game follows proper poker rules.

#### Acceptance Criteria

1. WHEN a player has no winning potential THEN the system SHALL automatically fold the hand
2. WHEN street transitions occur THEN the system SHALL ensure all actions are complete before advancing
3. WHEN evaluating hands THEN the system SHALL maintain best_five idempotency and consistent ordering

### Requirement 14: Cross-Platform Compatibility

**User Story:** As a user on different systems, I want the application to handle various text formats, so that it works regardless of my platform.

#### Acceptance Criteria

1. WHEN input contains CRLF line endings THEN the system SHALL normalize to LF internally and process correctly