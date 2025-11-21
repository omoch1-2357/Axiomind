//! # Axiomind CLI Library
//!
//! This library provides the command-line interface for the Axiomind poker engine.
//! It exposes subcommands for playing, simulating, analyzing, and verifying poker hands.
//!
//! ## Main Entry Point
//!
//! The primary entry point is the [`run`] function, which parses command-line arguments
//! and executes the appropriate subcommand.
//!
//! ## Example Usage
//!
//! ```no_run
//! use std::io;
//! let args = vec!["axiomind", "play", "--vs", "ai", "--hands", "10"];
//! let code = axiomind_cli::run(args, &mut io::stdout(), &mut io::stderr());
//! assert_eq!(code, 0);
//! ```
//!
//! ## Available Subcommands
//!
//! - `play`: Play poker hands against AI or human opponents
//! - `sim`: Run large-scale simulations and generate hand histories
//! - `stats`: Aggregate statistics from JSONL hand history files
//! - `verify`: Validate game rules and hand history integrity
//! - `replay`: Replay previously recorded hands
//! - `deal`: Deal a single hand for inspection
//! - `bench`: Benchmark hand evaluation performance
//! - `eval`: Evaluate AI policies head-to-head
//! - `export`: Convert hand histories to various formats (CSV, JSON, SQLite)
//! - `dataset`: Create training/validation/test splits for ML
//! - `cfg`: Display current configuration settings
//! - `doctor`: Run environment diagnostics
//! - `rng`: Verify RNG properties

use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashMap;
use std::io::Write;
mod commands;
mod config;
mod error;
pub mod formatters;
pub mod io_utils;
pub mod ui;
pub mod validation;

// Import utility functions from extracted modules
use commands::{
    handle_bench_command, handle_cfg_command, handle_deal_command, handle_doctor_command,
    handle_eval_command, handle_export_command, handle_play_command, handle_replay_command,
    handle_rng_command, handle_stats_command,
};
use io_utils::ensure_parent_dir;
use validation::validate_dealing_meta;

use axiomind_ai::create_ai;
use axiomind_engine::engine::{Engine, blinds_for_level};
use axiomind_engine::logger::HandRecord;
pub use error::{BatchValidationError, CliError};
use rand::{SeedableRng, seq::SliceRandom};

use std::collections::HashSet;

fn ensure_no_reopen_after_short_all_in(
    actions: &[serde_json::Value],
    big_blind: i64,
    min_chip_unit: i64,
    starting_stacks: &HashMap<String, i64>,
    hand_index: u64,
) -> Result<(), String> {
    #[derive(Clone, Copy)]
    enum ActionKind {
        Bet(i64),
        Raise(i64),
        AllIn(Option<i64>),
        Call,
        Check,
        Fold,
        Other,
    }

    let mut remaining = starting_stacks.clone();
    let mut prev_street: Option<String> = None;
    let mut street_committed: HashMap<String, i64> = HashMap::new();
    let mut current_high: i64 = 0;
    let mut last_full_raise: i64 = big_blind.max(min_chip_unit);
    let mut reopen_blocked = false;

    let extract_player_id = |act: &serde_json::Value| -> Option<String> {
        act.get("player_id")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .or_else(|| {
                act.get("player_id").and_then(|v| {
                    v.as_i64().map(|n| {
                        let candidate = format!("p{}", n);
                        if starting_stacks.contains_key(&candidate) {
                            candidate
                        } else {
                            n.to_string()
                        }
                    })
                })
            })
    };

    for (idx, act) in actions.iter().enumerate() {
        let Some(player_id) = extract_player_id(act) else {
            continue;
        };

        if !starting_stacks.contains_key(&player_id) {
            return Err(format!(
                "Unknown player {} at hand {} (action #{})",
                player_id,
                hand_index,
                idx + 1
            ));
        }

        if let Some(street) = act.get("street").and_then(|s| s.as_str())
            && prev_street.as_deref() != Some(street)
        {
            prev_street = Some(street.to_string());
            street_committed.clear();
            current_high = 0;
            last_full_raise = big_blind.max(min_chip_unit);
            reopen_blocked = false;
        }

        let action_kind: ActionKind = match act.get("action") {
            Some(serde_json::Value::Object(map)) => {
                if let Some(amount) = map.get("Bet").and_then(|v| v.as_i64()) {
                    ActionKind::Bet(amount)
                } else if let Some(amount) = map.get("Raise").and_then(|v| v.as_i64()) {
                    ActionKind::Raise(amount)
                } else if let Some(amount) = map.get("AllIn").and_then(|v| v.as_i64()) {
                    ActionKind::AllIn(Some(amount))
                } else if map.get("Call").is_some() {
                    ActionKind::Call
                } else if map.get("Check").is_some() {
                    ActionKind::Check
                } else if map.get("Fold").is_some() {
                    ActionKind::Fold
                } else {
                    ActionKind::Other
                }
            }
            Some(serde_json::Value::String(name)) => match name.to_ascii_lowercase().as_str() {
                "bet" => {
                    return Err(format!(
                        "Bet action missing amount at hand {} (action #{})",
                        hand_index,
                        idx + 1
                    ));
                }
                "raise" => {
                    return Err(format!(
                        "Bet action missing amount at hand {} (action #{})",
                        hand_index,
                        idx + 1
                    ));
                }
                "call" => ActionKind::Call,
                "check" => ActionKind::Check,
                "fold" => ActionKind::Fold,
                "allin" | "all-in" => ActionKind::AllIn(None),
                _ => ActionKind::Other,
            },
            _ => ActionKind::Other,
        };

        let commit_before = street_committed.get(&player_id).copied().unwrap_or(0);
        let mut target_commit = commit_before;

        match action_kind {
            ActionKind::Bet(amount) => {
                if amount % min_chip_unit != 0 {
                    return Err(format!(
                        "Invalid bet amount {} at hand {} (action #{})",
                        amount,
                        hand_index,
                        idx + 1
                    ));
                }
                let min_bet = big_blind.max(min_chip_unit);
                if amount < min_bet {
                    return Err(format!(
                        "Bet below minimum {} at hand {} (action #{})",
                        min_bet,
                        hand_index,
                        idx + 1
                    ));
                }
                target_commit = amount;
            }
            ActionKind::Raise(amount) => {
                if amount % min_chip_unit != 0 {
                    return Err(format!(
                        "Invalid raise amount {} at hand {} (action #{})",
                        amount,
                        hand_index,
                        idx + 1
                    ));
                }
                let min_delta = last_full_raise.max(big_blind).max(min_chip_unit);
                if amount < min_delta {
                    return Err(format!(
                        "Raise delta {} below minimum {} at hand {} (action #{})",
                        amount,
                        min_delta,
                        hand_index,
                        idx + 1
                    ));
                }
                target_commit = current_high + amount;
            }
            ActionKind::AllIn(Some(amount)) => {
                if amount % min_chip_unit != 0 {
                    return Err(format!(
                        "Invalid all-in amount {} at hand {} (action #{})",
                        amount,
                        hand_index,
                        idx + 1
                    ));
                }
                target_commit = commit_before + amount;
            }
            ActionKind::AllIn(None) => {
                let remaining_stack = remaining.get(&player_id).copied().unwrap_or(0);
                if remaining_stack % min_chip_unit != 0 {
                    return Err(format!(
                        "Invalid all-in amount {} at hand {} (action #{})",
                        remaining_stack,
                        hand_index,
                        idx + 1
                    ));
                }
                target_commit = commit_before + remaining_stack;
            }
            ActionKind::Call => {
                target_commit = current_high.max(commit_before);
            }
            ActionKind::Check | ActionKind::Fold | ActionKind::Other => {}
        }

        let mut delta_chips = target_commit.saturating_sub(commit_before);
        if let Some(rem) = remaining.get(&player_id)
            && delta_chips > *rem
        {
            delta_chips = *rem;
        }
        let new_commit = commit_before + delta_chips;

        if delta_chips > 0
            && let Some(rem) = remaining.get_mut(&player_id)
        {
            if *rem < delta_chips {
                return Err(format!(
                    "Player {} commits more chips than stack at hand {} (action #{})",
                    player_id,
                    hand_index,
                    idx + 1
                ));
            }
            *rem -= delta_chips;
        }

        let extra = new_commit.saturating_sub(current_high);
        if extra > 0 {
            if reopen_blocked {
                return Err(format!(
                    "Betting illegally reopened after short all-in at hand {} (action #{})",
                    hand_index,
                    idx + 1
                ));
            }
            let min_full_raise = last_full_raise.max(big_blind).max(min_chip_unit);
            if extra < min_full_raise {
                reopen_blocked = true;
            } else {
                last_full_raise = extra;
                reopen_blocked = false;
            }
            current_high = new_commit;
        } else {
            current_high = current_high.max(new_commit);
        }

        street_committed.insert(player_id, new_commit);
    }

    Ok(())
}

/// Represents a validation error found during hand verification
#[derive(Debug, Clone)]
struct ValidationError {
    hand_id: String,
    hand_number: usize,
    message: String,
}

impl ValidationError {
    fn new(hand_id: impl Into<String>, hand_number: usize, message: impl Into<String>) -> Self {
        Self {
            hand_id: hand_id.into(),
            hand_number,
            message: message.into(),
        }
    }
}

/// Main entry point for the CLI application.
///
/// Parses command-line arguments and dispatches to the appropriate subcommand handler.
///
/// # Arguments
///
/// * `args` - Iterator over command-line arguments (typically `std::env::args()`)
/// * `out` - Output stream for normal output (typically `stdout`)
/// * `err` - Output stream for error messages (typically `stderr`)
///
/// # Returns
///
/// Exit code: `0` for success, `2` for errors, `130` for interruptions
///
/// # Example
///
/// ```
/// use std::io;
/// let args = vec!["axiomind", "deal", "--seed", "42"];
/// let code = axiomind_cli::run(args, &mut io::stdout(), &mut io::stderr());
/// assert_eq!(code, 0);
/// ```
///
/// # Available Commands
///
/// - `play --vs {ai|human} --hands N`: Play N hands against AI or human
/// - `sim --hands N --output FILE`: Simulate N hands and save to FILE
/// - `stats --input PATH`: Display statistics from hand history files
/// - `verify --input PATH`: Validate hand history integrity
/// - `replay --input FILE`: Replay recorded hands
/// - `deal --seed N`: Deal a single hand with optional seed
/// - `bench`: Benchmark hand evaluation performance
/// - `eval --ai-a A --ai-b B --hands N`: Compare two AI policies
/// - `export --input IN --format FMT --output OUT`: Convert hand histories
/// - `dataset --input IN --outdir DIR`: Split data for training
/// - `cfg`: Display configuration settings
/// - `doctor`: Run environment diagnostics
/// - `rng --seed N`: Test RNG output
pub fn run<I, S>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fn strip_utf8_bom(s: &mut String) {
        const UTF8_BOM: &str = "\u{feff}";
        if s.starts_with(UTF8_BOM) {
            s.drain(..UTF8_BOM.len());
        }
    }

    fn read_text_auto(path: &str) -> Result<String, String> {
        let mut content = if path.ends_with(".zst") {
            // Read entire compressed file then decompress; more portable across platforms
            let comp = std::fs::read(path).map_err(|e| e.to_string())?;
            // Use a conservative initial capacity; zstd will grow as needed
            let dec = zstd::bulk::decompress(&comp, 8 * 1024 * 1024).map_err(|e| e.to_string())?;
            String::from_utf8(dec).map_err(|e| e.to_string())?
        } else {
            std::fs::read_to_string(path).map_err(|e| e.to_string())?
        };
        strip_utf8_bom(&mut content);
        Ok(content)
    }

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

    fn dataset_stream_if_needed(
        input: &str,
        outdir: &str,
        train: Option<f64>,
        val: Option<f64>,
        test: Option<f64>,
        seed: Option<u64>,
        err: &mut dyn Write,
    ) -> Result<Option<()>, CliError> {
        use std::io::{BufRead, BufReader, BufWriter};

        let threshold = std::env::var("axiomind_DATASET_STREAM_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(10_000);
        if threshold == 0 {
            return Ok(None);
        }

        let trace_stream = std::env::var("axiomind_DATASET_STREAM_TRACE")
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);

        let count_file = match std::fs::File::open(input) {
            Ok(f) => f,
            Err(e) => {
                ui::write_error(err, &format!("Failed to read {}: {}", input, e))?;
                return Err(CliError::Io(e));
            }
        };

        let mut record_count = 0usize;
        {
            let reader = BufReader::new(count_file);
            let mut first_line = true;
            for line in reader.lines() {
                match line {
                    Ok(mut line) => {
                        if first_line {
                            strip_utf8_bom(&mut line);
                            first_line = false;
                        }
                        if !line.trim().is_empty() {
                            record_count += 1;
                        }
                    }
                    Err(e) => {
                        ui::write_error(err, &format!("Failed to read {}: {}", input, e))?;
                        return Err(CliError::Io(e));
                    }
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

        let splits = match compute_splits(train, val, test) {
            Ok(v) => v,
            Err(msg) => {
                ui::write_error(err, &msg)?;
                return Err(CliError::InvalidInput(msg));
            }
        };

        let tr = splits[0];
        let va = splits[1];
        let n = record_count;
        let n_tr = ((tr * n as f64).round() as usize).min(n);
        let n_va = ((va * n as f64).round() as usize).min(n.saturating_sub(n_tr));

        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
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

        if let Err(e) = std::fs::create_dir_all(outdir) {
            ui::write_error(
                err,
                &format!("Failed to create directory {}: {}", outdir, e),
            )?;
            return Err(CliError::Io(e));
        }

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

        let mut create_writer =
            |path: &std::path::Path| -> Result<BufWriter<std::fs::File>, CliError> {
                match std::fs::File::create(path) {
                    Ok(f) => Ok(BufWriter::new(f)),
                    Err(e) => {
                        ui::write_error(
                            err,
                            &format!("Failed to create {}: {}", path.display(), e),
                        )?;
                        Err(CliError::Io(e))
                    }
                }
            };

        let mut train_writer = create_writer(&train_path)?;
        let mut val_writer = create_writer(&val_path)?;
        let mut test_writer = create_writer(&test_path)?;

        let data_file = match std::fs::File::open(input) {
            Ok(f) => f,
            Err(e) => {
                ui::write_error(err, &format!("Failed to read {}: {}", input, e))?;
                return Err(CliError::Io(e));
            }
        };
        let reader = BufReader::new(data_file);
        let mut record_idx = 0usize;
        let mut first_line = true;

        for (line_idx, line_res) in reader.lines().enumerate() {
            let mut line = match line_res {
                Ok(line) => line,
                Err(e) => {
                    ui::write_error(err, &format!("Failed to read {}: {}", input, e))?;
                    return Err(CliError::Io(e));
                }
            };
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::io;
    /// let input = "data/hands/sample.jsonl";
    /// let code = axiomind_cli::run(
    ///     vec!["axiomind", "stats", "--input", input],
    ///     &mut io::stdout(),
    ///     &mut io::stderr()
    /// );
    /// assert_eq!(code, 0);
    /// ```
    const COMMANDS: &[&str] = &[
        "play", "replay", "stats", "verify", "deal", "bench", "sim", "eval", "export", "dataset",
        "cfg", "doctor", "rng",
    ];
    let argv: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();

    let parsed = AxiomindCli::try_parse_from(&argv);
    match parsed {
        Err(e) => {
            use clap::error::ErrorKind;

            // Help and version should print to stdout and exit 0
            match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    if write!(out, "{}", e).is_err() {
                        return 2;
                    }
                    0
                }
                _ => {
                    // Print clap error first
                    if writeln!(err, "{}", e).is_err()
                        || writeln!(err).is_err()
                        || writeln!(err, "Axiomind Poker CLI").is_err()
                        || writeln!(err, "Usage: axiomind <command> [options]\n").is_err()
                        || writeln!(err, "Commands:").is_err()
                    {
                        return 2;
                    }
                    for c in COMMANDS {
                        if writeln!(err, "  {}", c).is_err() {
                            return 2;
                        }
                    }
                    if writeln!(err, "\nFor full help, run: axiomind --help").is_err() {
                        return 2;
                    }
                    2
                }
            }
        }
        Ok(cli) => match cli.cmd {
            Commands::Cfg => match handle_cfg_command(out, err) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Play {
                vs,
                hands,
                seed,
                level,
            } => {
                // Use stdin for real input (supports both TTY and piped stdin)
                let stdin = std::io::stdin();
                let mut stdin_lock = stdin.lock();
                match handle_play_command(vs, hands, seed, level, out, err, &mut stdin_lock) {
                    Ok(()) => 0,
                    Err(e) => {
                        if writeln!(err, "Error: {}", e).is_err() {
                            return 2;
                        }
                        2
                    }
                }
            }
            Commands::Replay { input, speed } => {
                match handle_replay_command(input, speed, out, err) {
                    Ok(()) => 0,
                    Err(_) => 2,
                }
            }
            Commands::Stats { input } => match handle_stats_command(input, out, err) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Verify { input } => {
                // Enhanced verification with error collection and comprehensive validations
                let mut errors: Vec<ValidationError> = Vec::new();
                let mut hands = 0u64;
                let mut game_over = false;
                let mut stacks_after_hand: HashMap<String, i64> = HashMap::new();
                const MIN_CHIP_UNIT: i64 = 25;
                let Some(path) = input else {
                    let _ = ui::write_error(err, "input required");
                    return 2;
                };
                let valid_id = |s: &str| -> bool {
                    s.len() == 15
                        && s[0..8].chars().all(|c| c.is_ascii_digit())
                        && &s[8..9] == "-"
                        && s[9..].chars().all(|c| c.is_ascii_digit())
                };
                match read_text_auto(&path) {
                    Ok(content) => {
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            hands += 1;
                            if game_over {
                                errors.push(ValidationError::new(
                                    String::new(),
                                    hands as usize,
                                    format!(
                                        "Hand {} recorded after player elimination (zero stack)",
                                        hands
                                    ),
                                ));
                                continue;
                            }
                            // parse as Value first to validate optional net_result chip conservation
                            let v: serde_json::Value = match serde_json::from_str(line) {
                                Ok(v) => v,
                                Err(_) => {
                                    errors.push(ValidationError::new(
                                        String::new(),
                                        hands as usize,
                                        "Invalid JSON record",
                                    ));
                                    continue;
                                }
                            };

                            // Extract hand_id early for better error reporting
                            let hand_id = v
                                .get("hand_id")
                                .and_then(|h| h.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            // Level 1 Validation: Check for missing result field
                            let has_result = v.get("result").map(|r| !r.is_null()).unwrap_or(false);
                            if !has_result {
                                errors.push(ValidationError::new(
                                    hand_id.clone(),
                                    hands as usize,
                                    "Missing or null result field",
                                ));
                            }

                            // Level 2 Validation: Action sequence and street progression
                            if let Some(actions) = v.get("actions").and_then(|a| a.as_array()) {
                                let mut prev_street: Option<String> = None;
                                let streets_order = ["Preflop", "Flop", "Turn", "River"];
                                let mut street_index = 0;

                                for action in actions {
                                    if let Some(street_str) =
                                        action.get("street").and_then(|s| s.as_str())
                                    {
                                        let current_street = street_str.to_string();

                                        if let Some(ref prev) = prev_street {
                                            if prev != &current_street {
                                                // Street changed, validate progression
                                                if let Some(current_idx) = streets_order
                                                    .iter()
                                                    .position(|s| s == &current_street)
                                                {
                                                    if current_idx <= street_index {
                                                        errors.push(ValidationError::new(
                                                            hand_id.clone(),
                                                            hands as usize,
                                                            format!("Invalid street progression: {} appears after {}", current_street, prev),
                                                        ));
                                                    } else if current_idx != street_index + 1 {
                                                        errors.push(ValidationError::new(
                                                            hand_id.clone(),
                                                            hands as usize,
                                                            format!("Street skipped: jumped from {} to {}", prev, current_street),
                                                        ));
                                                    }
                                                    street_index = current_idx;
                                                }
                                            }
                                        } else {
                                            // First street
                                            if let Some(idx) = streets_order
                                                .iter()
                                                .position(|s| s == &current_street)
                                            {
                                                street_index = idx;
                                            }
                                        }
                                        prev_street = Some(current_street);
                                    }
                                }
                            }

                            let mut starting_stacks: Option<HashMap<String, i64>> = None;
                            if let Some(players) = v.get("players").and_then(|p| p.as_array()) {
                                let mut start_map = HashMap::new();
                                for player in players {
                                    let Some(id) = player.get("id").and_then(|x| x.as_str()) else {
                                        continue;
                                    };
                                    let stack = player
                                        .get("stack_start")
                                        .and_then(|x| x.as_i64())
                                        .unwrap_or(0);
                                    start_map.insert(id.to_string(), stack);
                                }
                                let prev_state = if stacks_after_hand.is_empty() {
                                    None
                                } else {
                                    Some(&stacks_after_hand)
                                };

                                // Collect roster state errors
                                if let Some(prev_map) = prev_state {
                                    for (id, stack_start) in &start_map {
                                        if let Some(prev_stack) = prev_map.get(id) {
                                            if *prev_stack != *stack_start {
                                                errors.push(ValidationError::new(
                                                    hand_id.clone(),
                                                    hands as usize,
                                                    format!(
                                                        "Stack mismatch for {} at hand {}",
                                                        id, hands
                                                    ),
                                                ));
                                            }
                                            if *prev_stack <= 0 {
                                                errors.push(ValidationError::new(
                                                    hand_id.clone(),
                                                    hands as usize,
                                                    format!(
                                                        "Player {} reappeared after elimination",
                                                        id
                                                    ),
                                                ));
                                            }
                                        } else {
                                            errors.push(ValidationError::new(
                                                hand_id.clone(),
                                                hands as usize,
                                                format!(
                                                    "Unexpected player {} at hand {}",
                                                    id, hands
                                                ),
                                            ));
                                        }
                                    }
                                    for (id, prev_stack) in prev_map {
                                        if !start_map.contains_key(id) && *prev_stack > 0 {
                                            errors.push(ValidationError::new(
                                                hand_id.clone(),
                                                hands as usize,
                                                format!("Missing player {} at hand {}", id, hands),
                                            ));
                                        }
                                    }
                                }
                                for (id, stack_start) in &start_map {
                                    if *stack_start <= 0 {
                                        errors.push(ValidationError::new(
                                            hand_id.clone(),
                                            hands as usize,
                                            format!(
                                                "Player {} has non-positive starting stack",
                                                id
                                            ),
                                        ));
                                    }
                                }

                                starting_stacks = Some(start_map.clone());
                                stacks_after_hand = start_map;
                                if let Some(nr_obj) =
                                    v.get("net_result").and_then(|x| x.as_object())
                                {
                                    for id in nr_obj.keys() {
                                        if !stacks_after_hand.contains_key(id) {
                                            errors.push(ValidationError::new(
                                                hand_id.clone(),
                                                hands as usize,
                                                format!("Unknown player {} in net_result", id),
                                            ));
                                        }
                                    }
                                }
                            }
                            if let (Some(start_map), Some(meta_obj)) = (
                                starting_stacks.as_ref(),
                                v.get("meta").and_then(|m| m.as_object()),
                            ) {
                                let button_id = v.get("button").and_then(|b| b.as_str());
                                if let Err(msg) =
                                    validate_dealing_meta(meta_obj, button_id, start_map, hands)
                                {
                                    errors.push(ValidationError::new(
                                        hand_id.clone(),
                                        hands as usize,
                                        &msg,
                                    ));
                                }
                            }
                            let mut big_blind = MIN_CHIP_UNIT;
                            if let Some(blinds_val) = v.get("blinds") {
                                if let Some(bb) = blinds_val.get("bb").and_then(|x| x.as_i64()) {
                                    big_blind = bb;
                                } else if let Some(arr) = blinds_val.as_array()
                                    && arr.len() >= 2
                                    && let Some(bb) = arr[1].as_i64()
                                {
                                    big_blind = bb;
                                }
                            }
                            if big_blind < MIN_CHIP_UNIT {
                                big_blind = MIN_CHIP_UNIT;
                            }
                            if let Some(actions) = v.get("actions").and_then(|a| a.as_array())
                                && let Some(ref start_map) = starting_stacks
                                && let Err(msg) = ensure_no_reopen_after_short_all_in(
                                    actions,
                                    big_blind,
                                    MIN_CHIP_UNIT,
                                    start_map,
                                    hands,
                                )
                            {
                                errors.push(ValidationError::new(
                                    hand_id.clone(),
                                    hands as usize,
                                    &msg,
                                ));
                            }
                            if let Some(nr) = v.get("net_result").and_then(|x| x.as_object()) {
                                let mut sum: i64 = 0;
                                for val in nr.values() {
                                    if let Some(n) = val.as_i64() {
                                        sum += n;
                                    }
                                }
                                if sum != 0 {
                                    errors.push(ValidationError::new(
                                        hand_id.clone(),
                                        hands as usize,
                                        "Chip conservation violated",
                                    ));
                                }
                                for (id, delta) in nr.iter() {
                                    if let Some(val) = delta.as_i64() {
                                        let entry =
                                            stacks_after_hand.entry(id.clone()).or_insert(0);
                                        *entry += val;
                                    }
                                }
                                if stacks_after_hand.values().any(|stack| *stack <= 0) {
                                    game_over = true;
                                }
                            }
                            match serde_json::from_value::<axiomind_engine::logger::HandRecord>(
                                v.clone(),
                            ) {
                                Ok(rec) => {
                                    if rec.board.len() != 5 {
                                        errors.push(ValidationError::new(
                                            hand_id.clone(),
                                            hands as usize,
                                            format!(
                                                "Invalid board length: expected 5 cards but found {}",
                                                rec.board.len()
                                            ),
                                        ));
                                    }

                                    let mut seen_cards: HashSet<axiomind_engine::cards::Card> =
                                        HashSet::new();
                                    let mut duplicate_cards: HashSet<axiomind_engine::cards::Card> =
                                        HashSet::new();
                                    {
                                        let mut record_card =
                                            |card: axiomind_engine::cards::Card| {
                                                if !seen_cards.insert(card) {
                                                    duplicate_cards.insert(card);
                                                }
                                            };
                                        for card in &rec.board {
                                            record_card(*card);
                                        }
                                        if let Some(players) =
                                            v.get("players").and_then(|p| p.as_array())
                                        {
                                            for player in players {
                                                let pid = player
                                                    .get("id")
                                                    .and_then(|x| x.as_str())
                                                    .unwrap_or("unknown");
                                                if let Some(hole_cards) = player
                                                    .get("hole_cards")
                                                    .and_then(|h| h.as_array())
                                                {
                                                    for card_val in hole_cards {
                                                        match serde_json::from_value::<
                                                            axiomind_engine::cards::Card,
                                                        >(
                                                            card_val.clone()
                                                        ) {
                                                            Ok(card) => record_card(card),
                                                            Err(_) => {
                                                                errors.push(ValidationError::new(
                                                                    hand_id.clone(),
                                                                    hands as usize,
                                                                    format!(
                                                                        "Invalid card specification for {}",
                                                                        pid
                                                                    ),
                                                                ));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if !duplicate_cards.is_empty() {
                                        let mut cards: Vec<String> = duplicate_cards
                                            .iter()
                                            .map(|card| format!("{:?} {:?}", card.rank, card.suit))
                                            .collect();
                                        cards.sort();
                                        errors.push(ValidationError::new(
                                            hand_id.clone(),
                                            hands as usize,
                                            format!(
                                                "Duplicate card(s) detected: {}",
                                                cards.join(", ")
                                            ),
                                        ));
                                    }

                                    if !valid_id(&rec.hand_id) {
                                        errors.push(ValidationError::new(
                                            hand_id.clone(),
                                            hands as usize,
                                            "Invalid hand_id format",
                                        ));
                                    }
                                }
                                Err(_) => {
                                    errors.push(ValidationError::new(
                                        hand_id.clone(),
                                        hands as usize,
                                        "Invalid record structure",
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if ui::write_error(err, &format!("Failed to read {}: {}", path, e)).is_err()
                        {
                            return 2;
                        }
                        return 2;
                    }
                }

                // Output results
                if errors.is_empty() {
                    if writeln!(out, "Verify: OK (hands={})", hands).is_err() {
                        return 2;
                    }
                    0
                } else {
                    if writeln!(out, "Verify: FAIL (hands={})", hands).is_err()
                        || writeln!(err).is_err()
                        || writeln!(err, "Errors found:").is_err()
                    {
                        return 2;
                    }
                    for error in &errors {
                        let result = if error.hand_id.is_empty() {
                            writeln!(err, "  Hand {}: {}", error.hand_number, error.message)
                        } else {
                            writeln!(err, "  Hand {}: {}", error.hand_id, error.message)
                        };
                        if result.is_err() {
                            return 2;
                        }
                    }
                    if writeln!(err).is_err() {
                        return 2;
                    }
                    let invalid_hand_numbers: HashSet<usize> =
                        errors.iter().map(|e| e.hand_number).collect();
                    let invalid_hands = invalid_hand_numbers.len() as u64;
                    let percentage = if hands > 0 {
                        (invalid_hands as f64 / hands as f64 * 100.0).round() as u32
                    } else {
                        0
                    };
                    if writeln!(
                        err,
                        "Summary: {} error(s) in {} hands ({} invalid hands, {}% invalid)",
                        errors.len(),
                        hands,
                        invalid_hands,
                        percentage
                    )
                    .is_err()
                    {
                        return 2;
                    }
                    2
                }
            }
            Commands::Doctor => match handle_doctor_command(out, err) {
                Ok(()) => 0,
                Err(_) => 2,
            },
            Commands::Eval {
                ai_a,
                ai_b,
                hands,
                seed,
            } => match handle_eval_command(&ai_a, &ai_b, hands, seed, out) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Bench => match handle_bench_command(out) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Deal { seed } => match handle_deal_command(seed, out) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Rng { seed } => match handle_rng_command(seed, out) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Sim {
                hands,
                output,
                seed,
                level,
                resume,
            } => {
                let total: usize = hands as usize;
                if total == 0 {
                    if ui::write_error(err, "hands must be >= 1").is_err() {
                        return 2;
                    }
                    return 2;
                }
                let level = level.unwrap_or(1).clamp(1, 20);
                let mut completed = 0usize;
                let mut path = None;
                if let Some(outp) = output.clone() {
                    path = Some(std::path::PathBuf::from(outp));
                }
                // resume: count existing unique hand_ids and warn on duplicates
                if let Some(res) = resume.as_ref() {
                    let contents = std::fs::read_to_string(res).unwrap_or_default();
                    let mut seen = std::collections::HashSet::new();
                    let mut dups = 0usize;
                    for line in contents.lines().filter(|l| !l.trim().is_empty()) {
                        let hid = serde_json::from_str::<serde_json::Value>(line)
                            .ok()
                            .and_then(|v| {
                                v.get("hand_id")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string())
                            })
                            .unwrap_or_default();
                        if hid.is_empty() {
                            continue;
                        }
                        if !seen.insert(hid) {
                            dups += 1;
                        }
                    }
                    completed = seen.len();
                    path = Some(std::path::PathBuf::from(res));
                    if dups > 0
                        && writeln!(err, "Warning: {} duplicate hand_id(s) skipped", dups).is_err()
                    {
                        return 2;
                    }
                    if writeln!(out, "Resumed from {}", completed).is_err() {
                        return 2;
                    }
                }
                let base_seed = seed.unwrap_or_else(rand::random);
                let mut eng = Engine::new(Some(base_seed), level);
                eng.shuffle();
                let break_after = std::env::var("axiomind_SIM_BREAK_AFTER")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok());
                let per_hand_delay = std::env::var("axiomind_SIM_SLEEP_MICROS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(std::time::Duration::from_micros);
                let fast_mode = std::env::var("axiomind_SIM_FAST")
                    .map(|v| {
                        matches!(
                            v.trim().to_ascii_lowercase().as_str(),
                            "1" | "true" | "yes" | "on"
                        )
                    })
                    .unwrap_or(false);
                if !fast_mode {
                    let _ = &per_hand_delay;
                }
                if fast_mode {
                    return match sim_run_fast(
                        total,
                        level,
                        seed,
                        base_seed,
                        break_after,
                        per_hand_delay,
                        completed,
                        path.as_deref(),
                        out,
                        err,
                    ) {
                        Ok(()) => 0,
                        Err(CliError::Interrupted(_)) => 130,
                        Err(_) => 2,
                    };
                }
                #[allow(clippy::mut_range_bound)]
                for i in completed..total {
                    // create a fresh engine per hand to avoid residual hole cards
                    let mut e = Engine::new(Some(base_seed + i as u64), level);
                    e.shuffle();
                    let _ = e.deal_hand();

                    // Play the hand to completion
                    let (actions, result, showdown) = play_hand_to_completion(&mut e);

                    if let Some(p) = &path {
                        if let Err(e) = ensure_parent_dir(p) {
                            if ui::write_error(err, &e).is_err() {
                                return 2;
                            }
                            return 2;
                        }

                        let mut f = match std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(p)
                        {
                            Ok(file) => file,
                            Err(e) => {
                                if ui::write_error(
                                    err,
                                    &format!("Failed to open output file: {}", e),
                                )
                                .is_err()
                                {
                                    return 2;
                                }
                                return 2;
                            }
                        };
                        let hand_id = format!("19700101-{:06}", i + 1);
                        let board = e.board().clone();
                        let rec = serde_json::json!({
                            "hand_id": hand_id,
                            "seed": base_seed + i as u64,
                            "level": level,
                            "actions": actions,
                            "board": board,
                            "result": result,
                            "ts": "1970-01-01T00:00:00+00:00".to_string(),
                            "meta": null,
                            "showdown": showdown
                        });
                        let json_str = match serde_json::to_string(&rec) {
                            Ok(s) => s,
                            Err(e) => {
                                if ui::write_error(err, &format!("Failed to serialize hand: {}", e))
                                    .is_err()
                                {
                                    return 2;
                                }
                                return 2;
                            }
                        };
                        if writeln!(f, "{}", json_str).is_err() {
                            if ui::write_error(err, "Failed to write hand to file").is_err() {
                                return 2;
                            }
                            return 2;
                        }
                    }
                    completed += 1;
                    if let Some(b) = break_after
                        && completed == b
                    {
                        if writeln!(out, "Interrupted: saved {}/{}", completed, total).is_err() {
                            return 2;
                        }
                        return 130;
                    }
                }
                if writeln!(out, "Simulated: {} hands", completed).is_err() {
                    return 2;
                }
                0
            }
            Commands::Export {
                input,
                format,
                output,
            } => match handle_export_command(input, output, format, out, err) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Dataset {
                input,
                outdir,
                train,
                val,
                test,
                seed,
            } => {
                match dataset_stream_if_needed(&input, &outdir, train, val, test, seed, err) {
                    Ok(Some(())) => return 0,
                    Ok(None) => { /* Continue with normal processing */ }
                    Err(_) => return 2,
                }
                let content = match std::fs::read_to_string(&input) {
                    Ok(c) => c,
                    Err(e) => {
                        if ui::write_error(err, &format!("Failed to read {}: {}", input, e))
                            .is_err()
                        {
                            return 2;
                        }
                        return 2;
                    }
                };
                let mut lines: Vec<String> = content
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|s| s.to_string())
                    .collect();
                let n = lines.len();
                if n == 0 {
                    if ui::write_error(err, "Empty input").is_err() {
                        return 2;
                    }
                    return 2;
                }
                let splits = match compute_splits(train, val, test) {
                    Ok(v) => v,
                    Err(msg) => {
                        if ui::write_error(err, &msg).is_err() {
                            return 2;
                        }
                        return 2;
                    }
                };
                let tr = splits[0];
                let va = splits[1];
                let te = splits[2];
                let sum = tr + va + te;
                if (sum - 1.0).abs() > 1e-6 {
                    if ui::write_error(err, "Splits must sum to 100% (1.0 total)").is_err() {
                        return 2;
                    }
                    return 2;
                }
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
                lines.shuffle(&mut rng);
                let n_tr = ((tr * n as f64).round() as usize).min(n);
                let n_va = ((va * n as f64).round() as usize).min(n.saturating_sub(n_tr));
                let _n_te = n.saturating_sub(n_tr + n_va);
                for (idx, raw) in lines.iter().enumerate() {
                    let trimmed = raw.trim();
                    if let Err(e) =
                        serde_json::from_str::<axiomind_engine::logger::HandRecord>(trimmed)
                    {
                        if ui::write_error(
                            err,
                            &format!("Invalid record at line {}: {}", idx + 1, e),
                        )
                        .is_err()
                        {
                            return 2;
                        }
                        return 2;
                    }
                }
                let (trv, rest) = lines.split_at(n_tr);
                let (vav, tev) = rest.split_at(n_va);
                let out_root = std::path::Path::new(&outdir);
                if let Err(e) = std::fs::create_dir_all(out_root) {
                    if ui::write_error(
                        err,
                        &format!("Failed to create directory {}: {}", outdir, e),
                    )
                    .is_err()
                    {
                        return 2;
                    }
                    return 2;
                }
                let mut write_split = |name: &str, data: &[String]| -> Result<(), ()> {
                    let path = out_root.join(name);
                    let file = match std::fs::File::create(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            let _ = ui::write_error(
                                err,
                                &format!("Failed to create {}: {}", path.display(), e),
                            );
                            return Err(());
                        }
                    };
                    let mut writer = std::io::BufWriter::new(file);
                    for l in data {
                        if let Err(e) = writeln!(writer, "{}", l) {
                            let _ = ui::write_error(
                                err,
                                &format!("Failed to write {}: {}", path.display(), e),
                            );
                            return Err(());
                        }
                    }
                    Ok(())
                };
                if write_split("train.jsonl", trv).is_err() {
                    return 2;
                }
                if write_split("val.jsonl", vav).is_err() {
                    return 2;
                }
                if write_split("test.jsonl", tev).is_err() {
                    return 2;
                }
                0
            }
        },
    }
}

#[allow(clippy::too_many_arguments, clippy::mut_range_bound)]
/// Helper function to play out a complete hand with AI opponents
/// Returns (actions, result_string, showdown_info_option)
fn play_hand_to_completion(
    engine: &mut Engine,
) -> (
    Vec<axiomind_engine::logger::ActionRecord>,
    String,
    Option<serde_json::Value>,
) {
    use axiomind_engine::cards::Card;
    use axiomind_engine::hand::{compare_hands, evaluate_hand};

    let ai = create_ai("baseline");

    // Play through the hand
    while let Ok(current_player) = engine.current_player() {
        let action = ai.get_action(engine, current_player);

        match engine.apply_action(current_player, action) {
            Ok(state) if state.is_hand_complete() => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }

    // Get action history
    let actions = engine.action_history();

    // Determine winner
    let (result_string, showdown) = if let Some(folded) = engine.folded_player() {
        // Someone folded - other player wins
        let winner = 1 - folded;
        let pot = engine.pot();
        (format!("Player {} wins {} (fold)", winner, pot), None)
    } else if engine.reached_showdown() {
        // Evaluate hands at showdown
        let players = engine.players();
        let board = engine.community_cards();

        // Build 7-card hands for each player
        let mut player0_cards = Vec::new();
        let mut player1_cards = Vec::new();

        if let [Some(c1), Some(c2)] = players[0].hole_cards() {
            player0_cards.push(c1);
            player0_cards.push(c2);
        }
        if let [Some(c1), Some(c2)] = players[1].hole_cards() {
            player1_cards.push(c1);
            player1_cards.push(c2);
        }

        player0_cards.extend_from_slice(&board);
        player1_cards.extend_from_slice(&board);

        if player0_cards.len() == 7 && player1_cards.len() == 7 {
            let hand0: [Card; 7] = player0_cards.try_into().unwrap();
            let hand1: [Card; 7] = player1_cards.try_into().unwrap();

            let strength0 = evaluate_hand(&hand0);
            let strength1 = evaluate_hand(&hand1);

            let pot = engine.pot();
            let comparison = compare_hands(&strength0, &strength1);

            use std::cmp::Ordering;
            let (result_str, showdown_info) = match comparison {
                Ordering::Greater => (
                    format!("Player 0 wins {} (showdown)", pot),
                    Some(serde_json::json!({"winners": [0]})),
                ),
                Ordering::Less => (
                    format!("Player 1 wins {} (showdown)", pot),
                    Some(serde_json::json!({"winners": [1]})),
                ),
                Ordering::Equal => (
                    format!("Split pot {} (tie)", pot),
                    Some(serde_json::json!({"winners": [0, 1]})),
                ),
            };
            (result_str, showdown_info)
        } else {
            ("Unknown result".to_string(), None)
        }
    } else {
        ("Hand incomplete".to_string(), None)
    };

    (actions, result_string, showdown)
}

#[allow(clippy::too_many_arguments, clippy::mut_range_bound)]
fn sim_run_fast(
    total: usize,
    level: u8,
    _seed: Option<u64>,
    base_seed: u64,
    break_after: Option<usize>,
    per_hand_delay: Option<std::time::Duration>,
    mut completed: usize,
    path: Option<&std::path::Path>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    let mut writer = match path {
        Some(p) => {
            if let Err(e) = ensure_parent_dir(p) {
                ui::write_error(err, &e)?;
                return Err(CliError::Io(std::io::Error::other(e)));
            }

            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
            {
                Ok(file) => Some(std::io::BufWriter::new(file)),
                Err(e) => {
                    ui::write_error(err, &format!("Failed to open {}: {}", p.display(), e))?;
                    return Err(CliError::Io(e));
                }
            }
        }
        None => None,
    };

    for i in completed..total {
        let mut engine = Engine::new(Some(base_seed + i as u64), level);
        engine.shuffle();
        let _ = engine.deal_hand();

        // Play the hand to completion
        let (actions, result, showdown) = play_hand_to_completion(&mut engine);

        if let Some(w) = writer.as_mut() {
            let hand_id = format!("19700101-{:06}", i + 1);
            let board = engine.board().clone();
            let record = serde_json::json!({
                "hand_id": hand_id,
                "seed": base_seed + i as u64,
                "level": level,
                "actions": actions,
                "board": board,
                "result": result,
                "ts": "1970-01-01T00:00:00+00:00".to_string(),
                "meta": null,
                "showdown": showdown
            });
            if let Err(e) = writeln!(w, "{}", serde_json::to_string(&record).unwrap()) {
                ui::write_error(err, "Failed to write simulation output")?;
                return Err(CliError::Io(e));
            }
        }

        completed += 1;

        if let Some(delay) = per_hand_delay {
            std::thread::sleep(delay);
        }

        if let Some(b) = break_after
            && completed == b
        {
            if let Some(w) = writer.as_mut()
                && let Err(e) = w.flush()
            {
                ui::write_error(err, "Failed to flush simulation output")?;
                return Err(CliError::Io(e));
            }
            writeln!(out, "Interrupted: saved {}/{}", completed, total)?;
            return Err(CliError::Interrupted(format!(
                "Interrupted: saved {}/{}",
                completed, total
            )));
        }
    }

    if let Some(mut w) = writer
        && let Err(e) = w.flush()
    {
        ui::write_error(err, "Failed to flush simulation output")?;
        return Err(CliError::Io(e));
    }

    writeln!(out, "Simulated: {} hands", completed)?;
    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    name = "axiomind",
    author = "Axiomind",
    version,
    about = "Axiomind Poker CLI"
)]
struct AxiomindCli {
    #[command(subcommand)]
    cmd: Commands,
}

/// Available CLI subcommands.
///
/// Each variant represents a distinct operation mode of the Axiomind CLI.
/// Commands are designed to be composable: output from one command can often
/// be used as input to another (e.g., `sim` generates data, `stats` analyzes it).
#[derive(Subcommand, Debug)]
enum Commands {
    /// Play poker hands interactively or against AI.
    ///
    /// Run one or more hands of heads-up Texas Hold'em. In AI mode, the opponent
    /// makes automatic decisions. In human mode, the user is prompted for actions.
    ///
    /// # Options
    ///
    /// * `--vs` - Opponent type: `ai` or `human`
    /// * `--hands` - Number of hands to play (default: 1)
    /// * `--seed` - RNG seed for reproducibility (default: random)
    /// * `--level` - Blind level (1-20, higher means bigger blinds; levels 21+ treated as level 20)
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind play --vs ai --hands 10 --seed 42 --level 2
    /// ```
    Play {
        #[arg(long, value_enum)]
        vs: Vs,
        #[arg(long)]
        hands: Option<u32>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=20))]
        level: Option<u8>,
    },
    /// Replay previously recorded hands from a JSONL file.
    ///
    /// Load and replay hand histories, optionally controlling playback speed.
    ///
    /// # Options
    ///
    /// * `--input` - Path to JSONL file containing hand histories
    /// * `--speed` - Playback speed multiplier (default: 1.0, must be > 0)
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind replay --input data/hands/session.jsonl --speed 2.0
    /// ```
    Replay {
        #[arg(long)]
        input: String,
        #[arg(long)]
        speed: Option<f64>,
    },
    /// Aggregate statistics from hand history files.
    ///
    /// Compute summary statistics including total hands, win rates, and chip conservation
    /// validation from JSONL files or directories.
    ///
    /// # Options
    ///
    /// * `--input` - Path to JSONL file or directory
    ///
    /// # Output Format
    ///
    /// JSON object with:
    /// - `hands`: Total number of hands
    /// - `winners`: Win counts per player
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind stats --input data/hands/
    /// ```
    Stats {
        #[arg(long)]
        input: String,
    },
    /// Evaluate AI policies head-to-head.
    ///
    /// Compare two AI models by simulating N hands and reporting win rates.
    ///
    /// # Options
    ///
    /// * `--ai-a` - First AI policy identifier
    /// * `--ai-b` - Second AI policy identifier
    /// * `--hands` - Number of hands to simulate
    /// * `--seed` - RNG seed for reproducibility
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind eval --ai-a baseline --ai-b experimental --hands 1000 --seed 42
    /// ```
    Eval {
        #[arg(long, name = "ai-a")]
        ai_a: String,
        #[arg(long, name = "ai-b")]
        ai_b: String,
        #[arg(long)]
        hands: u32,
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Validate hand history integrity and game rules.
    ///
    /// Perform comprehensive validation checks on hand histories:
    /// - Board completeness (exactly 5 cards)
    /// - No duplicate cards
    /// - Chip conservation (net_result sums to zero)
    /// - Valid hand IDs
    /// - Betting rules compliance (no illegal reopening after short all-in)
    /// - Player roster consistency across hands
    ///
    /// # Options
    ///
    /// * `--input` - Path to JSONL file to verify
    ///
    /// # Returns
    ///
    /// Exit code 0 if all checks pass, 2 if any violations detected.
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind verify --input data/hands/session.jsonl
    /// ```
    Verify {
        #[arg(long)]
        input: Option<String>,
    },
    /// Deal a single hand for inspection.
    ///
    /// Generate and display hole cards for both players and the full board.
    /// Useful for debugging, testing, or manual game setup.
    ///
    /// # Options
    ///
    /// * `--seed` - RNG seed for reproducible deals (default: random)
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind deal --seed 12345
    /// ```
    Deal {
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Benchmark hand evaluation performance.
    ///
    /// Evaluates 200 random 7-card hands and reports execution time.
    /// Used for performance regression testing and optimization validation.
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind bench
    /// ```
    Bench,
    /// Run large-scale hand simulations.
    ///
    /// Generate and optionally record N hands of poker. Supports resuming from
    /// previous runs and breaking early for testing.
    ///
    /// # Options
    ///
    /// * `--hands` - Total number of hands to simulate
    /// * `--output` - Path to save hand histories (JSONL format)
    /// * `--seed` - Base RNG seed (each hand uses seed + hand_index)
    /// * `--level` - Blind level (1-20, higher means bigger blinds; levels 21+ treated as level 20)
    /// * `--resume` - Resume from existing JSONL file (skips completed hands)
    ///
    /// # Environment Variables
    ///
    /// * `axiomind_SIM_FAST` - Enable fast mode (batch writes, minimal output)
    /// * `axiomind_SIM_BREAK_AFTER` - Break after N hands (for testing)
    /// * `axiomind_SIM_SLEEP_MICROS` - Delay between hands in microseconds
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind sim --hands 10000 --output data/sim.jsonl --seed 42 --level 3
    /// ```
    Sim {
        #[arg(long)]
        hands: u64,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=20))]
        level: Option<u8>,
        #[arg(long)]
        resume: Option<String>,
    },
    /// Convert hand histories to various formats.
    ///
    /// Export JSONL hand histories to CSV, pretty-printed JSON, or SQLite database.
    ///
    /// # Options
    ///
    /// * `--input` - Path to input JSONL file
    /// * `--format` - Output format: `csv`, `json`, or `sqlite`
    /// * `--output` - Path to output file
    ///
    /// # Format Details
    ///
    /// - **csv**: Tabular format with columns: hand_id, seed, result, ts, actions, board
    /// - **json**: Pretty-printed JSON array of all hands
    /// - **sqlite**: Relational database with full-text search capability
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind export --input data/hands.jsonl --format sqlite --output data/hands.db
    /// ```
    Export {
        #[arg(long)]
        input: String,
        #[arg(long)]
        format: String,
        #[arg(long)]
        output: String,
    },
    /// Create training/validation/test dataset splits.
    ///
    /// Split a JSONL hand history file into train/val/test sets for machine learning.
    /// Supports both in-memory and streaming modes for large datasets.
    ///
    /// # Options
    ///
    /// * `--input` - Path to input JSONL file
    /// * `--outdir` - Output directory for split files
    /// * `--train` - Training set proportion (default: 0.8, can specify as 80 or 0.8)
    /// * `--val` - Validation set proportion (default: 0.1)
    /// * `--test` - Test set proportion (default: 0.1)
    /// * `--seed` - RNG seed for reproducible shuffling
    ///
    /// # Environment Variables
    ///
    /// * `axiomind_DATASET_STREAM_THRESHOLD` - Min records for streaming mode (default: 10000)
    /// * `axiomind_DATASET_STREAM_TRACE` - Enable streaming debug output
    ///
    /// # Output Files
    ///
    /// Creates `train.jsonl`, `val.jsonl`, and `test.jsonl` in the output directory.
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind dataset --input data/sim.jsonl --outdir data/splits --train 0.7 --val 0.2 --test 0.1
    /// ```
    Dataset {
        #[arg(long)]
        input: String,
        #[arg(long)]
        outdir: String,
        #[arg(long)]
        train: Option<f64>,
        #[arg(long)]
        val: Option<f64>,
        #[arg(long)]
        test: Option<f64>,
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Display current configuration settings.
    ///
    /// Shows all configuration values and their sources (default, file, or env var).
    /// Configuration hierarchy: environment variables > config file > defaults.
    ///
    /// # Configuration Sources
    ///
    /// 1. Environment variables (e.g., `axiomind_SEED`)
    /// 2. Config file (`~/.axiomind.toml` or project `.axiomind.toml`)
    /// 3. Built-in defaults
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind cfg
    /// ```
    Cfg,
    /// Run environment diagnostics.
    ///
    /// Verify that SQLite, file system, and locale are properly configured.
    /// Outputs a JSON report with pass/fail status for each check.
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind doctor
    /// ```
    Doctor,
    /// Test RNG output for debugging.
    ///
    /// Generate and display 5 random u64 values from the ChaCha20 RNG.
    /// Useful for verifying deterministic behavior and seed consistency.
    ///
    /// # Options
    ///
    /// * `--seed` - RNG seed (default: random)
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind rng --seed 12345
    /// ```
    Rng {
        #[arg(long)]
        seed: Option<u64>,
    },
}

/// Opponent type for the `play` command.
///
/// Determines whether the user plays against a human (interactive prompts)
/// or an AI opponent (automated decisions).
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Vs {
    /// Play against a human opponent (requires TTY for interactive input).
    Human,
    /// Play against an AI opponent (automated decision-making).
    Ai,
}

impl Vs {
    /// Returns the string representation of the opponent type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axiomind_cli::Vs;
    /// let opponent = Vs::Ai;
    /// assert_eq!(opponent.as_str(), "ai");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Vs::Human => "human",
            Vs::Ai => "ai",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test command dispatch integration (Task 8.1)
    #[test]
    fn test_cfg_command_dispatch() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_cfg_command(&mut out, &mut err);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Configuration") || !output.is_empty());
    }

    #[test]
    fn test_doctor_command_dispatch() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_doctor_command(&mut out, &mut err);
        // Doctor command should succeed
        assert!(result.is_ok()); // Either outcome is valid for test
    }

    #[test]
    fn test_rng_command_dispatch_with_seed() {
        let mut out = Vec::new();

        let result = handle_rng_command(Some(42), &mut out);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("RNG") || output.contains("seed") || !output.is_empty());
    }

    #[test]
    fn test_rng_command_dispatch_without_seed() {
        let mut out = Vec::new();

        let result = handle_rng_command(None, &mut out);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deal_command_dispatch_with_seed() {
        let mut out = Vec::new();

        let result = handle_deal_command(Some(42), &mut out);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_deal_command_dispatch_without_seed() {
        let mut out = Vec::new();

        let result = handle_deal_command(None, &mut out);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bench_command_dispatch() {
        let mut out = Vec::new();

        let result = handle_bench_command(&mut out);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("hands/sec") || output.contains("Benchmark") || !output.is_empty());
    }

    // Integration tests for play command moved to commands/play.rs module

    #[test]
    fn test_play_level_validation_rejects_out_of_range() {
        // Test that clap rejects level=0
        let result =
            AxiomindCli::try_parse_from(["axiomind", "play", "--vs", "ai", "--level", "0"]);
        assert!(result.is_err());

        // Test that clap rejects level=21
        let result =
            AxiomindCli::try_parse_from(["axiomind", "play", "--vs", "ai", "--level", "21"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_play_level_validation_accepts_valid_range() {
        // Test that clap accepts level=1
        let result =
            AxiomindCli::try_parse_from(["axiomind", "play", "--vs", "ai", "--level", "1"]);
        assert!(result.is_ok());

        // Test that clap accepts level=20
        let result =
            AxiomindCli::try_parse_from(["axiomind", "play", "--vs", "ai", "--level", "20"]);
        assert!(result.is_ok());
    }

    // Phase 3 command dispatch tests (Task 12.1 - TDD)

    #[test]
    fn test_stats_command_dispatch_integration() {
        // Test that stats command module is properly integrated
        // This test verifies the dispatch works, not the full functionality
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Use a non-existent file to test error handling path
        let result = handle_stats_command("nonexistent.jsonl".to_string(), &mut out, &mut err);

        // Should return an error (file doesn't exist)
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_command_dispatch_integration() {
        // Test that eval command module is properly integrated
        let mut out = Vec::new();

        // Run eval with minimal hands count
        let result = handle_eval_command("baseline", "baseline", 1, Some(42), &mut out);

        // Should succeed with baseline AI
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        // Output should contain some eval results
        assert!(!output.is_empty());
    }

    #[test]
    fn test_export_command_dispatch_integration() {
        // Test that export command module is properly integrated
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Use a non-existent input file to test error handling
        let result = handle_export_command(
            "nonexistent.jsonl".to_string(),
            "output.csv".to_string(),
            "csv".to_string(),
            &mut out,
            &mut err,
        );

        // Should return an error (file doesn't exist)
        assert!(result.is_err());
    }

    #[test]
    fn test_play_command_dispatch_via_handler() {
        // Test that play command handler is properly exposed
        use std::io::Cursor;

        let mut out = Vec::new();
        let mut err = Vec::new();
        let input = "quit\n";
        let mut stdin = Cursor::new(input.as_bytes());

        // Run play command with AI opponent, 1 hand
        let result = handle_play_command(
            Vs::Ai,
            Some(1),
            Some(42),
            Some(1),
            &mut out,
            &mut err,
            &mut stdin,
        );

        // Should complete successfully via handler
        assert!(result.is_ok());
    }
}
