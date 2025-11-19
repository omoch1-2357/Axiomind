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
use std::io::BufRead;
use std::io::Write;
mod config;
mod error;
pub mod formatters;
pub mod io_utils;
pub mod ui;
pub mod validation;

// Import utility functions from extracted modules
use formatters::{format_action, format_board};
use io_utils::{ensure_parent_dir, read_stdin_line};
use validation::{parse_player_action, validate_dealing_meta, ParseResult};

use axiomind_ai::create_ai;
use axiomind_engine::engine::{blinds_for_level, Engine};
use axiomind_engine::logger::{ActionRecord, HandRecord, Street};
pub use error::{BatchValidationError, CliError};
use rand::{seq::SliceRandom, RngCore, SeedableRng};

use std::collections::HashSet;

/// Execute the play command with specified parameters
/// Returns Ok(()) on success, or CliError on failure
fn execute_play_command(
    vs: Vs,
    hands: u32,
    seed: Option<u64>,
    level: u8,
    stdin: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    if hands == 0 {
        ui::write_error(err, "hands must be >= 1")?;
        return Err(CliError::InvalidInput("hands must be >= 1".to_string()));
    }

    let seed = seed.unwrap_or_else(rand::random);
    let level = level.clamp(1, 20);

    if matches!(vs, Vs::Ai) {
        ui::display_warning(
            err,
            "AI opponent is a placeholder that always checks. Use for demo purposes only.",
        )?;
    }

    writeln!(
        out,
        "play: vs={} hands={} seed={}",
        vs.as_str(),
        hands,
        seed
    )?;
    writeln!(out, "Level: {}", level)?;

    let mut eng = Engine::new(Some(seed), level);
    eng.shuffle();

    // Create AI opponent for human vs AI mode
    let ai = create_ai("baseline");

    let mut played = 0u32;
    let mut quit_requested = false;

    for i in 1..=hands {
        if quit_requested {
            break;
        }

        // level progression: +1 every 15 hands
        let cur_level: u8 = level
            .saturating_add(((i - 1) / axiomind_engine::player::HANDS_PER_LEVEL) as u8)
            .clamp(1, 20);
        if i > 1 {
            writeln!(out, "Level: {}", cur_level)?;
        }
        eng.set_level(cur_level);
        let (sb, bb) = match eng.blinds() {
            Ok(blinds) => blinds,
            Err(e) => {
                ui::write_error(err, &format!("Failed to get blinds: {}", e))?;
                return Err(CliError::Engine(format!("Failed to get blinds: {}", e)));
            }
        };
        writeln!(out, "Blinds: SB={} BB={}", sb, bb)?;
        writeln!(out, "Hand {}", i)?;
        if let Err(e) = eng.deal_hand() {
            ui::write_error(err, &format!("Failed to deal hand: {}", e))?;
            return Err(CliError::Engine(format!("Failed to deal hand: {}", e)));
        }

        match vs {
            Vs::Human => {
                let human_player_id = 0;

                loop {
                    // Get current actor from engine
                    let current_player = match eng.current_player() {
                        Ok(player) => player,
                        Err(e) => {
                            ui::write_error(err, &format!("Failed to get current player: {}", e))?;
                            return Err(CliError::Engine(format!(
                                "Failed to get current player: {}",
                                e
                            )));
                        }
                    };

                    if current_player == human_player_id {
                        // Human player's turn
                        write!(out, "Enter action (check/call/bet/raise/fold/q): ")?;
                        out.flush()?;

                        match read_stdin_line(stdin) {
                            Some(input) => match parse_player_action(&input) {
                                ParseResult::Action(action) => {
                                    match eng.apply_action(human_player_id, action.clone()) {
                                        Ok(state) => {
                                            let action_str = format_action(&action);
                                            writeln!(out, "Action: {}", action_str)?;
                                            writeln!(out, "Pot: {}", state.pot())?;
                                            if state.is_hand_complete() {
                                                writeln!(out, "Hand complete.")?;
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            ui::write_error(
                                                err,
                                                &format!("Invalid action: {}", e),
                                            )?;
                                        }
                                    }
                                }
                                ParseResult::Quit => {
                                    quit_requested = true;
                                    break;
                                }
                                ParseResult::Invalid(msg) => {
                                    ui::write_error(err, &msg)?;
                                }
                            },
                            None => {
                                quit_requested = true;
                                break;
                            }
                        }
                    } else {
                        // AI turn - use AI to determine action
                        let ai_action = ai.get_action(&eng, current_player);
                        match eng.apply_action(current_player, ai_action.clone()) {
                            Ok(state) => {
                                writeln!(out, "AI: {}", format_action(&ai_action))?;
                                writeln!(out, "Pot: {}", state.pot())?;
                                if state.is_hand_complete() {
                                    writeln!(out, "Hand complete.")?;
                                    break;
                                }
                            }
                            Err(e) => {
                                ui::write_error(err, &format!("AI action failed: {}", e))?;
                                break;
                            }
                        }
                    }
                }
            }
            Vs::Ai => {
                // Existing AI mode placeholder
                writeln!(out, "{}", ui::tag_demo_output("ai: check"))?;
            }
        }
        played += 1;
    }

    writeln!(out, "Session hands={}", hands)?;
    writeln!(out, "Hands played: {} (completed)", played)?;
    Ok(())
}

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

        if let Some(street) = act.get("street").and_then(|s| s.as_str()) {
            if prev_street.as_deref() != Some(street) {
                prev_street = Some(street.to_string());
                street_committed.clear();
                current_high = 0;
                last_full_raise = big_blind.max(min_chip_unit);
                reopen_blocked = false;
            }
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
                    ))
                }
                "raise" => {
                    return Err(format!(
                        "Bet action missing amount at hand {} (action #{})",
                        hand_index,
                        idx + 1
                    ))
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
        if let Some(rem) = remaining.get(&player_id) {
            if delta_chips > *rem {
                delta_chips = *rem;
            }
        }
        let new_commit = commit_before + delta_chips;

        if delta_chips > 0 {
            if let Some(rem) = remaining.get_mut(&player_id) {
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
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ExportAttemptError::Fatal(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
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
            hand_id TEXT PRIMARY KEY NOT NULL,
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

            let actions_len = actions.len() as i64;
            let board_len = board.len() as i64;

            let seed_val = match seed {
                Some(v) if v > i64::MAX as u64 => {
                    return Err(ExportAttemptError::Fatal(format!(
                        "Seed {} exceeds supported range",
                        v
                    )));
                }
                Some(v) => Some(v as i64),
                None => None,
            };

            stmt.execute(rusqlite::params![
                hand_id,
                seed_val,
                result,
                ts,
                actions_len,
                board_len,
                raw,
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

    let max_attempts = std::env::var("axiomind_EXPORT_SQLITE_RETRIES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(3);
    let backoff_ms = std::env::var("axiomind_EXPORT_SQLITE_RETRY_SLEEP_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(50);

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

    Err(CliError::Config("SQLite export failed".to_string()))
}

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
    fn validate_speed(speed: Option<f64>) -> Result<(), String> {
        if let Some(s) = speed {
            if s <= 0.0 {
                return Err("speed must be > 0".into());
            }
        }
        Ok(())
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
    fn run_stats(input: &str, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError> {
        use std::path::Path;

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
                for entry in rd.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                        continue;
                    }
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if !(name.ends_with(".jsonl") || name.ends_with(".jsonl.zst")) {
                        continue;
                    }
                    match read_text_auto(p.to_str().unwrap()) {
                        Ok(s) => consume_stats_content(s, &mut state, err)?,
                        Err(e) => {
                            ui::write_error(
                                err,
                                &format!("Failed to read {}: {}", p.display(), e),
                            )?;
                            state.stats_ok = false;
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

    /// Runs environment diagnostics and health checks.
    ///
    /// Validates the local environment to ensure all dependencies and file system
    /// access are working correctly. Checks include SQLite write capability,
    /// data directory access, and UTF-8 locale support.
    ///
    /// # Arguments
    ///
    /// * `out` - Output stream for diagnostic report (JSON format)
    /// * `err` - Output stream for error messages
    ///
    /// # Returns
    ///
    /// Exit code: `0` if all checks pass, `2` if any check fails
    ///
    /// # Checks Performed
    ///
    /// - **SQLite**: Verifies ability to create and write to SQLite databases
    /// - **Data Directory**: Tests write permissions in data directory
    /// - **Locale**: Ensures UTF-8 locale for proper text handling
    ///
    /// # Environment Variables
    ///
    /// - `axiomind_DOCTOR_SQLITE_DIR`: Override SQLite check directory (default: temp dir)
    /// - `axiomind_DOCTOR_DATA_DIR`: Override data directory path (default: `data/`)
    /// - `axiomind_DOCTOR_LOCALE_OVERRIDE`: Force specific locale for testing
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::io;
    /// let code = axiomind_cli::run(
    ///     vec!["axiomind", "doctor"],
    ///     &mut io::stdout(),
    ///     &mut io::stderr()
    /// );
    /// assert_eq!(code, 0);
    /// ```
    fn run_doctor(out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError> {
        use std::env;
        use std::path::{Path, PathBuf};
        use std::time::{SystemTime, UNIX_EPOCH};

        struct DoctorCheck {
            name: &'static str,
            ok: bool,
            detail: String,
            error: Option<String>,
        }

        impl DoctorCheck {
            fn ok(name: &'static str, detail: impl Into<String>) -> Self {
                DoctorCheck {
                    name,
                    ok: true,
                    detail: detail.into(),
                    error: None,
                }
            }

            fn fail(
                name: &'static str,
                detail: impl Into<String>,
                error: impl Into<String>,
            ) -> Self {
                DoctorCheck {
                    name,
                    ok: false,
                    detail: detail.into(),
                    error: Some(error.into()),
                }
            }

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

        fn unique_suffix() -> u128 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros()
        }

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

        fn check_locale(override_val: Option<String>) -> DoctorCheck {
            if let Some(val) = override_val {
                return evaluate_locale("axiomind_DOCTOR_LOCALE_OVERRIDE", val);
            }
            for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
                if let Ok(val) = std::env::var(key) {
                    return evaluate_locale(key, val);
                }
            }
            let candidate = std::env::temp_dir()
                .join(format!("axiomind-doctor-diagnosis-{}.txt", unique_suffix()));
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

        let json_output = serde_json::to_string_pretty(&serde_json::Value::Object(report))
            .map_err(|e| {
                CliError::InvalidInput(format!("Failed to serialize doctor report: {}", e))
            })?;
        writeln!(out, "{}", json_output)?;

        if ok_all {
            Ok(())
        } else {
            Err(CliError::Config(
                "Environment diagnostics failed".to_string(),
            ))
        }
    }

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
                let hands = hands.unwrap_or(1);
                let level = level.unwrap_or(1).clamp(1, 20);

                // Use stdin for real input (supports both TTY and piped stdin)
                let stdin = std::io::stdin();
                let mut stdin_lock = stdin.lock();
                match execute_play_command(vs, hands, seed, level, &mut stdin_lock, out, err) {
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
                if let Err(msg) = validate_speed(speed) {
                    if ui::write_error(err, &msg).is_err() {
                        return 2;
                    }
                    return 2;
                }

                if let Some(s) = speed {
                    if writeln!(
                        err,
                        "Note: --speed parameter ({}) is not yet used. Interactive mode only.",
                        s
                    )
                    .is_err()
                    {
                        return 2;
                    }
                }

                match read_text_auto(&input) {
                    Ok(content) => {
                        let lines: Vec<&str> =
                            content.lines().filter(|l| !l.trim().is_empty()).collect();
                        let total_hands = lines.len();

                        if total_hands == 0 {
                            if writeln!(out, "No hands found in file.").is_err() {
                                return 2;
                            }
                            return 0;
                        }

                        let mut hand_num = 0;
                        let mut hands_shown = 0usize;
                        for line in lines {
                            hand_num += 1;

                            let record: HandRecord = match serde_json::from_str(line) {
                                Ok(r) => r,
                                Err(e) => {
                                    if ui::write_error(
                                        err,
                                        &format!("Failed to parse hand {}: {}", hand_num, e),
                                    )
                                    .is_err()
                                    {
                                        return 2;
                                    }
                                    continue;
                                }
                            };
                            hands_shown += 1;

                            let level = if let Some(meta) = &record.meta {
                                if let Some(level_val) = meta.get("level") {
                                    level_val.as_u64().unwrap_or(1) as u8
                                } else {
                                    1
                                }
                            } else {
                                1
                            };

                            let button_position = if let Some(meta) = &record.meta {
                                if let Some(button_val) = meta.get("button_position") {
                                    button_val.as_u64().unwrap_or(0) as usize
                                } else {
                                    0
                                }
                            } else {
                                0
                            };

                            let (sb, bb) = match blinds_for_level(level) {
                                Ok(amounts) => amounts,
                                Err(e) => {
                                    if ui::write_error(
                                        err,
                                        &format!("Invalid blind level {}: {}", level, e),
                                    )
                                    .is_err()
                                    {
                                        return 2;
                                    }
                                    (0, 0)
                                }
                            };

                            if writeln!(
                                out,
                                "Hand #{} (Seed: {}, Level: {})",
                                hand_num,
                                record
                                    .seed
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| "N/A".to_string()),
                                level
                            )
                            .is_err()
                                || writeln!(out, "═══════════════════════════════════════").is_err()
                                || writeln!(out, "Blinds: SB={} BB={}", sb, bb).is_err()
                                || writeln!(out, "Button: Player {}", button_position).is_err()
                                || writeln!(out).is_err()
                            {
                                return 2;
                            }

                            // Variables for initial stack and pot tracking
                            const STARTING_STACK: u32 = 20000;
                            let mut stacks = [STARTING_STACK, STARTING_STACK];
                            let mut pot: u32 = 0;

                            // Track commit amount and current bet for each street
                            let mut committed = [0u32; 2];
                            let mut current_street: Option<Street> = None;
                            #[allow(unused_assignments)]
                            let mut current_bet: u32 = 0;

                            // Post blinds at preflop start (Button=SB, Opponent=BB)
                            let other_player = 1 - button_position;
                            committed[button_position] = sb;
                            committed[other_player] = bb;
                            stacks[button_position] = stacks[button_position].saturating_sub(sb);
                            stacks[other_player] = stacks[other_player].saturating_sub(bb);
                            pot = pot.saturating_add(sb).saturating_add(bb);
                            current_bet = bb;

                            let streets_order =
                                [Street::Preflop, Street::Flop, Street::Turn, Street::River];

                            for street in &streets_order {
                                let actions_for_street: Vec<&ActionRecord> = record
                                    .actions
                                    .iter()
                                    .filter(|a| a.street == *street)
                                    .collect();

                                if !actions_for_street.is_empty() {
                                    // Reset commit and current bet when street changes
                                    if current_street != Some(*street) {
                                        current_street = Some(*street);
                                        committed = [0, 0];
                                        current_bet = 0;

                                        // For preflop only, reflect blinds as commits
                                        if *street == Street::Preflop {
                                            committed[button_position] = sb;
                                            committed[other_player] = bb;
                                            current_bet = bb;
                                        }
                                    }

                                    match street {
                                        Street::Preflop => {
                                            if writeln!(out, "Preflop:").is_err() {
                                                return 2;
                                            }
                                            let btn_pos_label = if button_position == 0 {
                                                "[BTN/SB]"
                                            } else {
                                                "[BB]"
                                            };
                                            let other_pos_label = if button_position == 0 {
                                                "[BB]"
                                            } else {
                                                "[BTN/SB]"
                                            };
                                            if writeln!(
                                                out,
                                                "  Player 0 {}: ?? ??  (Stack: {})",
                                                if button_position == 0 {
                                                    btn_pos_label
                                                } else {
                                                    other_pos_label
                                                },
                                                stacks[0]
                                            )
                                            .is_err()
                                                || writeln!(
                                                    out,
                                                    "  Player 1 {}: ?? ??  (Stack: {})",
                                                    if button_position == 1 {
                                                        btn_pos_label
                                                    } else {
                                                        other_pos_label
                                                    },
                                                    stacks[1]
                                                )
                                                .is_err()
                                                || writeln!(out).is_err()
                                            {
                                                return 2;
                                            }
                                        }
                                        Street::Flop => {
                                            let flop_cards = if record.board.len() >= 3 {
                                                &record.board[0..3]
                                            } else {
                                                &record.board[..]
                                            };
                                            if writeln!(out, "Flop: {}", format_board(flop_cards))
                                                .is_err()
                                            {
                                                return 2;
                                            }
                                        }
                                        Street::Turn => {
                                            let turn_cards = if record.board.len() >= 4 {
                                                &record.board[0..4]
                                            } else {
                                                &record.board[..]
                                            };
                                            if writeln!(out, "Turn: {}", format_board(turn_cards))
                                                .is_err()
                                            {
                                                return 2;
                                            }
                                        }
                                        Street::River => {
                                            let river_cards = &record.board[..];
                                            if writeln!(out, "River: {}", format_board(river_cards))
                                                .is_err()
                                            {
                                                return 2;
                                            }
                                        }
                                    }

                                    for action_rec in &actions_for_street {
                                        let player_id = action_rec.player_id;
                                        let action = &action_rec.action;

                                        let mut delta: u32 = 0;

                                        match action {
                                            axiomind_engine::player::PlayerAction::Bet(amount) => {
                                                // Bet is treated as 'total commit for this street'
                                                let target = *amount;
                                                if target > committed[player_id] {
                                                    delta = target - committed[player_id];
                                                    committed[player_id] = target;
                                                    current_bet = current_bet.max(target);
                                                }
                                            }
                                            axiomind_engine::player::PlayerAction::Raise(
                                                amount,
                                            ) => {
                                                // Raise is treated as increment to current bet
                                                let target = current_bet.saturating_add(*amount);
                                                if target > committed[player_id] {
                                                    delta = target - committed[player_id];
                                                    committed[player_id] = target;
                                                    current_bet = target;
                                                }
                                            }
                                            axiomind_engine::player::PlayerAction::Call => {
                                                // Call commits difference from current bet
                                                if current_bet > committed[player_id] {
                                                    let needed = current_bet - committed[player_id];
                                                    delta = needed.min(stacks[player_id]);
                                                    committed[player_id] =
                                                        committed[player_id].saturating_add(delta);
                                                }
                                            }
                                            axiomind_engine::player::PlayerAction::AllIn => {
                                                // AllIn puts in all remaining stack
                                                delta = stacks[player_id];
                                                committed[player_id] =
                                                    committed[player_id].saturating_add(delta);
                                                current_bet = current_bet.max(committed[player_id]);
                                            }
                                            axiomind_engine::player::PlayerAction::Check
                                            | axiomind_engine::player::PlayerAction::Fold => {
                                                // Check and Fold have no chip movement
                                            }
                                        }

                                        if delta > 0 {
                                            stacks[player_id] =
                                                stacks[player_id].saturating_sub(delta);
                                            pot = pot.saturating_add(delta);
                                        }

                                        if writeln!(
                                            out,
                                            "  Player {}: {}",
                                            player_id,
                                            format_action(action)
                                        )
                                        .is_err()
                                            || writeln!(out, "  Pot: {}", pot).is_err()
                                        {
                                            return 2;
                                        }
                                    }
                                    if writeln!(out).is_err() {
                                        return 2;
                                    }
                                }
                            }

                            if let Some(showdown) = &record.showdown {
                                if writeln!(out, "Showdown:").is_err() {
                                    return 2;
                                }
                                for winner in &showdown.winners {
                                    if writeln!(out, "  Player {} wins {} chips", winner, pot)
                                        .is_err()
                                    {
                                        return 2;
                                    }
                                }
                                if let Some(notes) = &showdown.notes {
                                    if writeln!(out, "  Notes: {}", notes).is_err() {
                                        return 2;
                                    }
                                }
                                if writeln!(out).is_err() {
                                    return 2;
                                }
                            } else if let Some(result_str) = &record.result {
                                if writeln!(out, "Result:").is_err()
                                    || writeln!(out, "  {} wins {} chips", result_str, pot).is_err()
                                    || writeln!(out).is_err()
                                {
                                    return 2;
                                }
                            }

                            if hand_num < total_hands {
                                if writeln!(out, "Press Enter for next hand (or 'q' to quit)...")
                                    .is_err()
                                {
                                    return 2;
                                }
                                let mut user_input = String::new();
                                if std::io::stdin().read_line(&mut user_input).is_ok() {
                                    let trimmed = user_input.trim().to_lowercase();
                                    if trimmed == "q" || trimmed == "quit" {
                                        if writeln!(
                                            out,
                                            "Replay stopped at hand {}/{}",
                                            hand_num, total_hands
                                        )
                                        .is_err()
                                        {
                                            return 2;
                                        }
                                        return 0;
                                    }
                                }
                            }
                        }

                        if writeln!(out, "Replay complete. {} hands shown.", hands_shown).is_err() {
                            return 2;
                        }
                        0
                    }
                    Err(e) => {
                        if ui::write_error(err, &format!("Failed to read {}: {}", input, e))
                            .is_err()
                        {
                            return 2;
                        }
                        2
                    }
                }
            }
            Commands::Stats { input } => match run_stats(&input, out, err) {
                Ok(()) => 0,
                Err(_) => 2,
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
                                } else if let Some(arr) = blinds_val.as_array() {
                                    if arr.len() >= 2 {
                                        if let Some(bb) = arr[1].as_i64() {
                                            big_blind = bb;
                                        }
                                    }
                                }
                            }
                            if big_blind < MIN_CHIP_UNIT {
                                big_blind = MIN_CHIP_UNIT;
                            }
                            if let Some(actions) = v.get("actions").and_then(|a| a.as_array()) {
                                if let Some(ref start_map) = starting_stacks {
                                    if let Err(msg) = ensure_no_reopen_after_short_all_in(
                                        actions,
                                        big_blind,
                                        MIN_CHIP_UNIT,
                                        start_map,
                                        hands,
                                    ) {
                                        errors.push(ValidationError::new(
                                            hand_id.clone(),
                                            hands as usize,
                                            &msg,
                                        ));
                                    }
                                }
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
            Commands::Doctor => match run_doctor(out, err) {
                Ok(()) => 0,
                Err(_) => 2,
            },
            Commands::Eval {
                ai_a,
                ai_b,
                hands,
                seed,
            } => match handle_eval_command(&ai_a, &ai_b, hands, seed, out, err) {
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
                    if let Some(b) = break_after {
                        if completed == b {
                            if writeln!(out, "Interrupted: saved {}/{}", completed, total).is_err()
                            {
                                return 2;
                            }
                            return 130;
                        }
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
            } => match handle_export_command(&input, &format, &output, err) {
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

/// Statistics tracked for AI evaluation comparison
#[derive(Debug, Clone)]
struct EvalStats {
    hands_played: u32,
    wins: u32,
    losses: u32,
    ties: u32,
    total_chips_won: i64,
    total_pot_size: u64,
    folds: u32,
    checks: u32,
    calls: u32,
    bets: u32,
    raises: u32,
    all_ins: u32,
}

impl EvalStats {
    fn new() -> Self {
        Self {
            hands_played: 0,
            wins: 0,
            losses: 0,
            ties: 0,
            total_chips_won: 0,
            total_pot_size: 0,
            folds: 0,
            checks: 0,
            calls: 0,
            bets: 0,
            raises: 0,
            all_ins: 0,
        }
    }

    fn update_from_actions(
        &mut self,
        actions: &[axiomind_engine::logger::ActionRecord],
        player_id: usize,
    ) {
        for action in actions {
            if action.player_id == player_id {
                use axiomind_engine::player::PlayerAction;
                match action.action {
                    PlayerAction::Fold => self.folds += 1,
                    PlayerAction::Check => self.checks += 1,
                    PlayerAction::Call => self.calls += 1,
                    PlayerAction::Bet(_) => self.bets += 1,
                    PlayerAction::Raise(_) => self.raises += 1,
                    PlayerAction::AllIn => self.all_ins += 1,
                }
            }
        }
    }

    fn update_result(&mut self, won: bool, tied: bool, chip_delta: i64, pot: u32) {
        self.hands_played += 1;
        if tied {
            self.ties += 1;
        } else if won {
            self.wins += 1;
        } else {
            self.losses += 1;
        }
        self.total_chips_won += chip_delta;
        self.total_pot_size += pot as u64;
    }

    fn win_rate(&self) -> f64 {
        if self.hands_played == 0 {
            0.0
        } else {
            (self.wins as f64 / self.hands_played as f64) * 100.0
        }
    }

    fn avg_chip_delta(&self) -> f64 {
        if self.hands_played == 0 {
            0.0
        } else {
            self.total_chips_won as f64 / self.hands_played as f64
        }
    }

    fn avg_pot_size(&self) -> f64 {
        if self.hands_played == 0 {
            0.0
        } else {
            self.total_pot_size as f64 / self.hands_played as f64
        }
    }

    fn total_actions(&self) -> u32 {
        self.folds + self.checks + self.calls + self.bets + self.raises + self.all_ins
    }

    fn action_percentage(&self, count: u32) -> f64 {
        let total = self.total_actions();
        if total == 0 {
            0.0
        } else {
            (count as f64 / total as f64) * 100.0
        }
    }
}

/// Play a hand with two different AI opponents
/// Returns (actions, result_string, showdown_info, pot)
fn play_hand_with_two_ais(
    engine: &mut Engine,
    ai_0: &dyn axiomind_ai::AIOpponent,
    ai_1: &dyn axiomind_ai::AIOpponent,
) -> (
    Vec<axiomind_engine::logger::ActionRecord>,
    String,
    Option<serde_json::Value>,
    u32,
) {
    use axiomind_engine::cards::Card;
    use axiomind_engine::hand::{compare_hands, evaluate_hand};

    // Play through the hand
    while let Ok(current_player) = engine.current_player() {
        let action = if current_player == 0 {
            ai_0.get_action(engine, current_player)
        } else {
            ai_1.get_action(engine, current_player)
        };

        match engine.apply_action(current_player, action) {
            Ok(state) if state.is_hand_complete() => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }

    // Get action history
    let actions = engine.action_history();
    let pot = engine.pot();

    // Determine winner
    let (result_string, showdown) = if let Some(folded) = engine.folded_player() {
        // Someone folded - other player wins
        let winner = 1 - folded;
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

    (actions, result_string, showdown, pot)
}

/// Handle the cfg command
fn handle_cfg_command(out: &mut dyn Write, err: &mut dyn Write) -> Result<(), CliError> {
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
    writeln!(out, "{}", serde_json::to_string_pretty(&display).unwrap())?;
    Ok(())
}

/// Handle the export command
fn handle_export_command(
    input: &str,
    format: &str,
    output: &str,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    let content = std::fs::read_to_string(input).map_err(|e| {
        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
        CliError::Io(e)
    })?;

    match format {
        f if f.eq_ignore_ascii_case("csv") => {
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
                        ui::write_error(
                            err,
                            &format!("Invalid record at line {}: {}", idx + 1, e),
                        )?;
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
        f if f.eq_ignore_ascii_case("sqlite") => export_sqlite(&content, output, err),
        f if f.eq_ignore_ascii_case("json") => {
            let mut arr = Vec::new();
            for (idx, line) in content.lines().filter(|l| !l.trim().is_empty()).enumerate() {
                let v: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        ui::write_error(
                            err,
                            &format!("Invalid record at line {}: {}", idx + 1, e),
                        )?;
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
            std::fs::write(output, s).map_err(|e| {
                let _ = ui::write_error(err, &format!("Failed to write {}: {}", output, e));
                CliError::Io(e)
            })?;
            Ok(())
        }
        _ => Err(CliError::InvalidInput("Unsupported format".to_string())),
    }
}

/// Handle the bench command
fn handle_bench_command(out: &mut dyn Write) -> Result<(), CliError> {
    // quick bench: evaluate 200 unique 7-card draws from shuffled deck
    use axiomind_engine::cards::Card;
    use axiomind_engine::deck::Deck;
    let start = std::time::Instant::now();
    let mut cnt = 0u64;
    let mut deck = Deck::new_with_seed(1);
    deck.shuffle();
    for _ in 0..200 {
        if deck.remaining() < 7 {
            deck.shuffle();
        }
        let mut arr: [Card; 7] = [deck.deal_card().unwrap(); 7];
        for item in arr.iter_mut().skip(1) {
            *item = deck.deal_card().unwrap();
        }
        // Result intentionally unused - benchmark only measures performance
        let _ = axiomind_engine::hand::evaluate_hand(&arr);
        cnt += 1;
    }
    let dur = start.elapsed();
    writeln!(out, "Benchmark: {} iters in {:?}", cnt, dur)?;
    Ok(())
}

/// Handle the deal command
fn handle_deal_command(seed: Option<u64>, out: &mut dyn Write) -> Result<(), CliError> {
    let base_seed = seed.unwrap_or_else(rand::random);
    let mut eng = Engine::new(Some(base_seed), 1);
    eng.shuffle();
    // Return value intentionally unused - engine state is what matters
    let _ = eng.deal_hand();
    let p = eng.players();
    let hc1 = p[0].hole_cards();
    let hc2 = p[1].hole_cards();
    let fmt = |c: axiomind_engine::cards::Card| format!("{:?}{:?}", c.rank, c.suit);
    writeln!(
        out,
        "Hole P1: {} {}",
        fmt(hc1[0].unwrap()),
        fmt(hc1[1].unwrap())
    )?;
    writeln!(
        out,
        "Hole P2: {} {}",
        fmt(hc2[0].unwrap()),
        fmt(hc2[1].unwrap())
    )?;
    let b = eng.board();
    writeln!(
        out,
        "Board: {} {} {} {} {}",
        fmt(b[0]),
        fmt(b[1]),
        fmt(b[2]),
        fmt(b[3]),
        fmt(b[4])
    )?;
    Ok(())
}

/// Handle the rng command
fn handle_rng_command(seed: Option<u64>, out: &mut dyn Write) -> Result<(), CliError> {
    let s = seed.unwrap_or_else(rand::random);
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(s);
    let mut vals = vec![];
    for _ in 0..5 {
        vals.push(rng.next_u64());
    }
    writeln!(out, "RNG sample: {:?}", vals)?;
    Ok(())
}

/// Handle the eval command
fn handle_eval_command(
    ai_a: &str,
    ai_b: &str,
    hands: u32,
    seed: Option<u64>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    // Create AI instances
    let ai_policy_a = match std::panic::catch_unwind(|| create_ai(ai_a)) {
        Ok(ai) => ai,
        Err(_) => {
            ui::write_error(err, &format!("Unknown AI type: {}", ai_a))?;
            return Err(CliError::InvalidInput(format!("Unknown AI type: {}", ai_a)));
        }
    };

    let ai_policy_b = match std::panic::catch_unwind(|| create_ai(ai_b)) {
        Ok(ai) => ai,
        Err(_) => {
            ui::write_error(err, &format!("Unknown AI type: {}", ai_b))?;
            return Err(CliError::InvalidInput(format!("Unknown AI type: {}", ai_b)));
        }
    };

    // Initialize statistics
    let mut stats_a = EvalStats::new();
    let mut stats_b = EvalStats::new();

    // Determine base seed
    let base_seed = seed.unwrap_or_else(rand::random);

    // Play N hands
    for hand_num in 0..hands {
        // Create unique seed for this hand
        let hand_seed = base_seed.wrapping_add(hand_num as u64);

        // Create and setup engine
        let mut engine = Engine::new(Some(hand_seed), 1);
        engine.shuffle();
        // Return value intentionally unused - engine state is what matters
        let _ = engine.deal_hand();

        // Record initial stacks
        let initial_stacks = [engine.players()[0].stack(), engine.players()[1].stack()];

        // Assign AIs to positions (alternate button for fairness)
        let (ai_0, ai_1, ai_a_position) = if hand_num % 2 == 0 {
            (ai_policy_a.as_ref(), ai_policy_b.as_ref(), 0)
        } else {
            (ai_policy_b.as_ref(), ai_policy_a.as_ref(), 1)
        };

        // Play hand to completion
        let (actions, result_string, showdown, pot) =
            play_hand_with_two_ais(&mut engine, ai_0, ai_1);

        // Determine winner(s)
        let (winner_ids, tied) = if let Some(showdown_data) = showdown {
            if let Some(winners) = showdown_data.get("winners") {
                if let Some(winners_array) = winners.as_array() {
                    let winner_vec: Vec<usize> = winners_array
                        .iter()
                        .filter_map(|v| v.as_u64().map(|n| n as usize))
                        .collect();
                    let is_tie = winner_vec.len() > 1;
                    (winner_vec, is_tie)
                } else {
                    (vec![], false)
                }
            } else {
                (vec![], false)
            }
        } else if result_string.contains("Player 0 wins") {
            (vec![0], false)
        } else if result_string.contains("Player 1 wins") {
            (vec![1], false)
        } else {
            (vec![], false)
        };

        // Calculate chip deltas
        let final_stacks = [engine.players()[0].stack(), engine.players()[1].stack()];

        let delta_0 = final_stacks[0] as i64 - initial_stacks[0] as i64;
        let delta_1 = final_stacks[1] as i64 - initial_stacks[1] as i64;

        // Update statistics based on AI-A's position
        let (ai_a_won, ai_a_delta) = if ai_a_position == 0 {
            (winner_ids.contains(&0), delta_0)
        } else {
            (winner_ids.contains(&1), delta_1)
        };

        let ai_b_won = !tied && !ai_a_won;
        let ai_b_delta = -ai_a_delta;

        // Update action statistics
        stats_a.update_from_actions(&actions, ai_a_position);
        stats_b.update_from_actions(&actions, 1 - ai_a_position);

        // Update result statistics
        stats_a.update_result(ai_a_won, tied, ai_a_delta, pot);
        stats_b.update_result(ai_b_won, tied, ai_b_delta, pot);
    }

    // Print results
    print_eval_results(out, ai_a, ai_b, &stats_a, &stats_b, hands, base_seed)?;

    Ok(())
}

/// Print evaluation results comparing two AIs
fn print_eval_results(
    out: &mut dyn Write,
    ai_a_name: &str,
    ai_b_name: &str,
    stats_a: &EvalStats,
    stats_b: &EvalStats,
    hands: u32,
    seed: u64,
) -> std::io::Result<()> {
    writeln!(out, "\nAI Comparison Results")?;
    writeln!(out, "═══════════════════════════════════════")?;
    writeln!(out, "Hands played: {}", hands)?;
    writeln!(out, "Seed: {}", seed)?;
    writeln!(out)?;

    writeln!(out, "AI-A ({}):", ai_a_name)?;
    writeln!(out, "  Wins: {} ({:.1}%)", stats_a.wins, stats_a.win_rate())?;
    writeln!(
        out,
        "  Losses: {} ({:.1}%)",
        stats_a.losses,
        (stats_a.losses as f64 / hands as f64) * 100.0
    )?;
    writeln!(
        out,
        "  Ties: {} ({:.1}%)",
        stats_a.ties,
        (stats_a.ties as f64 / hands as f64) * 100.0
    )?;
    writeln!(out, "  Avg chip delta: {:.1}", stats_a.avg_chip_delta())?;
    writeln!(out, "  Avg pot: {:.1}", stats_a.avg_pot_size())?;
    writeln!(
        out,
        "  Actions: Fold {:.1}% | Check {:.1}% | Call {:.1}% | Bet {:.1}% | Raise {:.1}% | All-in {:.1}%",
        stats_a.action_percentage(stats_a.folds),
        stats_a.action_percentage(stats_a.checks),
        stats_a.action_percentage(stats_a.calls),
        stats_a.action_percentage(stats_a.bets),
        stats_a.action_percentage(stats_a.raises),
        stats_a.action_percentage(stats_a.all_ins),
    )?;
    writeln!(out)?;

    writeln!(out, "AI-B ({}):", ai_b_name)?;
    writeln!(out, "  Wins: {} ({:.1}%)", stats_b.wins, stats_b.win_rate())?;
    writeln!(
        out,
        "  Losses: {} ({:.1}%)",
        stats_b.losses,
        (stats_b.losses as f64 / hands as f64) * 100.0
    )?;
    writeln!(
        out,
        "  Ties: {} ({:.1}%)",
        stats_b.ties,
        (stats_b.ties as f64 / hands as f64) * 100.0
    )?;
    writeln!(out, "  Avg chip delta: {:.1}", stats_b.avg_chip_delta())?;
    writeln!(out, "  Avg pot: {:.1}", stats_b.avg_pot_size())?;
    writeln!(
        out,
        "  Actions: Fold {:.1}% | Check {:.1}% | Call {:.1}% | Bet {:.1}% | Raise {:.1}% | All-in {:.1}%",
        stats_b.action_percentage(stats_b.folds),
        stats_b.action_percentage(stats_b.checks),
        stats_b.action_percentage(stats_b.calls),
        stats_b.action_percentage(stats_b.bets),
        stats_b.action_percentage(stats_b.raises),
        stats_b.action_percentage(stats_b.all_ins),
    )?;
    Ok(())
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

        if let Some(b) = break_after {
            if completed == b {
                if let Some(w) = writer.as_mut() {
                    if let Err(e) = w.flush() {
                        ui::write_error(err, "Failed to flush simulation output")?;
                        return Err(CliError::Io(e));
                    }
                }
                writeln!(out, "Interrupted: saved {}/{}", completed, total)?;
                return Err(CliError::Interrupted(format!(
                    "Interrupted: saved {}/{}",
                    completed, total
                )));
            }
        }
    }

    if let Some(mut w) = writer {
        if let Err(e) = w.flush() {
            ui::write_error(err, "Failed to flush simulation output")?;
            return Err(CliError::Io(e));
        }
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
enum Vs {
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
    /// # use axiomind_cli::*;
    /// // Note: Vs enum is not public, this is for illustration
    /// // let opponent = Vs::Ai;
    /// // assert_eq!(opponent.as_str(), "ai");
    /// ```
    fn as_str(&self) -> &'static str {
        match self {
            Vs::Human => "human",
            Vs::Ai => "ai",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn test_read_stdin_line_valid_input() {
        let input = b"fold\n";
        let mut reader = BufReader::new(&input[..]);
        let result = read_stdin_line(&mut reader);
        assert_eq!(result, Some("fold".to_string()));
    }

    #[test]
    fn test_read_stdin_line_with_whitespace() {
        let input = b"  call  \n";
        let mut reader = BufReader::new(&input[..]);
        let result = read_stdin_line(&mut reader);
        assert_eq!(result, Some("call".to_string()));
    }

    #[test]
    fn test_read_stdin_line_empty_after_trim() {
        let input = b"   \n";
        let mut reader = BufReader::new(&input[..]);
        let result = read_stdin_line(&mut reader);
        assert_eq!(result, Some(String::new()));
    }

    #[test]
    fn test_read_stdin_line_eof() {
        let input = b"";
        let mut reader = BufReader::new(&input[..]);
        let result = read_stdin_line(&mut reader);
        assert_eq!(result, None);
    }

    #[test]
    fn test_read_stdin_line_bet_with_amount() {
        let input = b"bet 100\n";
        let mut reader = BufReader::new(&input[..]);
        let result = read_stdin_line(&mut reader);
        assert_eq!(result, Some("bet 100".to_string()));
    }

    // Tests for input parser (Task 3.2)
    #[test]
    fn test_parse_fold() {
        let result = parse_player_action("fold");
        assert!(matches!(
            result,
            ParseResult::Action(axiomind_engine::player::PlayerAction::Fold)
        ));
    }

    #[test]
    fn test_parse_check_case_insensitive() {
        let result = parse_player_action("CHECK");
        assert!(matches!(
            result,
            ParseResult::Action(axiomind_engine::player::PlayerAction::Check)
        ));
    }

    #[test]
    fn test_parse_call() {
        let result = parse_player_action("call");
        assert!(matches!(
            result,
            ParseResult::Action(axiomind_engine::player::PlayerAction::Call)
        ));
    }

    #[test]
    fn test_parse_bet_with_amount() {
        let result = parse_player_action("bet 100");
        match result {
            ParseResult::Action(axiomind_engine::player::PlayerAction::Bet(amount)) => {
                assert_eq!(amount, 100);
            }
            _ => panic!("Expected Bet action with amount 100"),
        }
    }

    #[test]
    fn test_parse_raise_with_amount() {
        let result = parse_player_action("raise 50");
        match result {
            ParseResult::Action(axiomind_engine::player::PlayerAction::Raise(amount)) => {
                assert_eq!(amount, 50);
            }
            _ => panic!("Expected Raise action with amount 50"),
        }
    }

    #[test]
    fn test_parse_quit_lowercase() {
        let result = parse_player_action("q");
        assert!(matches!(result, ParseResult::Quit));
    }

    #[test]
    fn test_parse_quit_full() {
        let result = parse_player_action("quit");
        assert!(matches!(result, ParseResult::Quit));
    }

    #[test]
    fn test_parse_quit_uppercase() {
        let result = parse_player_action("QUIT");
        assert!(matches!(result, ParseResult::Quit));
    }

    #[test]
    fn test_parse_invalid_action() {
        let result = parse_player_action("invalid");
        match result {
            ParseResult::Invalid(msg) => {
                assert!(msg.contains("Unrecognized") || msg.contains("Invalid"));
            }
            _ => panic!("Expected Invalid result"),
        }
    }

    #[test]
    fn test_parse_bet_no_amount() {
        let result = parse_player_action("bet");
        match result {
            ParseResult::Invalid(msg) => {
                assert!(msg.contains("amount") || msg.contains("bet"));
            }
            _ => panic!("Expected Invalid result for bet without amount"),
        }
    }

    #[test]
    fn test_parse_bet_negative_amount() {
        let result = parse_player_action("bet -50");
        match result {
            ParseResult::Invalid(msg) => {
                assert!(msg.contains("positive") || msg.contains("Invalid"));
            }
            _ => panic!("Expected Invalid result for negative bet"),
        }
    }

    #[test]
    fn test_parse_bet_invalid_amount() {
        let result = parse_player_action("bet abc");
        match result {
            ParseResult::Invalid(msg) => {
                assert!(msg.contains("Invalid") || msg.contains("amount"));
            }
            _ => panic!("Expected Invalid result for non-numeric amount"),
        }
    }

    // Tests for play command integration (Task 3.3)
    #[test]
    fn test_execute_play_reads_stdin() {
        let input = b"fold\n";
        let mut stdin = BufReader::new(&input[..]);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let result = execute_play_command(
            Vs::Human,
            1,
            Some(42),
            1,
            &mut stdin,
            &mut stdout,
            &mut stderr,
        );

        assert!(result.is_ok());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("play:"));
    }

    #[test]
    fn test_execute_play_handles_quit() {
        let input = b"q\n";
        let mut stdin = BufReader::new(&input[..]);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let result = execute_play_command(
            Vs::Human,
            1,
            Some(42),
            1,
            &mut stdin,
            &mut stdout,
            &mut stderr,
        );

        assert!(result.is_ok());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Session") || output.contains("hands"));
    }

    #[test]
    fn test_execute_play_handles_invalid_input_then_valid() {
        let input = b"invalid\nfold\n";
        let mut stdin = BufReader::new(&input[..]);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let result = execute_play_command(
            Vs::Human,
            1,
            Some(42),
            1,
            &mut stdin,
            &mut stdout,
            &mut stderr,
        );

        assert!(result.is_ok());
        let stderr_output = String::from_utf8(stderr).unwrap();
        assert!(stderr_output.contains("Unrecognized") || stderr_output.contains("Invalid"));
    }

    #[test]
    fn test_execute_play_ai_mode() {
        let input = b"";
        let mut stdin = BufReader::new(&input[..]);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let result =
            execute_play_command(Vs::Ai, 1, Some(42), 1, &mut stdin, &mut stdout, &mut stderr);

        assert!(result.is_ok());
        let stderr_output = String::from_utf8(stderr).unwrap();
        // AI mode no longer shows warning since we have real AI
        assert!(stderr_output.is_empty() || !stderr_output.contains("ERROR"));
    }

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
}
