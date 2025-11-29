//! Macros for common CLI error handling patterns.
//!
//! This module provides macros to reduce boilerplate in error handling,
//! making the code more maintainable and consistent across the CLI.

/// Write to a stream and exit with error code if writing fails.
///
/// This macro handles the common pattern of attempting to write to stderr/stdout
/// and returning an error exit code if the write operation fails.
///
/// # Examples
///
/// ```ignore
/// write_or_exit!(err, "Error: {}", message);
/// ```
#[macro_export]
macro_rules! write_or_exit {
    ($dest:expr, $($arg:tt)*) => {
        if writeln!($dest, $($arg)*).is_err() {
            return $crate::exit_code::ERROR;
        }
    };
}

/// Parse a JSON line or continue to the next iteration on error.
///
/// This macro handles the common pattern of parsing JSONL records where
/// parse errors should be logged and the iteration should continue.
///
/// # Examples
///
/// ```ignore
/// let record: HandRecord = parse_json_or_continue!(line, err, hand_num);
/// ```
#[macro_export]
macro_rules! parse_json_or_continue {
    ($line:expr, $err:expr, $context:expr) => {
        match serde_json::from_str($line) {
            Ok(r) => r,
            Err(e) => {
                let _ =
                    $crate::ui::write_error($err, &format!("Failed to parse {}: {}", $context, e));
                continue;
            }
        }
    };
}
