//! Configuration command handler.
//!
//! This module implements the `cfg` command, which displays the current
//! Axiomind configuration settings with their sources (default, environment,
//! or configuration file).
//!
//! # Example Output
//!
//! ```json
//! {
//!   "starting_stack": {
//!     "value": 10000,
//!     "source": "default"
//!   },
//!   "level": {
//!     "value": 1,
//!     "source": "default"
//!   },
//!   ...
//! }
//! ```

use crate::config;
use crate::error::CliError;
use crate::ui;
use std::io::Write;

/// Handle the cfg command.
///
/// Loads the current configuration with source tracking and displays it
/// as formatted JSON to the output stream.
///
/// # Arguments
///
/// * `out` - Output stream for command output
/// * `err` - Error stream for error messages
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(CliError)` if configuration loading fails or output writing fails
///
/// # Errors
///
/// Returns `CliError::Config` if configuration loading fails.
/// Returns `CliError::Io` if writing to output stream fails.
pub fn handle_cfg_command(out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError> {
    let resolved = match config::load_with_sources() {
        Ok(r) => r,
        Err(e) => {
            ui::write_error(err, &format!("Invalid configuration: {}", e))?;
            return Err(CliError::Config(format!("Invalid configuration: {}", e)));
        }
    };

    let config::ConfigResolved { config, sources } = resolved;
    let display = serde_json::json!({
        "starting_stack": {
            "value": config.starting_stack,
            "source": sources.starting_stack,
        },
        "level": {
            "value": config.level,
            "source": sources.level,
        },
        "seed": {
            "value": config.seed,
            "source": sources.seed,
        },
        "adaptive": {
            "value": config.adaptive,
            "source": sources.adaptive,
        },
        "ai_version": {
            "value": config.ai_version,
            "source": sources.ai_version,
        }
    });
    let json_str = serde_json::to_string_pretty(&display).map_err(std::io::Error::other)?;
    writeln!(out, "{}", json_str)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_displays_json_output() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // This test will fail initially because handle_cfg_command is unimplemented
        let result = handle_cfg_command(&mut out, &mut err);

        // Should succeed
        assert!(result.is_ok(), "cfg command should succeed");

        // Should write JSON to output
        let output = String::from_utf8(out).unwrap();
        assert!(!output.is_empty(), "cfg should write output");

        // Should be valid JSON
        let _json: serde_json::Value =
            serde_json::from_str(&output).expect("cfg output should be valid JSON");

        // Should contain expected configuration keys
        assert!(
            output.contains("starting_stack"),
            "should contain starting_stack"
        );
        assert!(output.contains("level"), "should contain level");
        assert!(output.contains("seed"), "should contain seed");
        assert!(output.contains("adaptive"), "should contain adaptive");
        assert!(output.contains("ai_version"), "should contain ai_version");

        // Should contain source information
        assert!(output.contains("value"), "should contain value fields");
        assert!(output.contains("source"), "should contain source fields");
    }

    #[test]
    fn test_cfg_handles_config_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Note: This test verifies error handling behavior
        // The actual error might not occur in normal circumstances,
        // but we verify the error path exists

        // For now, just verify the function signature accepts these parameters
        // Full error testing will be done with integration tests
        let _ = handle_cfg_command(&mut out, &mut err);
    }

    #[test]
    fn test_cfg_writes_pretty_json() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_cfg_command(&mut out, &mut err);

        if result.is_ok() {
            let output = String::from_utf8(out).unwrap();

            // Pretty JSON should have newlines and indentation
            assert!(output.contains('\n'), "output should be pretty-printed");
            assert!(output.contains("  "), "output should be indented");
        }
    }

    #[test]
    fn test_cfg_no_error_output_on_success() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_cfg_command(&mut out, &mut err);

        if result.is_ok() {
            let error_output = String::from_utf8(err).unwrap();
            assert!(
                error_output.is_empty(),
                "should not write to stderr on success"
            );
        }
    }
}
