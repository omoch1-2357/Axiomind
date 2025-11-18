# Design Document

## Overview

This design implements a comprehensive test suite for the axiomind CLI poker application that ensures all functionality works correctly with zero warnings. The test framework is organized into multiple test categories that systematically validate CLI behavior, game logic, file I/O, configuration management, and error handling.

The design leverages Rust's built-in testing framework with additional utilities for CLI testing, temporary file management, and deterministic testing scenarios. The test suite is structured to run efficiently with `cargo test -q` and provides clear failure diagnostics.

## Architecture

### Test Organization Structure

```
rust/cli/tests/
├── integration/           # Integration test modules
│   ├── cli_basic.rs      # A1-A5: Basic CLI functionality
│   ├── game_logic.rs     # B1-B7: Game logic validation  
│   ├── file_io.rs        # C1-C4: File I/O operations
│   ├── simulation.rs     # D1-D3: Simulation features
│   ├── evaluation.rs     # E1-E2: Evaluation features
│   ├── statistics.rs     # F1-F3: Statistics processing
│   ├── export.rs         # G1: Export functionality
│   ├── dataset.rs        # H1-H2: Dataset management
│   ├── config.rs         # I1: Configuration display
│   ├── verification.rs   # J1-J4: Verification tools
│   ├── data_format.rs    # K1-K4: Data format validation
│   ├── game_state.rs     # L1-L3: Game state management
│   └── compatibility.rs  # M1: Cross-platform compatibility
├── helpers/              # Test utility modules
│   ├── mod.rs           # Common test utilities
│   ├── cli_runner.rs    # CLI execution helpers
│   ├── temp_files.rs    # Temporary file management
│   ├── assertions.rs    # Custom assertion helpers
│   └── fixtures.rs      # Test data fixtures
└── fixtures/            # Static test data
    ├── valid_hands.jsonl
    ├── invalid_hands.jsonl
    ├── compressed.jsonl.zst
    └── config_samples/
```

### Core Components

#### CLI Test Runner
A utility that executes the CLI with controlled inputs and captures outputs, exit codes, and timing information. Supports:
- Argument injection
- Environment variable control
- Input/output redirection
- Exit code validation
- Timeout handling

#### Temporary File Manager
Manages creation and cleanup of temporary files and directories for testing file I/O operations. Features:
- Automatic cleanup on test completion
- Structured directory creation
- File permission testing
- Compression support

#### Assertion Framework
Custom assertions for poker-specific validations:
- JSONL format validation
- Hand record completeness
- Chip conservation checks
- Deterministic behavior verification
- Error message format validation

#### Test Data Fixtures
Predefined test data for consistent testing:
- Valid hand records in various formats
- Invalid/corrupted data samples
- Configuration file templates
- Compressed file samples

## Components and Interfaces

### CLI Test Runner Interface

```rust
pub struct CliRunner {
    binary_path: PathBuf,
    temp_dir: TempDir,
}

impl CliRunner {
    pub fn new() -> Result<Self, TestError>;
    pub fn run(&self, args: &[&str]) -> CliResult;
    pub fn run_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> CliResult;
    pub fn run_with_input(&self, args: &[&str], input: &str) -> CliResult;
    pub fn run_with_timeout(&self, args: &[&str], timeout: Duration) -> CliResult;
}

pub struct CliResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}
```

### File Management Interface

```rust
pub struct TempFileManager {
    base_dir: TempDir,
}

impl TempFileManager {
    pub fn new() -> Result<Self, TestError>;
    pub fn create_file(&self, name: &str, content: &str) -> Result<PathBuf, TestError>;
    pub fn create_jsonl(&self, name: &str, records: &[HandRecord]) -> Result<PathBuf, TestError>;
    pub fn create_compressed(&self, name: &str, content: &str) -> Result<PathBuf, TestError>;
    pub fn create_directory(&self, name: &str) -> Result<PathBuf, TestError>;
    pub fn path(&self, name: &str) -> PathBuf;
}
```

### Assertion Helpers Interface

```rust
pub trait PokerAssertions {
    fn assert_valid_hand_id(&self, hand_id: &str);
    fn assert_chip_conservation(&self, records: &[HandRecord]);
    fn assert_deterministic_output(&self, seed: u64, output1: &str, output2: &str);
    fn assert_jsonl_format(&self, content: &str);
    fn assert_help_contains_commands(&self, help_text: &str);
}
```

## Data Models

### Test Configuration

```rust
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub binary_path: PathBuf,
    pub timeout: Duration,
    pub temp_dir_prefix: String,
    pub cleanup_on_failure: bool,
}

#[derive(Debug)]
pub struct TestError {
    pub kind: TestErrorKind,
    pub message: String,
    pub source: Option<Box<dyn std::error::Error>>,
}

#[derive(Debug)]
pub enum TestErrorKind {
    BinaryNotFound,
    ExecutionTimeout,
    UnexpectedExitCode,
    OutputMismatch,
    FileOperationFailed,
    AssertionFailed,
}
```

### Test Data Models

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestHandRecord {
    pub hand_id: String,
    pub seed: Option<u64>,
    pub level: u8,
    pub blinds: (u32, u32),
    pub button: String,
    pub players: Vec<TestPlayer>,
    pub actions: Vec<TestAction>,
    pub board: Vec<String>,
    pub showdown: Option<Vec<TestShowdown>>,
    pub net_result: HashMap<String, i32>,
    pub end_reason: String,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub struct TestScenario {
    pub name: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub input: Option<String>,
    pub expected_exit_code: i32,
    pub expected_stdout_contains: Vec<String>,
    pub expected_stderr_contains: Vec<String>,
}
```

## Error Handling

### Error Categories

1. **Test Setup Errors**: Binary not found, temporary directory creation failures
2. **Execution Errors**: CLI timeouts, unexpected crashes, signal handling
3. **Validation Errors**: Output format mismatches, assertion failures
4. **File I/O Errors**: Permission issues, corruption detection, cleanup failures

### Error Recovery Strategies

- **Graceful Degradation**: Continue with remaining tests when individual tests fail
- **Resource Cleanup**: Ensure temporary files are cleaned up even on test failures
- **Detailed Diagnostics**: Capture full context (args, env, files) when tests fail
- **Retry Logic**: Retry flaky operations like file system operations

### Error Reporting

```rust
impl TestError {
    pub fn with_context(self, context: &str) -> Self;
    pub fn with_args(self, args: &[&str]) -> Self;
    pub fn with_files(self, files: &[PathBuf]) -> Self;
}
```

## Testing Strategy

### Test Categories and Approach

#### A-Series: Basic CLI Functionality (A1-A5)
- **Approach**: Direct CLI execution with argument validation
- **Key Tests**: Version/help output, unknown command handling, default values, configuration precedence
- **Validation**: Exit codes, output content, help text completeness

#### B-Series: Game Logic Validation (B1-B7)  
- **Approach**: Game simulation with rule verification
- **Key Tests**: Argument combinations, termination conditions, betting validation, dealing order
- **Validation**: JSONL output correctness, chip conservation, rule compliance

#### C-Series: File I/O Operations (C1-C4)
- **Approach**: File manipulation with corruption testing
- **Key Tests**: Input validation, corrupted file handling, encoding support, compression
- **Validation**: Error handling, data recovery, format support

#### D-Series: Simulation Features (D1-D3)
- **Approach**: Long-running simulation testing
- **Key Tests**: Parameter validation, resume functionality, batch processing
- **Validation**: Output consistency, performance characteristics, resume accuracy

#### E-Series: Evaluation Features (E1-E2)
- **Approach**: AI evaluation with deterministic testing
- **Key Tests**: Required parameters, deterministic results
- **Validation**: Statistical consistency, reproducibility

#### F-Series: Statistics Processing (F1-F3)
- **Approach**: Data aggregation with validation
- **Key Tests**: File/directory input, error recovery, conservation laws
- **Validation**: Mathematical correctness, error reporting

#### G-Series: Export Functionality (G1)
- **Approach**: Format conversion testing
- **Key Tests**: JSONL to SQLite conversion, schema validation
- **Validation**: Data integrity, schema compliance

#### H-Series: Dataset Management (H1-H2)
- **Approach**: Data splitting with randomization testing
- **Key Tests**: Split ratio validation, deterministic splitting
- **Validation**: Ratio accuracy, reproducibility

#### I-Series: Configuration Display (I1)
- **Approach**: Configuration merging testing
- **Key Tests**: Source precedence, effective value display
- **Validation**: Configuration accuracy, source attribution

#### J-Series: Verification Tools (J1-J4)
- **Approach**: Rule verification and diagnostics
- **Key Tests**: Mathematical properties, system diagnostics, RNG testing
- **Validation**: Property verification, diagnostic accuracy

#### K-Series: Data Format Validation (K1-K4)
- **Approach**: Schema validation and performance testing
- **Key Tests**: Required fields, format compliance, concurrency, scalability
- **Validation**: Schema compliance, performance characteristics

#### L-Series: Game State Management (L1-L3)
- **Approach**: State transition testing
- **Key Tests**: Forced actions, transition validation, evaluation consistency
- **Validation**: State correctness, transition logic

#### M-Series: Cross-Platform Compatibility (M1)
- **Approach**: Text format normalization testing
- **Key Tests**: Line ending handling, encoding support
- **Validation**: Format normalization, compatibility

### Test Execution Strategy

#### Parallel Execution
- Tests are designed to run in parallel using Rust's built-in test runner
- Each test uses isolated temporary directories
- No shared state between tests

#### Deterministic Testing
- All tests that involve randomization use fixed seeds
- Deterministic tests verify reproducibility across runs
- Non-deterministic tests verify proper randomization

#### Performance Testing
- Benchmark tests measure performance characteristics
- Memory usage tests prevent OOM conditions
- Timeout tests ensure reasonable execution times

#### Integration Testing
- End-to-end workflows test complete user scenarios
- Cross-command integration tests verify data flow
- Configuration integration tests verify precedence rules

## Implementation Plan Integration

The test suite integrates with the existing codebase through:

1. **Cargo Integration**: Tests run via `cargo test` with proper dependency management
2. **CI/CD Integration**: Tests provide clear pass/fail status for automated builds  
3. **Development Workflow**: Tests can be run individually or in groups during development
4. **Documentation**: Test names and descriptions serve as living documentation

## Quality Assurance

### Code Coverage
- Aim for >90% code coverage across all CLI functionality
- Use `cargo tarpaulin` or similar tools for coverage measurement
- Identify and test edge cases in uncovered code paths

### Warning Elimination
- Address all compiler warnings identified in current codebase
- Implement `#![deny(warnings)]` in test modules to prevent regression
- Use clippy lints to maintain code quality

### Performance Benchmarks
- Establish baseline performance metrics
- Test with large datasets to ensure scalability
- Monitor memory usage to prevent resource leaks

### Maintenance Strategy
- Regular test review and updates as features evolve
- Automated test execution in CI/CD pipeline
- Clear documentation for adding new test cases