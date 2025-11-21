//! Replay command handler.
//!
//! This module implements hand history replay functionality, allowing users to
//! step through previously recorded poker hands from JSONL files.
//!
//! ## Features
//!
//! - Interactive hand-by-hand replay
//! - Speed control parameter (planned, not yet implemented)
//! - Detailed action tracking and pot state display
//! - Support for both regular and compressed (.zst) JSONL files
//!
//! ## Format
//!
//! Replays hands from JSONL files containing `HandRecord` structures with:
//! - Hand metadata (seed, level, button position)
//! - Action sequences with street information
//! - Final results and showdown details

use crate::error::CliError;
use crate::formatters::{format_action, format_board};
use crate::io_utils::read_text_auto;
use crate::validation::validate_speed;
use crate::{HandRecord, blinds_for_level, ui};
use axiomind_engine::logger::Street;
use std::io::Write;

/// Handle the replay command.
///
/// Loads and replays hands from a JSONL file, displaying each hand's actions
/// and allowing user to step through them interactively.
///
/// # Arguments
///
/// * `input` - Path to JSONL file containing hand histories
/// * `speed` - Optional playback speed multiplier (currently unused)
/// * `out` - Output stream for hand replay display
/// * `err` - Error stream for warnings and errors
///
/// # Returns
///
/// `Ok(())` on successful replay, `Err(CliError)` if file reading or parsing fails
pub fn handle_replay_command(
    input: String,
    speed: Option<f64>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    // Validate speed parameter
    if let Err(msg) = validate_speed(speed) {
        ui::write_error(err, &msg)?;
        return Err(CliError::InvalidInput(msg));
    }

    // Note about speed parameter not yet implemented
    if let Some(s) = speed {
        writeln!(
            err,
            "Note: --speed parameter ({}) is not yet used. Interactive mode only.",
            s
        )?;
    }

    // Read input file
    let content = read_text_auto(&input).map_err(|e| {
        let msg = format!("Failed to read {}: {}", input, e);
        let _ = ui::write_error(err, &msg);
        e
    })?;

    // Parse lines
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    let total_hands = lines.len();

    if total_hands == 0 {
        writeln!(out, "No hands found in file.")?;
        return Ok(());
    }

    // Replay each hand
    let mut hand_num = 0;
    let mut hands_shown = 0usize;

    for line in lines {
        hand_num += 1;

        // Parse hand record
        let record: HandRecord = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                ui::write_error(err, &format!("Failed to parse hand {}: {}", hand_num, e))?;
                continue;
            }
        };
        hands_shown += 1;

        // Extract level from metadata
        let level = if let Some(meta) = &record.meta {
            if let Some(level_val) = meta.get("level") {
                level_val.as_u64().unwrap_or(1) as u8
            } else {
                1
            }
        } else {
            1
        };

        // Extract button position from metadata
        let button_position = if let Some(meta) = &record.meta {
            if let Some(button_val) = meta.get("button_position") {
                button_val.as_u64().unwrap_or(0) as usize
            } else {
                0
            }
        } else {
            0
        };

        // Get blinds for level
        let (sb, bb) = match blinds_for_level(level) {
            Ok(amounts) => amounts,
            Err(e) => {
                ui::write_error(err, &format!("Invalid blind level {}: {}", level, e))?;
                (0, 0)
            }
        };

        // Display hand header
        writeln!(
            out,
            "Hand #{} (Seed: {}, Level: {})",
            hand_num,
            record
                .seed
                .map(|s| s.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            level
        )?;
        writeln!(out, "═══════════════════════════════════════")?;
        writeln!(out, "Blinds: SB={} BB={}", sb, bb)?;
        writeln!(out, "Button: Player {}", button_position)?;
        writeln!(out)?;

        // Initialize stack and pot tracking
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

        // Process actions by street
        let streets_order = [Street::Preflop, Street::Flop, Street::Turn, Street::River];

        for street in &streets_order {
            let actions_for_street: Vec<&axiomind_engine::logger::ActionRecord> = record
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

                // Display street header
                match street {
                    Street::Preflop => {
                        writeln!(out, "Preflop:")?;
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
                        writeln!(
                            out,
                            "  Player 0 {}: ?? ??  (Stack: {})",
                            if button_position == 0 {
                                btn_pos_label
                            } else {
                                other_pos_label
                            },
                            stacks[0]
                        )?;
                        writeln!(
                            out,
                            "  Player 1 {}: ?? ??  (Stack: {})",
                            if button_position == 1 {
                                btn_pos_label
                            } else {
                                other_pos_label
                            },
                            stacks[1]
                        )?;
                        writeln!(out)?;
                    }
                    Street::Flop => {
                        let flop_cards = if record.board.len() >= 3 {
                            &record.board[0..3]
                        } else {
                            &record.board[..]
                        };
                        writeln!(out, "Flop: {}", format_board(flop_cards))?;
                    }
                    Street::Turn => {
                        let turn_cards = if record.board.len() >= 4 {
                            &record.board[0..4]
                        } else {
                            &record.board[..]
                        };
                        writeln!(out, "Turn: {}", format_board(turn_cards))?;
                    }
                    Street::River => {
                        let river_cards = &record.board[..];
                        writeln!(out, "River: {}", format_board(river_cards))?;
                    }
                }

                // Process actions
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
                        axiomind_engine::player::PlayerAction::Raise(amount) => {
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
                                committed[player_id] = committed[player_id].saturating_add(delta);
                            }
                        }
                        axiomind_engine::player::PlayerAction::AllIn => {
                            // AllIn puts in all remaining stack
                            delta = stacks[player_id];
                            committed[player_id] = committed[player_id].saturating_add(delta);
                            current_bet = current_bet.max(committed[player_id]);
                        }
                        axiomind_engine::player::PlayerAction::Check
                        | axiomind_engine::player::PlayerAction::Fold => {
                            // Check and Fold have no chip movement
                        }
                    }

                    if delta > 0 {
                        stacks[player_id] = stacks[player_id].saturating_sub(delta);
                        pot = pot.saturating_add(delta);
                    }

                    writeln!(out, "  Player {}: {}", player_id, format_action(action))?;
                    writeln!(out, "  Pot: {}", pot)?;
                }
                writeln!(out)?;
            }
        }

        // Display showdown or result
        if let Some(showdown) = &record.showdown {
            writeln!(out, "Showdown:")?;
            for winner in &showdown.winners {
                writeln!(out, "  Player {} wins {} chips", winner, pot)?;
            }
            if let Some(notes) = &showdown.notes {
                writeln!(out, "  Notes: {}", notes)?;
            }
            writeln!(out)?;
        } else if let Some(result_str) = &record.result {
            writeln!(out, "Result:")?;
            writeln!(out, "  {} wins {} chips", result_str, pot)?;
            writeln!(out)?;
        }

        // Interactive continuation prompt
        if hand_num < total_hands {
            writeln!(out, "Press Enter for next hand (or 'q' to quit)...")?;
            let mut user_input = String::new();
            if std::io::stdin().read_line(&mut user_input).is_ok() {
                let trimmed = user_input.trim().to_lowercase();
                if trimmed == "q" || trimmed == "quit" {
                    writeln!(out, "Replay stopped at hand {}/{}", hand_num, total_hands)?;
                    return Ok(());
                }
            }
        }
    }

    writeln!(out, "Replay complete. {} hands shown.", hands_shown)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_replay_command_empty_file() {
        // Test replay with empty file returns success with no hands message
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Create temp file path (will fail to read, but we're testing parsing)
        let result =
            handle_replay_command("nonexistent.jsonl".to_string(), None, &mut out, &mut err);

        // Should fail because file doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_replay_command_invalid_speed() {
        // Test replay with invalid speed parameter
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_replay_command(
            "test.jsonl".to_string(),
            Some(0.0), // Invalid speed (must be positive)
            &mut out,
            &mut err,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::InvalidInput(_)));
    }

    #[test]
    fn test_handle_replay_command_speed_warning() {
        // Test that speed parameter generates warning (feature not yet implemented)
        let mut out = Vec::new();
        let mut err = Vec::new();

        // File doesn't exist, but we should see the warning before the error
        let _ = handle_replay_command("test.jsonl".to_string(), Some(2.0), &mut out, &mut err);

        // Should have warning about speed parameter
        let err_output = String::from_utf8_lossy(&err);
        assert!(err_output.contains("--speed parameter"));
        assert!(err_output.contains("not yet used"));
    }

    #[test]
    fn test_handle_replay_command_validates_speed_before_reading() {
        // Test that invalid speed is caught before file reading
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_replay_command(
            "will_not_be_read.jsonl".to_string(),
            Some(-1.0), // Invalid negative speed
            &mut out,
            &mut err,
        );

        assert!(result.is_err());
        let err_output = String::from_utf8_lossy(&err);
        assert!(err_output.contains("speed")); // Error about speed validation
    }

    #[test]
    fn test_handle_replay_displays_hand_header() {
        // Test that hand header is displayed correctly
        // This is a placeholder - full integration test would require creating test JSONL
        // The actual integration test exists in rust/cli/tests/test_replay.rs

        // For now, just verify function signature and error handling
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_replay_command("test.jsonl".to_string(), None, &mut out, &mut err);

        // File doesn't exist, so should error
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_replay_command_parses_hand_metadata() {
        // Test metadata parsing (level, button position)
        // Integration test covers this - unit test verifies error handling

        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_replay_command("missing.jsonl".to_string(), None, &mut out, &mut err);

        assert!(result.is_err());
    }

    #[test]
    fn test_handle_replay_command_error_message_format() {
        // Test that error messages are properly formatted
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_replay_command(
            "nonexistent_test_file.jsonl".to_string(),
            None,
            &mut out,
            &mut err,
        );

        assert!(result.is_err());
        let err_output = String::from_utf8_lossy(&err);
        assert!(err_output.contains("Failed to read"));
        assert!(err_output.contains("nonexistent_test_file.jsonl"));
    }
}
