# Implementation Plan

- [ ] 1. Fix existing compiler warnings and setup test infrastructure
  - Remove unused imports and variables from rust/cli/src/lib.rs
  - Fix unreachable pattern warnings in command matching
  - Add #![deny(warnings)] to prevent regression
  - _Requirements: All requirements depend on warning-free code_

- [x] 2. Create core test infrastructure and utilities
- [x] 2.1 Implement CLI test runner utility
  - Create rust/cli/tests/helpers/mod.rs with common test utilities
  - Implement CliRunner struct for executing CLI commands with controlled inputs/outputs
  - Add methods for capturing exit codes, stdout, stderr, and execution timing
  - _Requirements: 1.1, 1.2, 1.3, 3.1, 3.2_

- [x] 2.2 Implement temporary file management utilities
  - Create TempFileManager for creating and cleaning up test files
  - Add methods for creating JSONL files, compressed files, and directories
  - Implement automatic cleanup on test completion
  - _Requirements: 6.3, 6.4, 6.5, 6.6_

- [x] 2.3 Create poker-specific assertion helpers
  - Implement PokerAssertions trait with hand validation methods
  - Add chip conservation validation functions
  - Create deterministic output comparison utilities
  - _Requirements: 4.2, 5.1, 5.2, 8.1_

- [x] 3. Implement basic CLI functionality tests (A-series)
- [x] 3.1 Create basic CLI command tests
  - Write tests for axiomind --version and axiomind --help with exit code validation
  - Verify help text contains all required commands (play/replay/sim/eval/stats/verify/deal/bench/rng/cfg/doctor/export/dataset/serve/train)
  - Test unknown subcommand handling with proper error messages
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 3.2 Implement configuration and default value tests
  - Test default values for --seed (non-deterministic), --adaptive (on), --ai-version (latest)
  - Verify configuration precedence: CLI > environment > config file > defaults
  - Test input validation for required arguments and type checking
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3_

- [x] 4. Implement game logic validation tests (B-series)
- [x] 4.1 Create game termination and chip conservation tests
  - Test immediate game termination when any stack reaches zero
  - Verify JSONL final record satisfies chip conservation laws
  - Test invalid bet amount handling with minimum chip unit validation
  - _Requirements: 4.1, 4.2, 4.3_

- [x] 4.2 Implement betting rule validation tests
  - Test minimum raise delta enforcement
  - Verify all-in scenarios don't reopen betting inappropriately
  - Test BTN/BB dealing order and burn card placement
  - _Requirements: 4.4, 4.5, 4.6_

- [x] 4.3 Create deterministic behavior tests
  - Verify identical output with same --seed and --level parameters
  - Test reproducible evaluation results with fixed seeds
  - _Requirements: 5.1, 5.2_

- [x] 5. Implement file I/O and data handling tests (C-series)
- [x] 5.1 Create input validation and error handling tests
  - Test --input requirement for replay command
  - Verify --speed range validation (reject 0 and negative values)
  - Test TTY detection for --vs human mode
  - _Requirements: 6.1, 6.2, 3.3_

- [x] 5.2 Implement corrupted data recovery tests
  - Test incomplete JSONL line detection and recovery
  - Verify handling of non-UTF-8 and CRLF mixed content
  - Test compressed JSONL file support (*.jsonl.zst)
  - _Requirements: 6.3, 6.4, 6.5_

- [x] 5.3 Create directory and file processing tests
  - Test recursive directory scanning for JSONL files
  - Verify corrupted record skipping with warning reports
  - _Requirements: 6.6, 6.7_

- [x] 6. Implement simulation and evaluation tests (D-E series)
- [x] 6.1 Create simulation parameter validation tests
  - Test --hands parameter requirement and validation (reject N=0, handle large values)
  - Implement resume functionality testing with duplicate hand_id detection
  - Verify batched write performance for SQLite/JSONL output
  - _Requirements: 7.1, 7.2, 7.3_

- [x] 6.2 Implement evaluation requirement tests
  - Test required parameters: --ai-a, --ai-b, --hands for evaluation
  - Verify identical AI model warnings
  - Test deterministic evaluation results with seeds
  - _Requirements: 7.4, 7.5, 5.2_

- [ ] 7. Implement statistics and export tests (F-G series)
- [x] 7.1 Create statistics validation tests
  - Test win/loss total conservation (sum to zero)
  - Verify chip conservation in statistical calculations
  - Test file and directory input handling for statistics
  - _Requirements: 8.1, 6.6_

- [x] 7.2 Implement export functionality tests
  - Test JSONL to SQLite conversion with proper schema
  - Verify required columns and NOT NULL constraint enforcement
  - Test export format validation and error handling
  - _Requirements: 8.2_

- [x] 8. Implement dataset management tests (H-series)
- [x] 8.1 Create dataset splitting validation tests
  - Test split percentage validation (must sum to 100%)
  - Verify boundary cases (99/1/0 splits)
  - Test deterministic splitting with --seed parameter
  - _Requirements: 9.1, 9.2_

- [x] 9. Implement configuration display tests (I-series)
- [x] 9.1 Create configuration merging and display tests
  - Test effective configuration value display from all sources
  - Verify source attribution for configuration values
  - Test configuration precedence integration
  - _Requirements: 10.1, 10.2_

- [x] 10. Implement verification and diagnostic tests (J-series)
- [x] 10.1 Create rule verification tests
  - Test hand evaluation antisymmetry: compare(a,b) = -compare(b,a)
  - Verify evaluation idempotency (same hand evaluated twice)
  - Test side pot distribution conservation
  - _Requirements: 11.1, 11.2, 11.3_

- [x] 10.2 Implement diagnostic tool tests
  - Test SQLite write permission checking
  - Verify data directory access validation
  - Test UTF-8 locale confirmation
  - _Requirements: 11.4_

- [x] 10.3 Create RNG and dealing tests
  - Test deterministic RNG output with --seed
  - Verify non-deterministic behavior without seed
  - Test card dealing validation (52 unique cards, proper burn positions, 5 board cards)
  - _Requirements: 11.5, 11.6_

- [x] 11. Implement data format validation tests (K-series)
- [x] 11.1 Create JSONL schema validation tests
  - Test required field validation: hand_id, seed, level, blinds, button, players, actions, board, showdown, net_result, end_reason, timestamp
  - Verify hand_id format enforcement (YYYYMMDD-NNNNNN)
  - Test hand_id uniqueness validation
  - _Requirements: 12.1, 12.2_

- [x] 11.2 Implement concurrency and performance tests
  - Test SQLite single-process write lock handling
  - Verify retry/failure behavior for lock conflicts
  - Test large file processing without excessive memory usage
  - _Requirements: 12.3, 12.4_

- [x] 12. Implement game state management tests (L-series)
- [x] 12.1 Create game state validation tests
  - Test automatic folding for hands with no winning potential
  - Verify proper street transition logic
  - Test hand evaluation consistency and ordering
  - _Requirements: 13.1, 13.2, 13.3_

- [x] 13. Implement cross-platform compatibility tests (M-series)
- [x] 13.1 Create text format handling tests
  - Test CRLF to LF normalization
  - Verify cross-platform text processing
  - Test encoding compatibility
  - _Requirements: 14.1_

- [x] 14. Create comprehensive integration test suite
- [x] 14.1 Implement end-to-end workflow tests
  - Create complete user scenario tests combining multiple commands
  - Test data flow between commands (sim -> stats -> export)
  - Verify configuration integration across all commands
  - _Requirements: All requirements integrated_

- [x] 14.2 Add performance and stress tests
  - Implement benchmark tests for large dataset processing
  - Test memory usage limits and OOM prevention
  - Create timeout tests for reasonable execution times
  - _Requirements: 12.4, performance aspects of all requirements_

- [x] 15. Finalize test suite and documentation
- [x] 15.1 Complete test coverage and cleanup
  - Run cargo test -q to ensure all tests pass
  - Verify zero compiler warnings across entire test suite
  - Add test documentation and usage examples
  - _Requirements: All requirements verified and documented_


