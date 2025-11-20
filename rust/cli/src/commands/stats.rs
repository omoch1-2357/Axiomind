//! Statistics aggregation command for hand history analysis.
//!
//! This module provides functionality to aggregate statistics from JSONL hand history files.
//! It computes summary metrics including total hands played, win distribution by player,
//! and validates chip conservation laws.

use crate::error::CliError;
use crate::io_utils::read_text_auto;
use crate::ui;
use std::io::Write;
use std::path::Path;

/// Aggregates statistics from JSONL hand history files.
///
/// Reads hand history files (JSONL or .jsonl.zst) and computes summary statistics
/// including total hands played and win distribution by player.
///
/// # Arguments
///
/// * `input` - Path to JSONL file or directory containing hand histories
/// * `out` - Output stream for statistics report
/// * `err` - Output stream for error messages and warnings
///
/// # Returns
///
/// `Result<(), CliError>`: `Ok(())` when statistics are valid, otherwise an `Err` that maps
/// to exit code `2`.
///
/// # Validation
///
/// - Detects corrupted or incomplete records
/// - Verifies chip conservation laws (sum of net_result must be zero)
/// - Reports warnings for skipped records
pub fn handle_stats_command(
    input: String,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    run_stats(&input, out, err)
}

/// Internal statistics aggregation implementation
fn run_stats(input: &str, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError> {
    struct StatsState {
        hands: u64,
        p0: u64,
        p1: u64,
        skipped: u64,
        corrupted: u64,
        stats_ok: bool,
    }

    fn consume_stats_content(
        content: String,
        state: &mut StatsState,
        err: &mut dyn Write,
    ) -> Result<(), CliError> {
        let has_trailing_nl = content.ends_with('\n');
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        for (i, line) in lines.iter().enumerate() {
            let parsed: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => {
                    if i == lines.len() - 1 && !has_trailing_nl {
                        state.skipped += 1;
                    } else {
                        state.corrupted += 1;
                    }
                    continue;
                }
            };

            let rec: axiomind_engine::logger::HandRecord =
                match serde_json::from_value(parsed.clone()) {
                    Ok(v) => v,
                    Err(_) => {
                        state.corrupted += 1;
                        continue;
                    }
                };

            if let Some(net_obj) = parsed.get("net_result").and_then(|v| v.as_object()) {
                let mut sum = 0i64;
                let mut invalid = false;
                for (player, val) in net_obj {
                    if let Some(n) = val.as_i64() {
                        sum += n;
                    } else {
                        invalid = true;
                        state.stats_ok = false;
                        ui::write_error(
                            err,
                            &format!(
                                "Invalid net_result value for {} at hand {}",
                                player, rec.hand_id
                            ),
                        )?;
                    }
                }
                if sum != 0 {
                    state.stats_ok = false;
                    ui::write_error(
                        err,
                        &format!("Chip conservation violated at hand {}", rec.hand_id),
                    )?;
                }
                if invalid {
                    continue;
                }
            }

            state.hands += 1;
            if let Some(r) = rec.result.as_deref() {
                if r == "p0" {
                    state.p0 += 1;
                }
                if r == "p1" {
                    state.p1 += 1;
                }
            }
        }
        Ok(())
    }

    let path = Path::new(input);
    let mut state = StatsState {
        hands: 0,
        p0: 0,
        p1: 0,
        skipped: 0,
        corrupted: 0,
        stats_ok: true,
    };

    if path.is_dir() {
        let mut stack = vec![path.to_path_buf()];
        while let Some(d) = stack.pop() {
            let rd = match std::fs::read_dir(&d) {
                Ok(v) => v,
                Err(_) => continue,
            };
            for e in rd.filter_map(Result::ok) {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Some(fname) = p.file_name().and_then(|f| f.to_str())
                    && (fname.ends_with(".jsonl") || fname.ends_with(".jsonl.zst"))
                {
                    match read_text_auto(&p.to_string_lossy()) {
                        Ok(content) => {
                            consume_stats_content(content, &mut state, err)?;
                        }
                        Err(_) => {
                            state.corrupted += 1;
                        }
                    }
                }
            }
        }
    } else {
        match read_text_auto(input) {
            Ok(s) => consume_stats_content(s, &mut state, err)?,
            Err(e) => {
                ui::write_error(err, &format!("Failed to read {}: {}", input, e))?;
                return Err(CliError::Config(format!("Failed to read {}: {}", input, e)));
            }
        }
    }

    if state.corrupted > 0 {
        ui::write_error(
            err,
            &format!("Skipped {} corrupted record(s)", state.corrupted),
        )?;
    }
    if state.skipped > 0 {
        ui::write_error(
            err,
            &format!("Discarded {} incomplete final line(s)", state.skipped),
        )?;
    }
    if !path.is_dir() && state.hands == 0 && (state.corrupted > 0 || state.skipped > 0) {
        ui::write_error(err, "Invalid record")?;
        return Err(CliError::InvalidInput("Invalid record".to_string()));
    }

    let summary = serde_json::json!({
        "hands": state.hands,
        "winners": { "p0": state.p0, "p1": state.p1 },
    });
    let json_output = serde_json::to_string_pretty(&summary)
        .map_err(|e| CliError::InvalidInput(format!("Failed to serialize stats: {}", e)))?;
    writeln!(out, "{}", json_output)?;
    if state.stats_ok {
        Ok(())
    } else {
        Err(CliError::InvalidInput(
            "Statistics validation failed".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_empty_file() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap().to_string();

        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_stats_command(path, &mut out, &mut err);

        assert!(result.is_ok());
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("\"hands\": 0"));
    }

    #[test]
    fn test_stats_single_hand() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(
            &mut temp,
            br#"{"hand_id":"20250101-000001","seed":123,"actions":[],"board":[],"result":"p0","ts":"2025-01-01T00:00:00Z","meta":null,"showdown":null}
"#,
        )
        .unwrap();

        let path = temp.path().to_str().unwrap().to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_stats_command(path, &mut out, &mut err);

        assert!(result.is_ok());
        let output = String::from_utf8(out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["hands"], 1);
        assert_eq!(json["winners"]["p0"], 1);
        assert_eq!(json["winners"]["p1"], 0);
    }

    #[test]
    fn test_stats_multiple_hands() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(
            &mut temp,
            br#"{"hand_id":"20250101-000001","seed":123,"actions":[],"board":[],"result":"p0","ts":"2025-01-01T00:00:00Z","meta":null,"showdown":null}
{"hand_id":"20250101-000002","seed":124,"actions":[],"board":[],"result":"p1","ts":"2025-01-01T00:00:01Z","meta":null,"showdown":null}
{"hand_id":"20250101-000003","seed":125,"actions":[],"board":[],"result":"p0","ts":"2025-01-01T00:00:02Z","meta":null,"showdown":null}
"#,
        )
        .unwrap();

        let path = temp.path().to_str().unwrap().to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_stats_command(path, &mut out, &mut err);

        assert!(result.is_ok());
        let output = String::from_utf8(out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["hands"], 3);
        assert_eq!(json["winners"]["p0"], 2);
        assert_eq!(json["winners"]["p1"], 1);
    }

    #[test]
    fn test_stats_chip_conservation() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(
            &mut temp,
            br#"{"hand_id":"20250101-000001","seed":123,"actions":[],"board":[],"result":"p0","net_result":{"p0":100,"p1":-100},"ts":"2025-01-01T00:00:00Z","meta":null,"showdown":null}
"#,
        )
        .unwrap();

        let path = temp.path().to_str().unwrap().to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_stats_command(path, &mut out, &mut err);

        assert!(result.is_ok());
        let output = String::from_utf8(out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["hands"], 1);
    }

    #[test]
    fn test_stats_chip_conservation_violation() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(
            &mut temp,
            br#"{"hand_id":"20250101-000001","seed":123,"actions":[],"board":[],"result":"p0","net_result":{"p0":100,"p1":-50},"ts":"2025-01-01T00:00:00Z","meta":null,"showdown":null}
"#,
        )
        .unwrap();

        let path = temp.path().to_str().unwrap().to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_stats_command(path, &mut out, &mut err);

        assert!(result.is_err());
        let err_output = String::from_utf8(err).unwrap();
        assert!(err_output.contains("Chip conservation violated"));
    }

    #[test]
    fn test_stats_corrupted_record() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(
            &mut temp,
            br#"{"hand_id":"20250101-000001","seed":123,"actions":[],"board":[],"result":"p0","ts":"2025-01-01T00:00:00Z","meta":null,"showdown":null}
{invalid json}
{"hand_id":"20250101-000003","seed":125,"actions":[],"board":[],"result":"p1","ts":"2025-01-01T00:00:02Z","meta":null,"showdown":null}
"#,
        )
        .unwrap();

        let path = temp.path().to_str().unwrap().to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_stats_command(path, &mut out, &mut err);

        assert!(result.is_ok());
        let output = String::from_utf8(out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["hands"], 2);
        let err_output = String::from_utf8(err).unwrap();
        assert!(err_output.contains("corrupted"));
    }

    #[test]
    fn test_stats_nonexistent_file() {
        let path = "/nonexistent/path/to/file.jsonl".to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_stats_command(path, &mut out, &mut err);

        assert!(result.is_err());
    }
}
