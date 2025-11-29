//! Exit code constants for the CLI application.
//!
//! This module centralizes all exit codes used by the CLI, making them
//! easier to maintain and ensuring consistency across commands.

/// Success exit code (standard Unix convention).
pub const SUCCESS: i32 = 0;

/// General error exit code.
pub const ERROR: i32 = 2;

/// Interrupted by user (Ctrl+C) exit code.
pub const INTERRUPTED: i32 = 130;
