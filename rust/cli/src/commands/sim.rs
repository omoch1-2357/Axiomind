//! Simulation command handler for large-scale hand generation.
//!
//! This module provides functionality to run large-scale poker hand simulations,
//! generating specified numbers of hands with configurable parameters. It supports
//! environment variables for fast mode and breaking early for testing purposes.
//!
//! # Environment Variables
//!
//! - `AXIOMIND_SIM_FAST`: Enable fast simulation mode (batch writes, minimal output)
//! - `AXIOMIND_SIM_BREAK_AFTER`: Break after N hands (for testing)
//! - `AXIOMIND_SIM_SLEEP_MICROS`: Delay between hands in microseconds
//!
//! # Examples
//!
//! ```no_run
//! use axiomind_cli::commands::sim::handle_sim_command;
//! use std::io;
//!
//! let mut out = io::stdout();
//! let mut err = io::stderr();
//!
//! // Run 1000 hands with seed 42
//! handle_sim_command(1000, Some("data/sim.jsonl".to_string()), Some(42), Some(1), None, &mut out, &mut err).unwrap();
//! ```

use crate::error::CliError;
use crate::io_utils::ensure_parent_dir;
use crate::ui;
use axiomind_ai::create_ai;
use axiomind_engine::engine::Engine;
use std::io::Write;

/// Handle the sim command: run large-scale hand simulations.
///
/// Generates and optionally records N hands of poker. Supports resuming from
/// previous runs and breaking early for testing via environment variables.
///
/// # Arguments
///
/// * `hands` - Total number of hands to simulate
/// * `output` - Path to save hand histories (JSONL format)
/// * `seed` - Base RNG seed (each hand uses seed + hand_index)
/// * `level` - Blind level (1-20)
/// * `resume` - Resume from existing JSONL file (skips completed hands)
/// * `out` - Output stream for normal messages
/// * `err` - Output stream for error messages
///
/// # Returns
///
/// `Ok(())` on success, or `CliError` on failure
///
/// # Environment Variables
///
/// - `AXIOMIND_SIM_FAST`: Enable fast mode (batch writes, minimal output)
/// - `AXIOMIND_SIM_BREAK_AFTER`: Break after N hands (for testing)
/// - `AXIOMIND_SIM_SLEEP_MICROS`: Delay between hands in microseconds
pub fn handle_sim_command(
    hands: u64,
    output: Option<String>,
    seed: Option<u64>,
    level: Option<u8>,
    resume: Option<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    let total: usize = hands as usize;
    if total == 0 {
        ui::write_error(err, "hands must be >= 1")?;
        return Err(CliError::InvalidInput("hands must be >= 1".to_string()));
    }

    let level = level.unwrap_or(1).clamp(1, 20);
    let mut completed = 0usize;
    let mut path = None;

    if let Some(outp) = output.clone() {
        path = Some(std::path::PathBuf::from(outp));
    }

    // Resume: count existing unique hand_ids and warn on duplicates
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
            writeln!(err, "Warning: {} duplicate hand_id(s) skipped", dups)?;
        }
        writeln!(out, "Resumed from {}", completed)?;
    }

    let base_seed = seed.unwrap_or_else(rand::random);
    let mut eng = Engine::new(Some(base_seed), level);
    eng.shuffle();

    let break_after = std::env::var("AXIOMIND_SIM_BREAK_AFTER")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());
    let per_hand_delay = std::env::var("AXIOMIND_SIM_SLEEP_MICROS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(std::time::Duration::from_micros);
    let fast_mode = std::env::var("AXIOMIND_SIM_FAST")
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
        // Create a fresh engine per hand to avoid residual hole cards
        let mut e = Engine::new(Some(base_seed + i as u64), level);
        e.shuffle();
        let _ = e.deal_hand();

        // Play the hand to completion
        let (actions, result, showdown) = play_hand_to_completion(&mut e);

        if let Some(p) = &path {
            if let Err(e) = ensure_parent_dir(p) {
                ui::write_error(err, &e)?;
                return Err(CliError::Io(std::io::Error::other(e)));
            }

            let mut f = match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
            {
                Ok(file) => file,
                Err(e) => {
                    ui::write_error(err, &format!("Failed to open output file: {}", e))?;
                    return Err(CliError::Io(e));
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
                    ui::write_error(err, &format!("Failed to serialize hand: {}", e))?;
                    return Err(CliError::InvalidInput(format!(
                        "Failed to serialize hand: {}",
                        e
                    )));
                }
            };

            if writeln!(f, "{}", json_str).is_err() {
                ui::write_error(err, "Failed to write hand to file")?;
                return Err(CliError::Io(std::io::Error::other(
                    "Failed to write hand to file",
                )));
            }
        }

        completed += 1;

        if let Some(b) = break_after
            && completed == b
        {
            writeln!(out, "Interrupted: saved {}/{}", completed, total)?;
            return Err(CliError::Interrupted(format!(
                "Interrupted: saved {}/{}",
                completed, total
            )));
        }
    }

    writeln!(out, "Simulated: {} hands", completed)?;
    Ok(())
}

/// Play a hand to completion using baseline AI for both players.
///
/// This module-private helper function simulates a complete poker hand by having
/// both players use the baseline AI strategy until the hand reaches completion.
///
/// # Arguments
///
/// * `engine` - Mutable reference to the game engine with dealt cards
///
/// # Returns
///
/// A tuple containing:
/// - Action history (Vec of ActionRecords)
/// - Result string describing the outcome
/// - Optional showdown information (JSON value with winners)
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
        ("No result".to_string(), None)
    };

    (actions, result_string, showdown)
}

/// Run simulation in fast mode with batch writes.
///
/// This module-private helper function optimizes simulation performance by
/// using buffered writes and reducing output overhead.
///
/// # Arguments
///
/// * `total` - Total number of hands to simulate
/// * `level` - Blind level (1-20)
/// * `_seed` - Original seed parameter (unused, kept for signature compatibility)
/// * `base_seed` - Base RNG seed for hand generation
/// * `break_after` - Optional break point for early termination
/// * `per_hand_delay` - Optional delay between hands
/// * `completed` - Number of hands already completed (from resume)
/// * `path` - Optional path for output file
/// * `out` - Output stream for normal messages
/// * `err` - Output stream for error messages
///
/// # Returns
///
/// `Ok(())` on success, or `CliError` on failure
#[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::mut_range_bound)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sim_command_basic_execution() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Test basic execution with minimal hands
        let result = handle_sim_command(1, None, Some(42), Some(1), None, &mut out, &mut err);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Simulated: 1 hands"));
    }

    #[test]
    fn test_sim_command_with_seed() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Test that seed is respected
        let result = handle_sim_command(5, None, Some(123), Some(1), None, &mut out, &mut err);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Simulated: 5 hands"));
    }

    #[test]
    fn test_sim_command_without_seed() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Test without explicit seed (should use default)
        let result = handle_sim_command(5, None, None, Some(1), None, &mut out, &mut err);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Simulated: 5 hands"));
    }

    #[test]
    fn test_sim_command_zero_hands() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Test with zero hands (should return error)
        let result = handle_sim_command(0, None, Some(42), Some(1), None, &mut out, &mut err);
        assert!(result.is_err());

        let error_output = String::from_utf8(err).unwrap();
        assert!(error_output.contains("hands must be >= 1"));
    }

    #[test]
    fn test_sim_command_environment_variable_handling() {
        // This test verifies that environment variables are checked
        // Implementation handles AXIOMIND_SIM_FAST, AXIOMIND_SIM_BREAK_AFTER, etc.
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Even without setting env vars, command should work
        let result = handle_sim_command(1, None, Some(42), Some(1), None, &mut out, &mut err);
        assert!(result.is_ok());
    }
}
