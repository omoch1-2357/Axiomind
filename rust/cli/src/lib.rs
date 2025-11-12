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
//! let args = vec!["axm", "play", "--vs", "ai", "--hands", "10"];
//! let code = axm_cli::run(args, &mut io::stdout(), &mut io::stderr());
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
use std::io::{BufRead, IsTerminal};
mod config;
pub mod ui;
use axm_engine::engine::Engine;
use rand::{seq::SliceRandom, RngCore, SeedableRng};

use std::collections::HashSet;

/// Reads a line of input from a buffered reader, blocking until available.
/// Returns None on EOF, read error, or if the trimmed line is empty.
fn read_stdin_line(stdin: &mut dyn BufRead) -> Option<String> {
    let mut line = String::new();
    match stdin.read_line(&mut line) {
        Ok(0) => None, // EOF
        Ok(_) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Err(_) => None, // Read error
    }
}

/// Result type for parsing user input into player actions
#[derive(Debug, PartialEq)]
enum ParseResult {
    /// Valid player action
    Action(axm_engine::player::PlayerAction),
    /// User wants to quit
    Quit,
    /// Invalid input with error message
    Invalid(String),
}

/// Parse user input string into a PlayerAction
/// Returns ParseResult indicating success, quit, or error
fn parse_player_action(input: &str) -> ParseResult {
    let input = input.trim().to_lowercase();
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.is_empty() {
        return ParseResult::Invalid("Empty input".to_string());
    }

    // Check for quit commands first
    if parts[0] == "q" || parts[0] == "quit" {
        return ParseResult::Quit;
    }

    // Parse actions
    match parts[0] {
        "fold" => ParseResult::Action(axm_engine::player::PlayerAction::Fold),
        "check" => ParseResult::Action(axm_engine::player::PlayerAction::Check),
        "call" => ParseResult::Action(axm_engine::player::PlayerAction::Call),
        "bet" => {
            if parts.len() < 2 {
                return ParseResult::Invalid("Bet requires an amount (e.g., 'bet 100')".to_string());
            }
            match parts[1].parse::<u32>() {
                Ok(amount) if amount > 0 => {
                    ParseResult::Action(axm_engine::player::PlayerAction::Bet(amount))
                }
                Ok(_) => ParseResult::Invalid("Bet amount must be positive".to_string()),
                Err(_) => ParseResult::Invalid("Invalid bet amount".to_string()),
            }
        }
        "raise" => {
            if parts.len() < 2 {
                return ParseResult::Invalid(
                    "Raise requires an amount (e.g., 'raise 50')".to_string(),
                );
            }
            match parts[1].parse::<u32>() {
                Ok(amount) if amount > 0 => {
                    ParseResult::Action(axm_engine::player::PlayerAction::Raise(amount))
                }
                Ok(_) => ParseResult::Invalid("Raise amount must be positive".to_string()),
                Err(_) => ParseResult::Invalid("Invalid raise amount".to_string()),
            }
        }
        _ => ParseResult::Invalid(format!(
            "Unrecognized action '{}'. Valid actions: fold, check, call, bet <amount>, raise <amount>, q",
            parts[0]
        )),
    }
}

/// Execute the play command with specified parameters
/// Returns exit code (0 = success, non-zero = error)
fn execute_play_command(
    vs: Vs,
    hands: u32,
    seed: Option<u64>,
    level: u8,
    stdin: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    if hands == 0 {
        let _ = ui::write_error(err, "hands must be >= 1");
        return 2;
    }

    let seed = seed.unwrap_or_else(rand::random);
    let level = level.max(1);

    // Display warning for AI placeholder mode
    if matches!(vs, Vs::Ai) {
        let _ = ui::display_warning(
            err,
            "AI opponent is a placeholder that always checks. Use for demo purposes only.",
        );
    }

    let _ = writeln!(
        out,
        "play: vs={} hands={} seed={}",
        vs.as_str(),
        hands,
        seed
    );
    let _ = writeln!(out, "Level: {}", level);

    let mut eng = Engine::new(Some(seed), level);
    eng.shuffle();

    let mut played = 0u32;
    let mut quit_requested = false;

    for i in 1..=hands {
        if quit_requested {
            break;
        }

        // simple level progression: +1 every 2 hands
        let cur_level: u8 = level.saturating_add(((i - 1) / 2) as u8);
        if i > 1 {
            let _ = writeln!(out, "Level: {}", cur_level);
        }
        let (sb, bb) = match cur_level {
            1 => (50, 100),
            2 => (75, 150),
            3 => (100, 200),
            _ => (150, 300),
        };
        let _ = writeln!(out, "Blinds: SB={} BB={}", sb, bb);
        let _ = writeln!(out, "Hand {}", i);
        let _ = eng.deal_hand();

        match vs {
            Vs::Human => {
                // Human mode: read and parse actions from stdin
                loop {
                    let _ = write!(out, "Enter action (check/call/bet/raise/fold/q): ");
                    let _ = out.flush();

                    match read_stdin_line(stdin) {
                        Some(input) => {
                            match parse_player_action(&input) {
                                ParseResult::Action(_action) => {
                                    // For now, just acknowledge the action
                                    // Full game engine integration would apply the action here
                                    let _ = writeln!(out, "Action: {}", input);
                                    break; // Move to next hand
                                }
                                ParseResult::Quit => {
                                    quit_requested = true;
                                    break;
                                }
                                ParseResult::Invalid(msg) => {
                                    let _ = ui::write_error(err, &msg);
                                    // Re-prompt without terminating
                                }
                            }
                        }
                        None => {
                            // EOF - treat as quit
                            quit_requested = true;
                            break;
                        }
                    }
                }
            }
            Vs::Ai => {
                // AI mode: placeholder always checks
                let _ = writeln!(out, "{}", ui::tag_demo_output("ai: check"));
            }
        }
        played += 1;
    }

    let _ = writeln!(out, "Session hands={}", hands);
    let _ = writeln!(out, "Hands played: {} (completed)", played);
    0
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

fn validate_dealing_meta(
    meta: &serde_json::Map<String, serde_json::Value>,
    button: Option<&str>,
    starting_stacks: &HashMap<String, i64>,
    hand_index: u64,
) -> Result<(), String> {
    if starting_stacks.is_empty() {
        return Ok(());
    }
    let player_count = starting_stacks.len();
    let rounds = 2; // Texas Hold'em: two hole cards per player
    let sb = meta.get("small_blind").and_then(|v| v.as_str());
    let bb = meta.get("big_blind").and_then(|v| v.as_str());
    if let Some(sb_id) = sb {
        if !starting_stacks.contains_key(sb_id) {
            return Err(format!(
                "Invalid dealing order at hand {}: unknown small blind {}",
                hand_index, sb_id
            ));
        }
    }
    if let Some(bb_id) = bb {
        if !starting_stacks.contains_key(bb_id) {
            return Err(format!(
                "Invalid dealing order at hand {}: unknown big blind {}",
                hand_index, bb_id
            ));
        }
    }
    if let (Some(btn), Some(sb_id)) = (button, sb) {
        if sb_id != btn {
            return Err(format!(
                "Invalid dealing order at hand {}: button {} must match small blind {}",
                hand_index, btn, sb_id
            ));
        }
    }
    if let (Some(sb_id), Some(bb_id)) = (sb, bb) {
        if sb_id == bb_id {
            return Err(format!(
                "Invalid dealing order at hand {}: small blind and big blind must differ",
                hand_index
            ));
        }
        if player_count == 2 {
            if let Some(expected_bb) = starting_stacks
                .keys()
                .find(|id| id.as_str() != sb_id)
                .map(|s| s.as_str())
            {
                if bb_id != expected_bb {
                    return Err(format!(
                        "Invalid dealing order at hand {}: expected big blind {} but found {}",
                        hand_index, expected_bb, bb_id
                    ));
                }
            }
        }
    }
    if let Some(seq_val) = meta.get("deal_sequence") {
        let seq = seq_val.as_array().ok_or_else(|| {
            format!(
                "Invalid dealing order at hand {}: deal_sequence must be an array",
                hand_index
            )
        })?;
        let seq_ids: Option<Vec<&str>> = seq.iter().map(|v| v.as_str()).collect();
        let seq_ids = seq_ids.ok_or_else(|| {
            format!(
                "Invalid dealing order at hand {}: deal_sequence must contain player identifiers",
                hand_index
            )
        })?;
        let expected_len = player_count * rounds;
        if seq_ids.len() != expected_len {
            return Err(format!(
                "Invalid dealing order at hand {}: expected {} entries in deal_sequence but found {}",
                hand_index,
                expected_len,
                seq_ids.len()
            ));
        }
        let known: HashSet<&str> = starting_stacks.keys().map(|k| k.as_str()).collect();
        if seq_ids.iter().any(|id| !known.contains(id)) {
            return Err(format!(
                "Invalid dealing order at hand {}: deal_sequence references unknown player",
                hand_index
            ));
        }
        let first_round = &seq_ids[..player_count];
        if let Some(sb_id) = sb {
            if first_round.first().copied() != Some(sb_id) {
                return Err(format!(
                    "Invalid dealing order at hand {}: expected {} to receive the first card",
                    hand_index, sb_id
                ));
            }
        }
        if let Some(bb_id) = bb {
            if player_count >= 2 && first_round.get(1).copied() != Some(bb_id) {
                return Err(format!(
                    "Invalid dealing order at hand {}: expected {} to receive the second card",
                    hand_index, bb_id
                ));
            }
        }
        let first_round_set: HashSet<&str> = first_round.iter().copied().collect();
        if first_round_set.len() != player_count {
            return Err(format!(
                "Invalid dealing order at hand {}: duplicate players in first deal round",
                hand_index
            ));
        }
        for round_idx in 1..rounds {
            let chunk = &seq_ids[round_idx * player_count..(round_idx + 1) * player_count];
            if chunk != first_round {
                return Err(format!(
                    "Invalid dealing order at hand {}: inconsistent card distribution order",
                    hand_index
                ));
            }
        }
    }
    if let Some(burn_val) = meta.get("burn_positions") {
        let burn_arr = burn_val.as_array().ok_or_else(|| {
            format!(
                "Invalid dealing order at hand {}: burn_positions must be an array",
                hand_index
            )
        })?;
        let burn_positions: Option<Vec<i64>> = burn_arr.iter().map(|v| v.as_i64()).collect();
        let burn_positions = burn_positions.ok_or_else(|| {
            format!(
                "Invalid dealing order at hand {}: burn_positions must contain integers",
                hand_index
            )
        })?;
        if burn_positions.len() != 3 {
            return Err(format!(
                "Invalid dealing order at hand {}: expected 3 burn positions",
                hand_index
            ));
        }
        let player_count_i64 = player_count as i64;
        if player_count_i64 >= 2 {
            let hole_cards = player_count_i64 * 2;
            let expected = vec![
                hole_cards + 1,
                hole_cards + 1 + 3 + 1,
                hole_cards + 1 + 3 + 1 + 1 + 1,
            ];
            if burn_positions != expected {
                return Err(format!(
                    "Invalid dealing order at hand {}: expected burn positions {:?} but found {:?}",
                    hand_index, expected, burn_positions
                ));
            }
        }
    }
    Ok(())
}

fn validate_roster_state(
    prev: Option<&HashMap<String, i64>>,
    current: &HashMap<String, i64>,
    hands: u64,
    err: &mut dyn Write,
    ok: &mut bool,
) {
    if let Some(prev_map) = prev {
        for (id, stack_start) in current {
            if let Some(prev_stack) = prev_map.get(id) {
                if *prev_stack != *stack_start {
                    *ok = false;
                    let _ = ui::write_error(
                        err,
                        &format!("Stack mismatch for {} at hand {}", id, hands),
                    );
                }
                if *prev_stack <= 0 {
                    *ok = false;
                    let _ = ui::write_error(
                        err,
                        &format!(
                            "Player {} reappeared after elimination at hand {}",
                            id, hands
                        ),
                    );
                }
            } else {
                *ok = false;
                let _ =
                    ui::write_error(err, &format!("Unexpected player {} at hand {}", id, hands));
            }
        }
        for (id, prev_stack) in prev_map {
            if !current.contains_key(id) && *prev_stack > 0 {
                *ok = false;
                let _ = ui::write_error(err, &format!("Missing player {} at hand {}", id, hands));
            }
        }
    }
    for (id, stack_start) in current {
        if *stack_start <= 0 {
            *ok = false;
            let _ = ui::write_error(
                err,
                &format!(
                    "Player {} has non-positive starting stack at hand {}",
                    id, hands
                ),
            );
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
/// let args = vec!["axm", "deal", "--seed", "42"];
/// let code = axm_cli::run(args, &mut io::stdout(), &mut io::stderr());
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
    ) -> Option<i32> {
        use std::io::{BufRead, BufReader, BufWriter};

        let threshold = std::env::var("AXM_DATASET_STREAM_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(10_000);
        if threshold == 0 {
            return None;
        }

        let trace_stream = std::env::var("AXM_DATASET_STREAM_TRACE")
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
                let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                return Some(2);
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
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                        return Some(2);
                    }
                }
            }
        }

        if record_count == 0 {
            let _ = ui::write_error(err, "Empty input");
            return Some(2);
        }

        if record_count <= threshold {
            return None;
        }

        let splits = match compute_splits(train, val, test) {
            Ok(v) => v,
            Err(msg) => {
                let _ = ui::write_error(err, &msg);
                return Some(2);
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
            let _ = ui::write_error(
                err,
                &format!("Failed to create directory {}: {}", outdir, e),
            );
            return Some(2);
        }

        if trace_stream {
            let _ = ui::write_error(
                err,
                &format!("Streaming dataset input (records={})", record_count),
            );
        }

        let out_root = std::path::Path::new(outdir);
        let mut train_writer =
            BufWriter::new(std::fs::File::create(out_root.join("train.jsonl")).unwrap());
        let mut val_writer =
            BufWriter::new(std::fs::File::create(out_root.join("val.jsonl")).unwrap());
        let mut test_writer =
            BufWriter::new(std::fs::File::create(out_root.join("test.jsonl")).unwrap());

        let data_file = match std::fs::File::open(input) {
            Ok(f) => f,
            Err(e) => {
                let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                return Some(2);
            }
        };
        let reader = BufReader::new(data_file);
        let mut record_idx = 0usize;
        let mut first_line = true;

        for (line_idx, line_res) in reader.lines().enumerate() {
            let mut line = match line_res {
                Ok(line) => line,
                Err(e) => {
                    let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                    return Some(2);
                }
            };
            if first_line {
                strip_utf8_bom(&mut line);
                first_line = false;
            }
            if line.trim().is_empty() {
                continue;
            }
            if let Err(e) = serde_json::from_str::<axm_engine::logger::HandRecord>(&line) {
                let _ = ui::write_error(
                    err,
                    &format!("Invalid record at line {}: {}", line_idx + 1, e),
                );
                return Some(2);
            }
            let bucket = assignments
                .get(record_idx)
                .copied()
                .unwrap_or(SplitSlot::Test);
            record_idx += 1;
            match bucket {
                SplitSlot::Train => {
                    let _ = writeln!(train_writer, "{}", line);
                }
                SplitSlot::Val => {
                    let _ = writeln!(val_writer, "{}", line);
                }
                SplitSlot::Test => {
                    let _ = writeln!(test_writer, "{}", line);
                }
            }
        }

        Some(0)
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
    /// Exit code: `0` on success, `2` if errors detected
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
    /// let code = axm_cli::run(
    ///     vec!["axm", "stats", "--input", input],
    ///     &mut io::stdout(),
    ///     &mut io::stderr()
    /// );
    /// assert_eq!(code, 0);
    /// ```
    fn run_stats(input: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
        use std::path::Path;

        struct StatsState {
            hands: u64,
            p0: u64,
            p1: u64,
            skipped: u64,
            corrupted: u64,
            stats_ok: bool,
        }

        fn consume_stats_content(content: String, state: &mut StatsState, err: &mut dyn Write) {
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

                let rec: axm_engine::logger::HandRecord =
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
                            let _ = ui::write_error(
                                err,
                                &format!(
                                    "Invalid net_result value for {} at hand {}",
                                    player, rec.hand_id
                                ),
                            );
                        }
                    }
                    if sum != 0 {
                        state.stats_ok = false;
                        let _ = ui::write_error(
                            err,
                            &format!("Chip conservation violated at hand {}", rec.hand_id),
                        );
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
                        Ok(s) => consume_stats_content(s, &mut state, err),
                        Err(e) => {
                            let _ = ui::write_error(
                                err,
                                &format!("Failed to read {}: {}", p.display(), e),
                            );
                            state.stats_ok = false;
                        }
                    }
                }
            }
        } else {
            match read_text_auto(input) {
                Ok(s) => consume_stats_content(s, &mut state, err),
                Err(e) => {
                    let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                    return 2;
                }
            }
        }

        if state.corrupted > 0 {
            let _ = ui::write_error(
                err,
                &format!("Skipped {} corrupted record(s)", state.corrupted),
            );
        }
        if state.skipped > 0 {
            let _ = ui::write_error(
                err,
                &format!("Discarded {} incomplete final line(s)", state.skipped),
            );
        }
        if !path.is_dir() && state.hands == 0 && (state.corrupted > 0 || state.skipped > 0) {
            let _ = ui::write_error(err, "Invalid record");
            return 2;
        }

        let summary = serde_json::json!({
            "hands": state.hands,
            "winners": { "p0": state.p0, "p1": state.p1 },
        });
        let _ = writeln!(out, "{}", serde_json::to_string_pretty(&summary).unwrap());
        if state.stats_ok {
            0
        } else {
            2
        }
    }

    fn export_sqlite(content: &str, output: &str, err: &mut dyn Write) -> i32 {
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

                let record: axm_engine::logger::HandRecord = serde_json::from_str(raw)
                    .map_err(|e| ExportAttemptError::Fatal(format!("Invalid record: {}", e)))?;

                let axm_engine::logger::HandRecord {
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

        let max_attempts = std::env::var("AXM_EXPORT_SQLITE_RETRIES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(3);
        let backoff_ms = std::env::var("AXM_EXPORT_SQLITE_RETRY_SLEEP_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(50);

        for attempt in 1..=max_attempts {
            match export_sqlite_attempt(content, output) {
                Ok(()) => return 0,
                Err(ExportAttemptError::Busy(msg)) => {
                    if attempt == max_attempts {
                        let _ = ui::write_error(
                            err,
                            &format!("SQLite busy after {} attempt(s): {}", attempt, msg),
                        );
                        return 2;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(
                        backoff_ms * attempt as u64,
                    ));
                }
                Err(ExportAttemptError::Fatal(msg)) => {
                    let _ = ui::write_error(err, &msg);
                    return 2;
                }
            }
        }

        2
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
    /// - `AXM_DOCTOR_SQLITE_DIR`: Override SQLite check directory (default: temp dir)
    /// - `AXM_DOCTOR_DATA_DIR`: Override data directory path (default: `data/`)
    /// - `AXM_DOCTOR_LOCALE_OVERRIDE`: Force specific locale for testing
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::io;
    /// let code = axm_cli::run(
    ///     vec!["axm", "doctor"],
    ///     &mut io::stdout(),
    ///     &mut io::stderr()
    /// );
    /// assert_eq!(code, 0);
    /// ```
    fn run_doctor(out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
            let candidate = dir.join(format!("axm-doctor-{}.sqlite", unique_suffix()));
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
                return DoctorCheck::fail(
                    "data_dir",
                    format!("Data directory probe at {}", path.display()),
                    format!(
                        "Data directory check failed: {} does not exist",
                        path.display()
                    ),
                );
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
            let probe = path.join("axm-doctor-write.tmp");
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
                return evaluate_locale("AXM_DOCTOR_LOCALE_OVERRIDE", val);
            }
            for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
                if let Ok(val) = std::env::var(key) {
                    return evaluate_locale(key, val);
                }
            }
            let candidate =
                std::env::temp_dir().join(format!("axm-doctor-診断-{}.txt", unique_suffix()));
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

        let sqlite_dir = env::var("AXM_DOCTOR_SQLITE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::temp_dir());
        let data_dir = env::var("AXM_DOCTOR_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data"));
        let locale_override = env::var("AXM_DOCTOR_LOCALE_OVERRIDE").ok();

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
                    let _ = ui::write_error(err, msg);
                }
            }
            report.insert(check.name.to_string(), check.to_value());
        }

        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&serde_json::Value::Object(report)).unwrap()
        );

        if ok_all {
            0
        } else {
            2
        }
    }

    const COMMANDS: &[&str] = &[
        "play", "replay", "stats", "verify", "deal", "bench", "sim", "eval", "export", "dataset",
        "cfg", "doctor", "rng",
    ];
    let argv: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if argv.iter().any(|a| a == "--help" || a == "-h") {
        let _ = writeln!(out, "Axiomind Poker CLI\n");
        let _ = writeln!(out, "Usage: axm <command> [options]\n");
        let _ = writeln!(out, "Commands:");
        for c in COMMANDS {
            let _ = writeln!(out, "  {}", c);
        }
        let _ = writeln!(out, "\nOptions:\n  -h, --help     Show this help");
        return 0;
    }
    if argv.iter().any(|a| a == "--version" || a == "-V") {
        let _ = writeln!(out, "axm {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }

    let parsed = AxmCli::try_parse_from(&argv);
    match parsed {
        Err(e) => {
            // Print clap error first
            let _ = writeln!(err, "{}", e);
            // Then print an explicit help excerpt including the Commands list to stderr
            let _ = writeln!(err);
            let _ = writeln!(err, "Axiomind Poker CLI");
            let _ = writeln!(err, "Usage: axm <command> [options]\n");
            let _ = writeln!(err, "Commands:");
            for c in COMMANDS {
                let _ = writeln!(err, "  {}", c);
            }
            let _ = writeln!(err, "\nFor full help, run: axm --help");
            2
        }
        Ok(cli) => match cli.cmd {
            Commands::Cfg => match config::load_with_sources() {
                Ok(resolved) => {
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
                    let _ = writeln!(out, "{}", serde_json::to_string_pretty(&display).unwrap());
                    0
                }
                Err(e) => {
                    let _ = ui::write_error(err, &format!("Invalid configuration: {}", e));
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
                let level = level.unwrap_or(1);
                let non_tty_override = std::env::var("AXM_NON_TTY")
                    .ok()
                    .map(|v| {
                        let v = v.to_ascii_lowercase();
                        v == "1" || v == "true" || v == "yes" || v == "on"
                    })
                    .unwrap_or(false);
                if matches!(vs, Vs::Human) && (!std::io::stdin().is_terminal() || non_tty_override)
                {
                    let scripted = std::env::var("AXM_TEST_INPUT").ok();
                    if scripted.is_none() {
                        let _ =
                            ui::write_error(err, "Non-TTY environment: --vs human is not allowed");
                        return 2;
                    }
                }

                // Use stdin for real input
                let stdin = std::io::stdin();
                let mut stdin_lock = stdin.lock();
                execute_play_command(vs, hands, seed, level, &mut stdin_lock, out, err)
            }
            Commands::Replay { input, speed } => {
                // Display note about missing functionality
                let _ = writeln!(
                    err,
                    "Note: Full visual replay not yet implemented. This command only counts hands in the file."
                );

                match read_text_auto(&input) {
                    Ok(content) => {
                        // Validate speed via helper for clarity and future reuse
                        if let Err(msg) = validate_speed(speed) {
                            let _ = ui::write_error(err, &msg);
                            return 2;
                        }
                        let count = content.lines().filter(|l| !l.trim().is_empty()).count();
                        let _ = writeln!(out, "Counted: {} hands in file", count);
                        0
                    }
                    Err(e) => {
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                        2
                    }
                }
            }
            Commands::Stats { input } => run_stats(&input, out, err),
            Commands::Verify { input } => {
                // verify basic rule set covering board completion, chip conservation, and betting rules
                let mut ok = true;
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
                                ok = false;
                                let _ = ui::write_error(
                                    err,
                                    &format!("Hand {} recorded after player elimination", hands),
                                );
                            }
                            // parse as Value first to validate optional net_result chip conservation
                            let v: serde_json::Value = match serde_json::from_str(line) {
                                Ok(v) => v,
                                Err(_) => {
                                    ok = false;
                                    let _ = ui::write_error(err, "Invalid record");
                                    continue;
                                }
                            };
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
                                validate_roster_state(prev_state, &start_map, hands, err, &mut ok);
                                starting_stacks = Some(start_map.clone());
                                stacks_after_hand = start_map;
                                if let Some(nr_obj) =
                                    v.get("net_result").and_then(|x| x.as_object())
                                {
                                    for id in nr_obj.keys() {
                                        if !stacks_after_hand.contains_key(id) {
                                            ok = false;
                                            let _ = ui::write_error(
                                                err,
                                                &format!(
                                                    "Unknown player {} in net_result at hand {}",
                                                    id, hands
                                                ),
                                            );
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
                                    ok = false;
                                    let _ = ui::write_error(err, &msg);
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
                                        ok = false;
                                        let _ = ui::write_error(err, &msg);
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
                                    ok = false;
                                    let _ = ui::write_error(err, "Chip conservation violated");
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
                            match serde_json::from_value::<axm_engine::logger::HandRecord>(
                                v.clone(),
                            ) {
                                Ok(rec) => {
                                    if rec.board.len() != 5 {
                                        ok = false;
                                        let _ = ui::write_error(
                                            err,
                                            &format!(
                                                "Invalid board length at hand {}: expected 5 cards but found {}",
                                                hands,
                                                rec.board.len()
                                            ),
                                        );
                                    }

                                    let mut seen_cards: HashSet<axm_engine::cards::Card> =
                                        HashSet::new();
                                    let mut duplicate_cards: HashSet<axm_engine::cards::Card> =
                                        HashSet::new();
                                    {
                                        let mut record_card = |card: axm_engine::cards::Card| {
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
                                                            axm_engine::cards::Card,
                                                        >(
                                                            card_val.clone()
                                                        ) {
                                                            Ok(card) => record_card(card),
                                                            Err(_) => {
                                                                ok = false;
                                                                let _ = ui::write_error(
                                                                    err,
                                                                    &format!(
                                                                        "Invalid card specification for {} at hand {}",
                                                                        pid,
                                                                        hands
                                                                    ),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if !duplicate_cards.is_empty() {
                                        ok = false;
                                        let mut cards: Vec<String> = duplicate_cards
                                            .iter()
                                            .map(|card| format!("{:?} {:?}", card.rank, card.suit))
                                            .collect();
                                        cards.sort();
                                        let _ = ui::write_error(
                                            err,
                                            &format!(
                                                "Duplicate card(s) detected at hand {}: {}",
                                                hands,
                                                cards.join(", ")
                                            ),
                                        );
                                    }

                                    if !valid_id(&rec.hand_id) {
                                        ok = false;
                                        let _ = ui::write_error(err, "Invalid hand_id");
                                    }
                                }
                                Err(_) => {
                                    ok = false;
                                    let _ = ui::write_error(err, "Invalid record");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", path, e));
                        return 2;
                    }
                }
                let status = if ok { "OK" } else { "FAIL" };
                let _ = writeln!(out, "Verify: {} (hands={})", status, hands);
                if ok {
                    0
                } else {
                    2
                }
            }
            Commands::Doctor => run_doctor(out, err),
            Commands::Eval {
                ai_a,
                ai_b,
                hands,
                seed,
            } => {
                // Display placeholder warning
                let _ = ui::display_warning(
                    err,
                    "This is a placeholder returning random results. AI parameters are not used. For real simulations, use 'axm sim' command."
                );
                let _ = ui::warn_parameter_unused(err, "ai-a");
                let _ = ui::warn_parameter_unused(err, "ai-b");

                if ai_a == ai_b {
                    let _ = ui::write_error(err, "Warning: identical AI models");
                }
                let mut a_wins = 0u32;
                let mut b_wins = 0u32;
                let s = seed.unwrap_or_else(rand::random);
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(s);
                for _ in 0..hands {
                    if (rng.next_u32() & 1) == 0 {
                        a_wins += 1;
                    } else {
                        b_wins += 1;
                    }
                }
                let _ = writeln!(
                    out,
                    "Eval: hands={} A:{} B:{} [RANDOM RESULTS - NOT REAL AI COMPARISON]",
                    hands, a_wins, b_wins
                );
                0
            }
            Commands::Bench => {
                // quick bench: evaluate 200 unique 7-card draws from shuffled deck
                use axm_engine::cards::Card;
                use axm_engine::deck::Deck;
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
                    let _ = axm_engine::hand::evaluate_hand(&arr);
                    cnt += 1;
                }
                let dur = start.elapsed();
                let _ = writeln!(out, "Benchmark: {} iters in {:?}", cnt, dur);
                0
            }
            Commands::Deal { seed } => {
                let base_seed = seed.unwrap_or_else(rand::random);
                let mut eng = Engine::new(Some(base_seed), 1);
                eng.shuffle();
                let _ = eng.deal_hand();
                let p = eng.players();
                let hc1 = p[0].hole_cards();
                let hc2 = p[1].hole_cards();
                let fmt = |c: axm_engine::cards::Card| format!("{:?}{:?}", c.rank, c.suit);
                let _ = writeln!(
                    out,
                    "Hole P1: {} {}",
                    fmt(hc1[0].unwrap()),
                    fmt(hc1[1].unwrap())
                );
                let _ = writeln!(
                    out,
                    "Hole P2: {} {}",
                    fmt(hc2[0].unwrap()),
                    fmt(hc2[1].unwrap())
                );
                let b = eng.board();
                let _ = writeln!(
                    out,
                    "Board: {} {} {} {} {}",
                    fmt(b[0]),
                    fmt(b[1]),
                    fmt(b[2]),
                    fmt(b[3]),
                    fmt(b[4])
                );
                0
            }
            Commands::Rng { seed } => {
                let s = seed.unwrap_or_else(rand::random);
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(s);
                let mut vals = vec![];
                for _ in 0..5 {
                    vals.push(rng.next_u64());
                }
                let _ = writeln!(out, "RNG sample: {:?}", vals);
                0
            }
            Commands::Sim {
                hands,
                output,
                seed,
                level,
                resume,
            } => {
                let total: usize = hands as usize;
                if total == 0 {
                    let _ = ui::write_error(err, "hands must be >= 1");
                    return 2;
                }
                let level = level.unwrap_or(1);
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
                    if dups > 0 {
                        let _ = writeln!(err, "Warning: {} duplicate hand_id(s) skipped", dups);
                    }
                    let _ = writeln!(out, "Resumed from {}", completed);
                }
                let base_seed = seed.unwrap_or_else(rand::random);
                let mut eng = Engine::new(Some(base_seed), level);
                eng.shuffle();
                let break_after = std::env::var("AXM_SIM_BREAK_AFTER")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok());
                let per_hand_delay = std::env::var("AXM_SIM_SLEEP_MICROS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(std::time::Duration::from_micros);
                let fast_mode = std::env::var("AXM_SIM_FAST")
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
                    return sim_run_fast(
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
                    );
                }
                #[allow(clippy::mut_range_bound)]
                for i in completed..total {
                    // create a fresh engine per hand to avoid residual hole cards
                    let mut e = Engine::new(Some(base_seed + i as u64), level);
                    e.shuffle();
                    let _ = e.deal_hand();
                    if let Some(p) = &path {
                        let mut f = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(p)
                            .unwrap();
                        let hand_id = format!("19700101-{:06}", i + 1);
                        let board = e.board().clone();
                        let rec = serde_json::json!({
                            "hand_id": hand_id,
                            "seed": seed,
                            "level": level,
                            "actions": [],
                            "board": board,
                            "result": null,
                            "ts": null,
                            "meta": null
                        });
                        let _ = writeln!(f, "{}", serde_json::to_string(&rec).unwrap());
                    }
                    completed += 1;
                    if let Some(b) = break_after {
                        if completed == b {
                            let _ = writeln!(out, "Interrupted: saved {}/{}", completed, total);
                            return 130;
                        }
                    }
                }
                let _ = writeln!(out, "Simulated: {} hands", completed);
                0
            }
            Commands::Export {
                input,
                format,
                output,
            } => {
                let content = match std::fs::read_to_string(&input) {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                        return 2;
                    }
                };
                match format.as_str() {
                    f if f.eq_ignore_ascii_case("csv") => {
                        let mut w = std::fs::File::create(&output)
                            .map(std::io::BufWriter::new)
                            .map_err(|e| {
                                let _ = ui::write_error(
                                    err,
                                    &format!("Failed to write {}: {}", output, e),
                                );
                                e
                            })
                            .unwrap();
                        let _ = writeln!(w, "hand_id,seed,result,ts,actions,board");
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            let rec: axm_engine::logger::HandRecord =
                                serde_json::from_str(line).unwrap();
                            let seed = rec.seed.map(|v| v.to_string()).unwrap_or_else(|| "".into());
                            let result = rec.result.unwrap_or_default();
                            let ts = rec.ts.unwrap_or_default();
                            let _ = writeln!(
                                w,
                                "{},{},{},{},{},{}",
                                rec.hand_id,
                                seed,
                                result,
                                ts,
                                rec.actions.len(),
                                rec.board.len()
                            );
                        }
                        0
                    }
                    f if f.eq_ignore_ascii_case("json") => {
                        let mut arr = Vec::new();
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            let v: serde_json::Value = serde_json::from_str(line).unwrap();
                            arr.push(v);
                        }
                        let s = serde_json::to_string_pretty(&arr).unwrap();
                        std::fs::write(&output, s).unwrap();
                        0
                    }
                    f if f.eq_ignore_ascii_case("sqlite") => export_sqlite(&content, &output, err),
                    _ => {
                        let _ = ui::write_error(err, "Unsupported format");
                        2
                    }
                }
            }
            Commands::Dataset {
                input,
                outdir,
                train,
                val,
                test,
                seed,
            } => {
                if let Some(code) =
                    dataset_stream_if_needed(&input, &outdir, train, val, test, seed, err)
                {
                    return code;
                }
                let content = std::fs::read_to_string(&input)
                    .map_err(|e| {
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                        e
                    })
                    .unwrap();
                let mut lines: Vec<String> = content
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|s| s.to_string())
                    .collect();
                let n = lines.len();
                if n == 0 {
                    let _ = ui::write_error(err, "Empty input");
                    return 2;
                }
                let splits = match compute_splits(train, val, test) {
                    Ok(v) => v,
                    Err(msg) => {
                        let _ = ui::write_error(err, &msg);
                        return 2;
                    }
                };
                let tr = splits[0];
                let va = splits[1];
                let te = splits[2];
                let sum = tr + va + te;
                if (sum - 1.0).abs() > 1e-6 {
                    let _ = ui::write_error(err, "Splits must sum to 100% (1.0 total)");
                    return 2;
                }
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
                lines.shuffle(&mut rng);
                let n_tr = ((tr * n as f64).round() as usize).min(n);
                let n_va = ((va * n as f64).round() as usize).min(n.saturating_sub(n_tr));
                let _n_te = n.saturating_sub(n_tr + n_va);
                for (idx, raw) in lines.iter().enumerate() {
                    let trimmed = raw.trim();
                    if let Err(e) = serde_json::from_str::<axm_engine::logger::HandRecord>(trimmed)
                    {
                        let _ = ui::write_error(
                            err,
                            &format!("Invalid record at line {}: {}", idx + 1, e),
                        );
                        return 2;
                    }
                }
                let (trv, rest) = lines.split_at(n_tr);
                let (vav, tev) = rest.split_at(n_va);
                std::fs::create_dir_all(&outdir).unwrap();
                let write_split = |path: &std::path::Path, data: &[String]| {
                    let mut f = std::fs::File::create(path).unwrap();
                    for l in data {
                        let _ = writeln!(f, "{}", l);
                    }
                };
                write_split(&std::path::Path::new(&outdir).join("train.jsonl"), trv);
                write_split(&std::path::Path::new(&outdir).join("val.jsonl"), vav);
                write_split(&std::path::Path::new(&outdir).join("test.jsonl"), tev);
                0
            }
        },
    }
}

#[allow(clippy::too_many_arguments, clippy::mut_range_bound)]
fn sim_run_fast(
    total: usize,
    level: u8,
    seed: Option<u64>,
    base_seed: u64,
    break_after: Option<usize>,
    per_hand_delay: Option<std::time::Duration>,
    mut completed: usize,
    path: Option<&std::path::Path>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let mut writer = match path {
        Some(p) => match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            Ok(file) => Some(std::io::BufWriter::new(file)),
            Err(e) => {
                let _ = ui::write_error(err, &format!("Failed to open {}: {}", p.display(), e));
                return 2;
            }
        },
        None => None,
    };

    for i in completed..total {
        let mut engine = Engine::new(Some(base_seed + i as u64), level);
        engine.shuffle();
        let _ = engine.deal_hand();

        if let Some(w) = writer.as_mut() {
            let hand_id = format!("19700101-{:06}", i + 1);
            let board = engine.board().clone();
            let record = serde_json::json!({
                "hand_id": hand_id,
                "seed": seed,
                "level": level,
                "actions": [],
                "board": board,
                "result": null,
                "ts": null,
                "meta": null
            });
            if writeln!(w, "{}", serde_json::to_string(&record).unwrap()).is_err() {
                let _ = ui::write_error(err, "Failed to write simulation output");
                return 2;
            }
        }

        completed += 1;

        if let Some(delay) = per_hand_delay {
            std::thread::sleep(delay);
        }

        if let Some(b) = break_after {
            if completed == b {
                if let Some(w) = writer.as_mut() {
                    if w.flush().is_err() {
                        let _ = ui::write_error(err, "Failed to flush simulation output");
                        return 2;
                    }
                }
                let _ = writeln!(out, "Interrupted: saved {}/{}", completed, total);
                return 130;
            }
        }
    }

    if let Some(mut w) = writer {
        if w.flush().is_err() {
            let _ = ui::write_error(err, "Failed to flush simulation output");
            return 2;
        }
    }

    let _ = writeln!(out, "Simulated: {} hands", completed);
    0
}

#[derive(Parser, Debug)]
#[command(
    name = "axm",
    author = "Axiomind",
    version,
    about = "Axiomind Poker CLI",
    disable_help_flag = true
)]
struct AxmCli {
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
    /// * `--level` - Blind level (1-4, higher means bigger blinds)
    ///
    /// # Example
    ///
    /// ```bash
    /// axm play --vs ai --hands 10 --seed 42 --level 2
    /// ```
    Play {
        #[arg(long, value_enum)]
        vs: Vs,
        #[arg(long)]
        hands: Option<u32>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
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
    /// axm replay --input data/hands/session.jsonl --speed 2.0
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
    /// axm stats --input data/hands/
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
    /// axm eval --ai-a baseline --ai-b experimental --hands 1000 --seed 42
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
    /// axm verify --input data/hands/session.jsonl
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
    /// axm deal --seed 12345
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
    /// axm bench
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
    /// * `--level` - Blind level (1-4)
    /// * `--resume` - Resume from existing JSONL file (skips completed hands)
    ///
    /// # Environment Variables
    ///
    /// * `AXM_SIM_FAST` - Enable fast mode (batch writes, minimal output)
    /// * `AXM_SIM_BREAK_AFTER` - Break after N hands (for testing)
    /// * `AXM_SIM_SLEEP_MICROS` - Delay between hands in microseconds
    ///
    /// # Example
    ///
    /// ```bash
    /// axm sim --hands 10000 --output data/sim.jsonl --seed 42 --level 3
    /// ```
    Sim {
        #[arg(long)]
        hands: u64,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
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
    /// axm export --input data/hands.jsonl --format sqlite --output data/hands.db
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
    /// * `AXM_DATASET_STREAM_THRESHOLD` - Min records for streaming mode (default: 10000)
    /// * `AXM_DATASET_STREAM_TRACE` - Enable streaming debug output
    ///
    /// # Output Files
    ///
    /// Creates `train.jsonl`, `val.jsonl`, and `test.jsonl` in the output directory.
    ///
    /// # Example
    ///
    /// ```bash
    /// axm dataset --input data/sim.jsonl --outdir data/splits --train 0.7 --val 0.2 --test 0.1
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
    /// 1. Environment variables (e.g., `AXM_SEED`)
    /// 2. Config file (`~/.axm.toml` or project `.axm.toml`)
    /// 3. Built-in defaults
    ///
    /// # Example
    ///
    /// ```bash
    /// axm cfg
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
    /// axm doctor
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
    /// axm rng --seed 12345
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
    /// # use axm_cli::*;
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
        assert_eq!(result, None);
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
            ParseResult::Action(axm_engine::player::PlayerAction::Fold)
        ));
    }

    #[test]
    fn test_parse_check_case_insensitive() {
        let result = parse_player_action("CHECK");
        assert!(matches!(
            result,
            ParseResult::Action(axm_engine::player::PlayerAction::Check)
        ));
    }

    #[test]
    fn test_parse_call() {
        let result = parse_player_action("call");
        assert!(matches!(
            result,
            ParseResult::Action(axm_engine::player::PlayerAction::Call)
        ));
    }

    #[test]
    fn test_parse_bet_with_amount() {
        let result = parse_player_action("bet 100");
        match result {
            ParseResult::Action(axm_engine::player::PlayerAction::Bet(amount)) => {
                assert_eq!(amount, 100);
            }
            _ => panic!("Expected Bet action with amount 100"),
        }
    }

    #[test]
    fn test_parse_raise_with_amount() {
        let result = parse_player_action("raise 50");
        match result {
            ParseResult::Action(axm_engine::player::PlayerAction::Raise(amount)) => {
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

        assert_eq!(result, 0);
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

        assert_eq!(result, 0);
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

        assert_eq!(result, 0);
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

        assert_eq!(result, 0);
        let stderr_output = String::from_utf8(stderr).unwrap();
        assert!(stderr_output.contains("WARNING") || stderr_output.contains("placeholder"));
    }
}
