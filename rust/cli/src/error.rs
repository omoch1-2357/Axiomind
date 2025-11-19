//! Error types for the CLI application.
//!
//! This module defines the error types used throughout the CLI for better
//! error propagation and handling.
//!
//! ## Batch Validation Errors
//!
//! The `BatchValidationError<T>` type provides a reusable pattern for commands
//! that process multiple items and need to collect errors with context. It is
//! used by verify, dataset, and sim commands for structured error reporting.

use std::fmt;

/// Custom error type for CLI operations.
///
/// This enum encompasses all error types that can occur during CLI execution,
/// allowing for proper error propagation using the `?` operator.
#[derive(Debug)]
pub enum CliError {
    /// I/O error (file operations, stdout/stderr writes, etc.)
    Io(std::io::Error),

    /// Invalid user input or command-line arguments
    InvalidInput(String),

    /// Configuration error
    Config(String),

    /// Engine-related error
    Engine(String),

    /// Operation was interrupted (e.g., by user with Ctrl+C)
    Interrupted(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Io(e) => write!(f, "I/O error: {}", e),
            CliError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            CliError::Config(msg) => write!(f, "Configuration error: {}", msg),
            CliError::Engine(msg) => write!(f, "Engine error: {}", msg),
            CliError::Interrupted(msg) => write!(f, "Interrupted: {}", msg),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::Io(e) => Some(e),
            _ => None,
        }
    }
}

// Automatic conversion from std::io::Error to CliError
impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        CliError::Io(error)
    }
}

// Conversion from String to CliError (for engine errors)
impl From<String> for CliError {
    fn from(error: String) -> Self {
        CliError::Engine(error)
    }
}

// Conversion from &str to CliError (for convenience)
impl From<&str> for CliError {
    fn from(error: &str) -> Self {
        CliError::Engine(error.to_string())
    }
}

/// Generic error type for batch validation operations.
///
/// Used by commands that process multiple items and need to collect errors
/// with context for user-friendly error reporting. Each error tracks the item
/// that failed and a descriptive error message.
///
/// # Type Parameters
///
/// * `T` - Context type identifying the failed item (e.g., `usize` for index,
///   `String` for file path, etc.). Must implement `Display` for error formatting.
///
/// # Examples
///
/// ```rust
/// use axiomind_cli::BatchValidationError;
///
/// // Hand validation error (by index)
/// let error = BatchValidationError {
///     item_context: 5,
///     message: "Invalid hand state".to_string(),
/// };
/// assert_eq!(error.to_string(), "5: Invalid hand state");
///
/// // File validation error (by path)
/// let error = BatchValidationError {
///     item_context: "data.jsonl".to_string(),
///     message: "Corrupted file".to_string(),
/// };
/// assert_eq!(error.to_string(), "data.jsonl: Corrupted file");
/// ```
#[derive(Debug)]
#[allow(dead_code)] // Will be used in Phase 4 (verify, dataset, sim commands)
pub struct BatchValidationError<T> {
    /// Context identifying the item that failed validation
    pub item_context: T,
    /// Descriptive error message
    pub message: String,
}

impl<T: std::fmt::Display> std::fmt::Display for BatchValidationError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.item_context, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_validation_error_with_usize() {
        let error = BatchValidationError {
            item_context: 42,
            message: "Test error".to_string(),
        };
        assert_eq!(error.to_string(), "42: Test error");
    }

    #[test]
    fn test_batch_validation_error_with_string() {
        let error = BatchValidationError {
            item_context: "file.txt".to_string(),
            message: "File not found".to_string(),
        };
        assert_eq!(error.to_string(), "file.txt: File not found");
    }
}
