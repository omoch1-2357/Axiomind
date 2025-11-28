//! UI helper functions for terminal output formatting.
//!
//! This module provides utility functions for consistent user interface output
//! across CLI commands, including error messages, warnings, and status displays.

use std::io::Write;

pub fn write_error(err: &mut dyn Write, msg: &str) -> std::io::Result<()> {
    writeln!(err, "Error: {}", msg)
}

/// Display a warning message to stderr with "WARNING:" prefix
pub fn display_warning(err: &mut dyn Write, message: &str) -> std::io::Result<()> {
    writeln!(err, "WARNING: {}", message)
}

/// Display parameter ignored warning
pub fn warn_parameter_unused(err: &mut dyn Write, param_name: &str) -> std::io::Result<()> {
    writeln!(
        err,
        "WARNING: Parameter --{} is not used by the current implementation.",
        param_name
    )
}

/// Add demo mode tag to output line
pub fn tag_demo_output(line: &str) -> String {
    format!("{} [DEMO MODE]", line)
}
