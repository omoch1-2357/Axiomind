//! Environment diagnostics and health checks command.
//!
//! The `doctor` command validates the local environment to ensure all dependencies
//! and file system access are working correctly. It performs various health checks
//! and reports results in JSON format.
//!
//! ## Checks Performed
//!
//! - **SQLite**: Verifies ability to create and write to SQLite databases
//! - **Data Directory**: Tests write permissions in data directory
//! - **Locale**: Ensures UTF-8 locale for proper text handling
//!
//! ## Environment Variables
//!
//! - `axiomind_DOCTOR_SQLITE_DIR`: Override SQLite check directory (default: temp dir)
//! - `axiomind_DOCTOR_DATA_DIR`: Override data directory path (default: `data/`)
//! - `axiomind_DOCTOR_LOCALE_OVERRIDE`: Force specific locale for testing

use crate::error::CliError;
use crate::ui;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a single diagnostic check result.
struct DoctorCheck {
    name: &'static str,
    ok: bool,
    detail: String,
    error: Option<String>,
}

impl DoctorCheck {
    /// Create a passing check result.
    fn ok(name: &'static str, detail: impl Into<String>) -> Self {
        DoctorCheck {
            name,
            ok: true,
            detail: detail.into(),
            error: None,
        }
    }

    /// Create a failing check result.
    fn fail(name: &'static str, detail: impl Into<String>, error: impl Into<String>) -> Self {
        DoctorCheck {
            name,
            ok: false,
            detail: detail.into(),
            error: Some(error.into()),
        }
    }

    /// Convert check result to JSON value.
    fn to_value(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        map.insert(
            "status".into(),
            serde_json::Value::String(if self.ok { "ok" } else { "fail" }.into()),
        );
        map.insert(
            "detail".into(),
            serde_json::Value::String(self.detail.clone()),
        );
        if let Some(err) = &self.error {
            map.insert("error".into(), serde_json::Value::String(err.clone()));
        }
        serde_json::Value::Object(map)
    }
}

/// Generate a unique suffix for temporary file names.
fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
}

/// Check SQLite database creation and write capability.
fn check_sqlite(dir: &Path) -> DoctorCheck {
    if !dir.exists() {
        return DoctorCheck::fail(
            "sqlite",
            format!("SQLite check looked for {}", dir.display()),
            format!(
                "SQLite check failed: directory {} does not exist",
                dir.display()
            ),
        );
    }
    if !dir.is_dir() {
        return DoctorCheck::fail(
            "sqlite",
            format!("SQLite check attempted in {}", dir.display()),
            format!("SQLite check failed: {} is not a directory", dir.display()),
        );
    }
    let candidate = dir.join(format!("axiomind-doctor-{}.sqlite", unique_suffix()));
    match rusqlite::Connection::open(&candidate) {
        Ok(conn) => {
            let pragma = conn.execute("PRAGMA user_version = 1", []);
            drop(conn);
            if pragma.is_err() {
                let _ = std::fs::remove_file(&candidate);
                return DoctorCheck::fail(
                    "sqlite",
                    format!("SQLite write attempt in {}", dir.display()),
                    format!(
                        "SQLite check failed: unable to write to {}",
                        candidate.display()
                    ),
                );
            }
            let _ = std::fs::remove_file(&candidate);
            DoctorCheck::ok(
                "sqlite",
                format!("SQLite write test passed in {}", dir.display()),
            )
        }
        Err(e) => {
            let _ = std::fs::remove_file(&candidate);
            DoctorCheck::fail(
                "sqlite",
                format!("SQLite write attempt in {}", dir.display()),
                format!("SQLite check failed: {}", e),
            )
        }
    }
}

/// Check data directory creation and write permissions.
fn check_data_dir(path: &Path) -> DoctorCheck {
    if !path.exists() {
        // Attempt to create data directory and subdirectories
        if let Err(e) = std::fs::create_dir_all(path) {
            return DoctorCheck::fail(
                "data_dir",
                format!("Data directory creation attempt at {}", path.display()),
                format!("Failed to create data directory: {}", e),
            );
        }

        // Create subdirectories for hands and splits
        let hands_dir = path.join("hands");
        let splits_dir = path.join("splits");

        if let Err(e) = std::fs::create_dir_all(&hands_dir) {
            return DoctorCheck::fail(
                "data_dir",
                format!("Subdirectory creation attempt at {}", hands_dir.display()),
                format!("Failed to create hands directory: {}", e),
            );
        }

        if let Err(e) = std::fs::create_dir_all(&splits_dir) {
            return DoctorCheck::fail(
                "data_dir",
                format!("Subdirectory creation attempt at {}", splits_dir.display()),
                format!("Failed to create splits directory: {}", e),
            );
        }

        eprintln!("Created data directory at {}", path.display());
    }
    if !path.is_dir() {
        return DoctorCheck::fail(
            "data_dir",
            format!("Data directory probe at {}", path.display()),
            format!(
                "Data directory check failed: {} is not a directory",
                path.display()
            ),
        );
    }
    let probe = path.join("axiomind-doctor-write.tmp");
    match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(b"ok") {
                let _ = std::fs::remove_file(&probe);
                return DoctorCheck::fail(
                    "data_dir",
                    format!("Data directory write attempt in {}", path.display()),
                    format!("Data directory check failed: {}", e),
                );
            }
            drop(file);
            let _ = std::fs::remove_file(&probe);
            DoctorCheck::ok(
                "data_dir",
                format!("Data directory '{}' is writable", path.display()),
            )
        }
        Err(e) => DoctorCheck::fail(
            "data_dir",
            format!("Data directory write attempt in {}", path.display()),
            format!("Data directory check failed: {}", e),
        ),
    }
}

/// Evaluate locale value for UTF-8 support.
fn evaluate_locale(source: &str, value: String) -> DoctorCheck {
    let lowered = value.to_ascii_lowercase();
    let display = value.clone();
    if lowered.contains("utf-8") || lowered.contains("utf8") {
        DoctorCheck::ok(
            "locale",
            format!("{} reports UTF-8 locale ({})", source, display),
        )
    } else {
        DoctorCheck::fail(
            "locale",
            format!("{} reports non-UTF-8 locale ({})", source, display.clone()),
            format!("Locale check failed: {}={} is not UTF-8", source, display),
        )
    }
}

/// Check locale configuration for UTF-8 support.
fn check_locale(override_val: Option<String>) -> DoctorCheck {
    if let Some(val) = override_val {
        return evaluate_locale("axiomind_DOCTOR_LOCALE_OVERRIDE", val);
    }
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(val) = std::env::var(key) {
            return evaluate_locale(key, val);
        }
    }
    let candidate =
        std::env::temp_dir().join(format!("axiomind-doctor-diagnosis-{}.txt", unique_suffix()));
    match std::fs::File::create(&candidate) {
        Ok(mut file) => {
            if let Err(e) = file.write_all("✓".as_bytes()) {
                let _ = std::fs::remove_file(&candidate);
                return DoctorCheck::fail(
                    "locale",
                    "UTF-8 filesystem probe failed",
                    format!("Locale check failed: {}", e),
                );
            }
            drop(file);
            let _ = std::fs::remove_file(&candidate);
            DoctorCheck::ok(
                "locale",
                "UTF-8 filesystem probe succeeded (fallback)".to_string(),
            )
        }
        Err(e) => DoctorCheck::fail(
            "locale",
            "UTF-8 filesystem probe failed",
            format!("Locale check failed: {}", e),
        ),
    }
}

/// Handle the doctor command - run environment diagnostics.
///
/// Validates the local environment to ensure all dependencies and file system
/// access are working correctly. Outputs a JSON report of check results.
///
/// # Arguments
///
/// * `out` - Output stream for diagnostic report (JSON format)
/// * `err` - Output stream for error messages
///
/// # Returns
///
/// * `Ok(())` if all checks pass
/// * `Err(CliError::Config)` if any check fails
pub fn handle_doctor_command(out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError> {
    let sqlite_dir = env::var("axiomind_DOCTOR_SQLITE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir());
    let data_dir = env::var("axiomind_DOCTOR_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"));
    let locale_override = env::var("axiomind_DOCTOR_LOCALE_OVERRIDE").ok();

    let checks = vec![
        check_sqlite(&sqlite_dir),
        check_data_dir(&data_dir),
        check_locale(locale_override),
    ];

    let mut report = serde_json::Map::new();
    let mut ok_all = true;
    for check in checks {
        if !check.ok {
            ok_all = false;
            if let Some(msg) = &check.error {
                ui::write_error(err, msg)?;
            }
        }
        report.insert(check.name.to_string(), check.to_value());
    }

    let mut checks_map = serde_json::Map::new();
    for (key, value) in report {
        checks_map.insert(key, value);
    }

    let output = serde_json::json!({
        "checks": serde_json::Value::Object(checks_map)
    });

    let json_output = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::InvalidInput(format!("Failed to serialize doctor report: {}", e)))?;
    writeln!(out, "{}", json_output)?;

    if ok_all {
        Ok(())
    } else {
        Err(CliError::Config(
            "Environment diagnostics failed".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doctor_command_returns_ok_with_valid_environment() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_doctor_command(&mut out, &mut err);

        // Should succeed with proper environment
        assert!(result.is_ok());

        // Output should contain JSON status report
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("sqlite"));
        assert!(output.contains("status"));
    }

    #[test]
    fn test_doctor_command_outputs_json_format() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let _ = handle_doctor_command(&mut out, &mut err);

        let output = String::from_utf8(out).unwrap();

        // Output should be parseable as JSON
        let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(&output);
        assert!(parsed.is_ok(), "Output should be valid JSON");

        if let Ok(json) = parsed {
            assert!(json.get("checks").is_some(), "Should have 'checks' field");
        }
    }

    #[test]
    fn test_doctor_command_checks_sqlite() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let _ = handle_doctor_command(&mut out, &mut err);

        let output = String::from_utf8(out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();

        // Should have SQLite check
        let checks = json.get("checks").and_then(|c| c.as_object());
        assert!(checks.is_some());
        assert!(checks.unwrap().contains_key("sqlite"));
    }

    #[test]
    fn test_doctor_command_no_error_output_on_success() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_doctor_command(&mut out, &mut err);

        // On success, stderr should be empty
        if result.is_ok() {
            assert!(err.is_empty(), "No error output expected on success");
        }
    }
}
