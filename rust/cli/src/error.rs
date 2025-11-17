//! Error types for the CLI application.
//!
//! This module defines the error types used throughout the CLI for better
//! error propagation and handling.

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
