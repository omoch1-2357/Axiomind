//! Dataset splitting command handler for ML training preparation.
//!
//! This module provides functionality to split JSONL hand history files into
//! train/validation/test sets for machine learning workflows. Supports both
//! in-memory and streaming modes for handling large datasets efficiently.
//!
//! # Environment Variables
//!
//! - `AXIOMIND_DATASET_STREAM_THRESHOLD`: Minimum records for streaming mode (default: 10000)
//! - `AXIOMIND_DATASET_STREAM_TRACE`: Enable streaming debug output
//!
//! # Output Files
//!
//! Creates `train.jsonl`, `val.jsonl`, and `test.jsonl` in the output directory.
//!
//! # Examples
//!
//! ```no_run
//! use axiomind_cli::commands::dataset::handle_dataset_command;
//! use std::io;
//!
//! let mut out = io::stdout();
//! let mut err = io::stderr();
//!
//! // Split dataset with 70% train, 20% val, 10% test
//! handle_dataset_command(
//!     "data/sim.jsonl".to_string(),
//!     "data/splits".to_string(),
//!     Some(0.7),
//!     Some(0.2),
//!     Some(0.1),
//!     Some(42),
//!     &mut out,
//!     &mut err,
//! ).unwrap();
//! ```

use crate::error::CliError;
use crate::ui;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha20Rng;
use std::io::{BufRead, BufReader, BufWriter, Write};

/// Handle the dataset command: create training/validation/test splits.
///
/// Splits a JSONL hand history file into train/val/test sets for machine learning.
/// Supports both in-memory and streaming modes for large datasets.
///
/// # Arguments
///
/// * `input` - Path to input JSONL file
/// * `output_dir` - Output directory for split files
/// * `train` - Training set proportion (0.0-1.0 or percentage)
/// * `val` - Validation set proportion (0.0-1.0 or percentage)
/// * `test` - Test set proportion (0.0-1.0 or percentage)
/// * `seed` - RNG seed for reproducible shuffling
/// * `out` - Output stream for normal messages
/// * `err` - Output stream for error messages
///
/// # Returns
///
/// `Ok(())` on success, or `CliError` on failure
///
/// # Environment Variables
///
/// - `AXIOMIND_DATASET_STREAM_THRESHOLD`: Min records for streaming mode (default: 10000)
/// - `AXIOMIND_DATASET_STREAM_TRACE`: Enable streaming debug output
#[allow(clippy::too_many_arguments)]
pub fn handle_dataset_command(
    input: String,
    output_dir: String,
    train: Option<f64>,
    val: Option<f64>,
    test: Option<f64>,
    seed: Option<u64>,
    _out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    // Try streaming mode first
    match dataset_stream_if_needed(&input, &output_dir, train, val, test, seed, err)? {
        Some(()) => return Ok(()),
        None => { /* Continue with normal in-memory processing */ }
    }

    // In-memory processing for smaller datasets
    let content = std::fs::read_to_string(&input).map_err(|e| {
        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
        CliError::Io(e)
    })?;

    let mut lines: Vec<String> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|s| s.to_string())
        .collect();

    let n = lines.len();
    if n == 0 {
        ui::write_error(err, "Empty input")?;
        return Err(CliError::InvalidInput("Empty input".to_string()));
    }

    let splits = compute_splits(train, val, test).map_err(|msg| {
        let _ = ui::write_error(err, &msg);
        CliError::InvalidInput(msg)
    })?;

    let tr = splits[0];
    let va = splits[1];
    let te = splits[2];
    let sum = tr + va + te;

    if (sum - 1.0).abs() > 1e-6 {
        ui::write_error(err, "Splits must sum to 100% (1.0 total)")?;
        return Err(CliError::InvalidInput(
            "Splits must sum to 100% (1.0 total)".to_string(),
        ));
    }

    let mut rng = ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
    lines.shuffle(&mut rng);

    let n_tr = ((tr * n as f64).round() as usize).min(n);
    let n_va = ((va * n as f64).round() as usize).min(n.saturating_sub(n_tr));
    let _n_te = n.saturating_sub(n_tr + n_va);

    // Validate all records
    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if let Err(e) = serde_json::from_str::<axiomind_engine::logger::HandRecord>(trimmed) {
            ui::write_error(err, &format!("Invalid record at line {}: {}", idx + 1, e))?;
            return Err(CliError::InvalidInput(format!(
                "Invalid record at line {}: {}",
                idx + 1,
                e
            )));
        }
    }

    let (trv, rest) = lines.split_at(n_tr);
    let (vav, tev) = rest.split_at(n_va);

    let out_root = std::path::Path::new(&output_dir);
    std::fs::create_dir_all(out_root).map_err(|e| {
        let _ = ui::write_error(
            err,
            &format!("Failed to create directory {}: {}", output_dir, e),
        );
        CliError::Io(e)
    })?;

    let mut write_split = |name: &str, data: &[String]| -> Result<(), CliError> {
        let path = out_root.join(name);
        let file = std::fs::File::create(&path).map_err(|e| {
            let _ = ui::write_error(err, &format!("Failed to create {}: {}", path.display(), e));
            CliError::Io(e)
        })?;
        let mut writer = BufWriter::new(file);
        for l in data {
            writeln!(writer, "{}", l).map_err(|e| {
                let _ = ui::write_error(err, &format!("Failed to write {}: {}", path.display(), e));
                CliError::Io(e)
            })?;
        }
        Ok(())
    };

    write_split("train.jsonl", trv)?;
    write_split("val.jsonl", vav)?;
    write_split("test.jsonl", tev)?;

    Ok(())
}

/// Compute dataset split ratios from optional inputs.
///
/// Handles both percentage (>1.0) and ratio (0.0-1.0) formats.
/// Defaults to 80% train, 10% val, 10% test if not specified.
///
/// # Arguments
///
/// * `train` - Optional train ratio/percentage
/// * `val` - Optional validation ratio/percentage
/// * `test` - Optional test ratio/percentage
///
/// # Returns
///
/// Array of [train_ratio, val_ratio, test_ratio] or error message
fn compute_splits(
    train: Option<f64>,
    val: Option<f64>,
    test: Option<f64>,
) -> Result<[f64; 3], String> {
    const DEFAULTS: [f64; 3] = [0.8, 0.1, 0.1];
    let mut splits = [0.0; 3];

    for (idx, opt) in [train, val, test].into_iter().enumerate() {
        splits[idx] = match opt {
            Some(v) if v.is_sign_negative() => {
                return Err("Splits must be non-negative".into());
            }
            Some(v) if v > 1.0 + 1e-6 => v / 100.0,
            Some(v) => v,
            None => DEFAULTS[idx],
        };
    }

    let sum: f64 = splits.iter().sum();
    if (sum - 1.0).abs() > 1e-6 {
        return Err("Splits must sum to 100% (1.0 total)".into());
    }

    Ok(splits)
}

/// Attempt streaming mode for large datasets.
///
/// If the dataset exceeds the threshold, use streaming mode to avoid loading
/// everything into memory. Returns Some(()) if streaming was used, None otherwise.
fn dataset_stream_if_needed(
    input: &str,
    outdir: &str,
    train: Option<f64>,
    val: Option<f64>,
    test: Option<f64>,
    seed: Option<u64>,
    err: &mut dyn Write,
) -> Result<Option<()>, CliError> {
    fn strip_utf8_bom(s: &mut String) {
        const UTF8_BOM: &str = "\u{feff}";
        if s.starts_with(UTF8_BOM) {
            s.drain(..UTF8_BOM.len());
        }
    }

    let threshold = std::env::var("AXIOMIND_DATASET_STREAM_THRESHOLD")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(10_000);

    if threshold == 0 {
        return Ok(None);
    }

    let trace_stream = std::env::var("AXIOMIND_DATASET_STREAM_TRACE")
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);

    let count_file = std::fs::File::open(input).map_err(|e| {
        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
        CliError::Io(e)
    })?;

    let mut record_count = 0usize;
    {
        let reader = BufReader::new(count_file);
        let mut first_line = true;
        for line in reader.lines() {
            let mut line = line.map_err(|e| {
                let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                CliError::Io(e)
            })?;
            if first_line {
                strip_utf8_bom(&mut line);
                first_line = false;
            }
            if !line.trim().is_empty() {
                record_count += 1;
            }
        }
    }

    if record_count == 0 {
        ui::write_error(err, "Empty input")?;
        return Err(CliError::InvalidInput("Empty input".to_string()));
    }

    if record_count <= threshold {
        return Ok(None);
    }

    let splits = compute_splits(train, val, test).map_err(|msg| {
        let _ = ui::write_error(err, &msg);
        CliError::InvalidInput(msg)
    })?;

    let tr = splits[0];
    let va = splits[1];
    let n = record_count;
    let n_tr = ((tr * n as f64).round() as usize).min(n);
    let n_va = ((va * n as f64).round() as usize).min(n.saturating_sub(n_tr));

    let mut rng = ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
    let mut indices: Vec<usize> = (0..record_count).collect();
    indices.shuffle(&mut rng);

    #[derive(Clone, Copy)]
    enum SplitSlot {
        Train,
        Val,
        Test,
    }

    let mut assignments = vec![SplitSlot::Test; record_count];
    for &idx in indices.iter().take(n_tr) {
        assignments[idx] = SplitSlot::Train;
    }
    for &idx in indices.iter().skip(n_tr).take(n_va) {
        assignments[idx] = SplitSlot::Val;
    }

    std::fs::create_dir_all(outdir).map_err(|e| {
        let _ = ui::write_error(
            err,
            &format!("Failed to create directory {}: {}", outdir, e),
        );
        CliError::Io(e)
    })?;

    if trace_stream {
        ui::write_error(
            err,
            &format!("Streaming dataset input (records={})", record_count),
        )?;
    }

    let out_root = std::path::Path::new(outdir);
    let train_path = out_root.join("train.jsonl");
    let val_path = out_root.join("val.jsonl");
    let test_path = out_root.join("test.jsonl");

    let mut create_writer = |path: &std::path::Path| -> Result<BufWriter<std::fs::File>, CliError> {
        std::fs::File::create(path)
            .map(BufWriter::new)
            .map_err(|e| {
                let _ =
                    ui::write_error(err, &format!("Failed to create {}: {}", path.display(), e));
                CliError::Io(e)
            })
    };

    let mut train_writer = create_writer(&train_path)?;
    let mut val_writer = create_writer(&val_path)?;
    let mut test_writer = create_writer(&test_path)?;

    let data_file = std::fs::File::open(input).map_err(|e| {
        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
        CliError::Io(e)
    })?;

    let reader = BufReader::new(data_file);
    let mut record_idx = 0usize;
    let mut first_line = true;

    for (line_idx, line_res) in reader.lines().enumerate() {
        let mut line = line_res.map_err(|e| {
            let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
            CliError::Io(e)
        })?;

        if first_line {
            strip_utf8_bom(&mut line);
            first_line = false;
        }

        if line.trim().is_empty() {
            continue;
        }

        if let Err(e) = serde_json::from_str::<axiomind_engine::logger::HandRecord>(&line) {
            ui::write_error(
                err,
                &format!("Invalid record at line {}: {}", line_idx + 1, e),
            )?;
            return Err(CliError::InvalidInput(format!(
                "Invalid record at line {}: {}",
                line_idx + 1,
                e
            )));
        }

        let bucket = assignments
            .get(record_idx)
            .copied()
            .unwrap_or(SplitSlot::Test);
        record_idx += 1;

        match bucket {
            SplitSlot::Train => {
                writeln!(train_writer, "{}", line)?;
            }
            SplitSlot::Val => {
                writeln!(val_writer, "{}", line)?;
            }
            SplitSlot::Test => {
                writeln!(test_writer, "{}", line)?;
            }
        }
    }

    Ok(Some(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_splits_defaults() {
        let result = compute_splits(None, None, None).unwrap();
        assert_eq!(result, [0.8, 0.1, 0.1]);
    }

    #[test]
    fn test_compute_splits_custom_ratios() {
        let result = compute_splits(Some(0.7), Some(0.2), Some(0.1)).unwrap();
        assert_eq!(result, [0.7, 0.2, 0.1]);
    }

    #[test]
    fn test_compute_splits_percentages() {
        let result = compute_splits(Some(70.0), Some(20.0), Some(10.0)).unwrap();
        assert_eq!(result, [0.7, 0.2, 0.1]);
    }

    #[test]
    fn test_compute_splits_must_sum_to_one() {
        let result = compute_splits(Some(0.5), Some(0.3), Some(0.1));
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_splits_negative_rejects() {
        let result = compute_splits(Some(-0.1), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_dataset_command_basic_execution() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Test with non-existent file should fail
        let result = handle_dataset_command(
            "nonexistent.jsonl".to_string(),
            "test_output".to_string(),
            Some(0.8),
            Some(0.1),
            Some(0.1),
            Some(42),
            &mut out,
            &mut err,
        );

        assert!(result.is_err());
    }
}
