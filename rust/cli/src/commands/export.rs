//! Hand history export command.
//!
//! This module provides functionality to convert hand histories between different formats
//! including CSV, JSON arrays, and SQLite databases.

use crate::error::CliError;
use crate::io_utils::read_text_auto;
use crate::ui;
use std::io::Write;

/// Handles the export command to convert hand histories between formats.
///
/// # Arguments
///
/// * `input` - Path to input JSONL file
/// * `output` - Path to output file
/// * `format` - Output format ("csv", "json", or "sqlite")
/// * `out` - Output stream for status messages
/// * `err` - Output stream for error messages
///
/// # Returns
///
/// `Result<(), CliError>`: `Ok(())` when export completes successfully.
pub fn handle_export_command(
    input: String,
    output: String,
    format: String,
    _out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    let content = read_text_auto(&input).map_err(|e| {
        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
        CliError::Config(format!("Failed to read {}: {}", input, e))
    })?;

    match format.as_str() {
        f if f.eq_ignore_ascii_case("csv") => export_csv(&content, &output, err),
        f if f.eq_ignore_ascii_case("sqlite") => export_sqlite(&content, &output, err),
        f if f.eq_ignore_ascii_case("json") => export_json(&content, &output, err),
        _ => Err(CliError::InvalidInput(format!(
            "Unsupported format: {}",
            format
        ))),
    }
}

/// Export to CSV format
fn export_csv(content: &str, output: &str, err: &mut dyn Write) -> Result<(), CliError> {
    if let Some(parent) = std::path::Path::new(output).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            let _ = ui::write_error(
                err,
                &format!("Failed to create parent directory for {}: {}", output, e),
            );
            CliError::Io(e)
        })?;
    }
    let mut w = std::fs::File::create(output)
        .map(std::io::BufWriter::new)
        .map_err(|e| {
            let _ = ui::write_error(err, &format!("Failed to write {}: {}", output, e));
            CliError::Io(e)
        })?;
    writeln!(w, "hand_id,seed,result,ts,actions,board")?;
    for (idx, line) in content.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let rec: axiomind_engine::logger::HandRecord = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                ui::write_error(err, &format!("Invalid record at line {}: {}", idx + 1, e))?;
                return Err(CliError::InvalidInput(format!(
                    "Invalid record at line {}: {}",
                    idx + 1,
                    e
                )));
            }
        };
        let seed_str = rec.seed.map(|v| v.to_string()).unwrap_or_else(|| "".into());
        let result = rec.result.unwrap_or_default();
        let ts = rec.ts.unwrap_or_default();
        writeln!(
            w,
            "{},{},{},{},{},{}",
            rec.hand_id,
            seed_str,
            result,
            ts,
            rec.actions.len(),
            rec.board.len()
        )?;
    }
    Ok(())
}

/// Export to JSON array format
fn export_json(content: &str, output: &str, err: &mut dyn Write) -> Result<(), CliError> {
    let mut arr = Vec::new();
    for (idx, line) in content.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                ui::write_error(err, &format!("Invalid record at line {}: {}", idx + 1, e))?;
                return Err(CliError::InvalidInput(format!(
                    "Invalid record at line {}: {}",
                    idx + 1,
                    e
                )));
            }
        };
        arr.push(v);
    }
    let s = serde_json::to_string_pretty(&arr).map_err(|e| {
        let _ = ui::write_error(err, &format!("Failed to serialize JSON: {}", e));
        CliError::InvalidInput(format!("Failed to serialize JSON: {}", e))
    })?;
    if let Some(parent) = std::path::Path::new(output).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            let _ = ui::write_error(
                err,
                &format!("Failed to create parent directory for {}: {}", output, e),
            );
            CliError::Io(e)
        })?;
    }
    std::fs::write(output, s).map_err(|e| {
        let _ = ui::write_error(err, &format!("Failed to write {}: {}", output, e));
        CliError::Io(e)
    })?;
    Ok(())
}

/// Export to SQLite format
fn export_sqlite(content: &str, output: &str, err: &mut dyn Write) -> Result<(), CliError> {
    enum ExportAttemptError {
        Busy(String),
        Fatal(String),
    }

    fn sqlite_busy(err: &rusqlite::Error) -> bool {
        matches!(
            err,
            rusqlite::Error::SqliteFailure(info, _)
                if matches!(
                    info.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                )
        )
    }

    fn export_sqlite_attempt(content: &str, output: &str) -> Result<(), ExportAttemptError> {
        let output_path = std::path::Path::new(output);
        if let Some(parent) = output_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|e| {
                ExportAttemptError::Fatal(format!(
                    "Failed to create parent directory for {}: {}",
                    output, e
                ))
            })?;
        }

        let mut conn = rusqlite::Connection::open(output).map_err(|e| {
            if sqlite_busy(&e) {
                ExportAttemptError::Busy(format!("open {}: {}", output, e))
            } else {
                ExportAttemptError::Fatal(format!("Failed to open {}: {}", output, e))
            }
        })?;

        let tx = conn.transaction().map_err(|e| {
            if sqlite_busy(&e) {
                ExportAttemptError::Busy(format!("start transaction: {}", e))
            } else {
                ExportAttemptError::Fatal(format!("Failed to start transaction: {}", e))
            }
        })?;

        tx.execute("DROP TABLE IF EXISTS hands", []).map_err(|e| {
            if sqlite_busy(&e) {
                ExportAttemptError::Busy(format!("reset schema: {}", e))
            } else {
                ExportAttemptError::Fatal(format!("Failed to reset schema: {}", e))
            }
        })?;

        tx.execute(
            "CREATE TABLE hands (
                hand_id TEXT NOT NULL PRIMARY KEY,
                seed INTEGER,
                result TEXT,
                ts TEXT,
                actions INTEGER NOT NULL,
                board INTEGER NOT NULL,
                raw_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| {
            if sqlite_busy(&e) {
                ExportAttemptError::Busy(format!("create schema: {}", e))
            } else {
                ExportAttemptError::Fatal(format!("Failed to create schema: {}", e))
            }
        })?;

        let mut stmt = tx
            .prepare(
                "INSERT INTO hands (hand_id, seed, result, ts, actions, board, raw_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| {
                if sqlite_busy(&e) {
                    ExportAttemptError::Busy(format!("prepare insert: {}", e))
                } else {
                    ExportAttemptError::Fatal(format!("Failed to prepare insert: {}", e))
                }
            })?;

        for (line_idx, line) in content.lines().enumerate() {
            let raw = line.trim();
            if raw.is_empty() {
                continue;
            }

            let record: axiomind_engine::logger::HandRecord = serde_json::from_str(raw)
                .map_err(|e| ExportAttemptError::Fatal(format!("Invalid record: {}", e)))?;

            let axiomind_engine::logger::HandRecord {
                hand_id,
                seed,
                actions,
                board,
                result,
                ts,
                ..
            } = record;

            let seed_val = match seed {
                Some(v) if v <= i64::MAX as u64 => Some(v as i64),
                Some(v) => {
                    return Err(ExportAttemptError::Fatal(format!(
                        "Seed value {} exceeds SQLite INTEGER range",
                        v
                    )));
                }
                None => None,
            };
            let result_val = result.unwrap_or_default();
            let ts_val = ts.unwrap_or_default();
            let actions_count = actions.len() as i64;
            let board_count = board.len() as i64;

            stmt.execute(rusqlite::params![
                &hand_id,
                seed_val,
                &result_val,
                &ts_val,
                actions_count,
                board_count,
                raw
            ])
            .map_err(|e| {
                if sqlite_busy(&e) {
                    ExportAttemptError::Busy(format!(
                        "insert record at line {}: {}",
                        line_idx + 1,
                        e
                    ))
                } else {
                    ExportAttemptError::Fatal(format!("Failed to insert record: {}", e))
                }
            })?;
        }

        drop(stmt);

        tx.commit().map_err(|e| {
            if sqlite_busy(&e) {
                ExportAttemptError::Busy(format!("commit export: {}", e))
            } else {
                ExportAttemptError::Fatal(format!("Failed to commit export: {}", e))
            }
        })?;

        Ok(())
    }

    let backoff_ms = std::env::var("AXIOMIND_SQLITE_BACKOFF_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let max_attempts = std::env::var("AXIOMIND_SQLITE_MAX_ATTEMPTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);

    if max_attempts == 0 {
        ui::write_error(err, "AXIOMIND_SQLITE_MAX_ATTEMPTS must be >= 1 (got 0)")?;
        return Err(CliError::Config(
            "AXIOMIND_SQLITE_MAX_ATTEMPTS must be >= 1".to_string(),
        ));
    }

    for attempt in 1..=max_attempts {
        match export_sqlite_attempt(content, output) {
            Ok(()) => return Ok(()),
            Err(ExportAttemptError::Busy(msg)) => {
                if attempt == max_attempts {
                    ui::write_error(
                        err,
                        &format!("SQLite busy after {} attempt(s): {}", attempt, msg),
                    )?;
                    return Err(CliError::Config(format!(
                        "SQLite busy after {} attempt(s): {}",
                        attempt, msg
                    )));
                }
                std::thread::sleep(std::time::Duration::from_millis(
                    backoff_ms * attempt as u64,
                ));
            }
            Err(ExportAttemptError::Fatal(msg)) => {
                ui::write_error(err, &msg)?;
                return Err(CliError::Config(msg));
            }
        }
    }

    unreachable!("export_sqlite loop should always return before this point")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_csv() {
        let temp_in = tempfile::NamedTempFile::new().unwrap();
        let temp_out = tempfile::NamedTempFile::new().unwrap();

        std::fs::write(
            temp_in.path(),
            br#"{"hand_id":"20250101-000001","seed":123,"actions":[],"board":[],"result":"p0","ts":"2025-01-01T00:00:00Z","meta":null,"showdown":null}
"#,
        )
        .unwrap();

        let input = temp_in.path().to_str().unwrap().to_string();
        let output = temp_out.path().to_str().unwrap().to_string();

        let mut out = Vec::new();
        let mut err = Vec::new();

        let result =
            handle_export_command(input, output.clone(), "csv".to_string(), &mut out, &mut err);

        assert!(result.is_ok());
        let csv_content = std::fs::read_to_string(output).unwrap();
        assert!(csv_content.contains("hand_id,seed,result"));
        assert!(csv_content.contains("20250101-000001"));
    }

    #[test]
    fn test_export_json() {
        let temp_in = tempfile::NamedTempFile::new().unwrap();
        let temp_out = tempfile::NamedTempFile::new().unwrap();

        std::fs::write(
            temp_in.path(),
            br#"{"hand_id":"20250101-000001","seed":123,"actions":[],"board":[],"result":"p0","ts":"2025-01-01T00:00:00Z","meta":null,"showdown":null}
"#,
        )
        .unwrap();

        let input = temp_in.path().to_str().unwrap().to_string();
        let output = temp_out.path().to_str().unwrap().to_string();

        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_export_command(
            input,
            output.clone(),
            "json".to_string(),
            &mut out,
            &mut err,
        );

        assert!(result.is_ok());
        let json_content = std::fs::read_to_string(output).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_content).unwrap();
        assert!(json.is_array());
    }

    #[test]
    fn test_export_unsupported_format() {
        let temp_in = tempfile::NamedTempFile::new().unwrap();
        let temp_out = tempfile::NamedTempFile::new().unwrap();

        std::fs::write(temp_in.path(), b"{}").unwrap();

        let input = temp_in.path().to_str().unwrap().to_string();
        let output = temp_out.path().to_str().unwrap().to_string();

        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_export_command(input, output, "xml".to_string(), &mut out, &mut err);

        assert!(result.is_err());
    }
}
