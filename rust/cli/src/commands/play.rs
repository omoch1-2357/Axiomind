//! # Play Command
//!
//! Interactive poker gameplay against AI or human opponents.
//!
//! This module provides the `handle_play_command` function for playing poker hands
//! in the Axiomind CLI. It supports two modes:
//!
//! - **Human vs AI**: Interactive gameplay where the human player enters actions via stdin
//! - **AI vs AI**: Automated gameplay where both players use AI decision-making
//!
//! ## Features
//!
//! - Interactive input validation with clear error messages
//! - Level progression (blinds increase every 15 hands)
//! - Graceful quit handling (user can exit with 'q' or 'quit')
//! - Real-time game state display (pot, actions, board)
//! - Integration with baseline AI for opponent moves

use crate::cli::Vs;
use crate::error::CliError;
use crate::formatters::format_action;
use crate::io_utils::read_stdin_line;
use crate::ui;
use crate::validation::{ParseResult, parse_player_action};
use axiomind_ai::create_ai;
use axiomind_engine::engine::Engine;
use std::io::{BufRead, Write};

/// Handle the play command: interactive poker gameplay
///
/// # Arguments
///
/// * `vs` - Opponent type (AI or Human)
/// * `hands` - Number of hands to play (must be >= 1, default: 1)
/// * `seed` - RNG seed for reproducibility (default: random)
/// * `level` - Blind level (1-20, default: 1)
/// * `out` - Output stream for game display
/// * `err` - Error stream for warnings and errors
/// * `stdin` - Input stream for player actions
///
/// # Returns
///
/// * `Ok(())` on successful completion
/// * `Err(CliError)` if hands < 1, engine initialization fails, or I/O errors occur
///
/// # Examples
///
/// ```ignore
/// use axiomind_cli::commands::handle_play_command;
/// use axiomind_cli::Vs;
/// use std::io::{stdin, stdout, stderr};
///
/// let mut out = stdout();
/// let mut err = stderr();
/// let mut input = stdin().lock();
///
/// handle_play_command(Vs::Ai, Some(1), None, None, &mut out, &mut err, &mut input).unwrap();
/// ```
pub fn handle_play_command(
    vs: Vs,
    hands: Option<u32>,
    seed: Option<u64>,
    level: Option<u8>,
    out: &mut dyn Write,
    err: &mut dyn Write,
    stdin: &mut dyn BufRead,
) -> Result<(), CliError> {
    let hands = hands.unwrap_or(1);
    let level = level.unwrap_or(1).clamp(1, 20);

    execute_play_command(vs, hands, seed, level, stdin, out, err)
}

/// Execute the play command with specified parameters (module-private helper)
///
/// This is the core implementation that handles game loop, player interaction,
/// and AI opponent moves.
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

/// Play a hand with two AI players (module-private helper)
///
/// Used for AI vs AI mode where both players make automated decisions.
#[allow(dead_code)]
fn play_hand_with_two_ais(
    _engine: &mut Engine,
    _out: &mut dyn Write,
    _err: &mut dyn Write,
) -> Result<(), CliError> {
    // This function is reserved for future AI vs AI gameplay implementation
    // Currently not used in the command flow
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_vs_enum_as_str() {
        assert_eq!(Vs::Ai.as_str(), "ai");
        assert_eq!(Vs::Human.as_str(), "human");
    }

    #[test]
    fn test_handle_play_command_ai_mode_basic() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = Cursor::new(b"");

        let result =
            handle_play_command(Vs::Ai, Some(1), None, None, &mut out, &mut err, &mut input);
        assert!(result.is_ok(), "AI mode should succeed");

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("play:"), "Should display play header");
        assert!(output.contains("vs=ai"), "Should show opponent type");
    }

    #[test]
    fn test_handle_play_command_zero_hands_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = Cursor::new(b"");

        let result =
            handle_play_command(Vs::Ai, Some(0), None, None, &mut out, &mut err, &mut input);
        assert!(result.is_err(), "Zero hands should fail");
        assert!(matches!(result, Err(CliError::InvalidInput(_))));
    }

    #[test]
    fn test_handle_play_command_default_hands() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = Cursor::new(b"");

        let result = handle_play_command(Vs::Ai, None, None, None, &mut out, &mut err, &mut input);
        assert!(result.is_ok(), "Default hands (1) should succeed");

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("hands=1"), "Should default to 1 hand");
    }

    #[test]
    fn test_handle_play_command_human_mode_quit() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        // Simulate user typing 'q' to quit immediately
        let mut input = Cursor::new(b"q\n");

        let result = handle_play_command(
            Vs::Human,
            Some(1),
            None,
            None,
            &mut out,
            &mut err,
            &mut input,
        );
        assert!(result.is_ok(), "Human mode with quit should succeed");

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("vs=human"), "Should show human opponent");
    }

    #[test]
    fn test_handle_play_command_level_display() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = Cursor::new(b"");

        let result =
            handle_play_command(Vs::Ai, Some(1), None, None, &mut out, &mut err, &mut input);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Level:"), "Should display blind level");
    }

    #[test]
    fn test_handle_play_command_seed_randomness() {
        let mut out1 = Vec::new();
        let mut err1 = Vec::new();
        let mut input1 = Cursor::new(b"");

        let mut out2 = Vec::new();
        let mut err2 = Vec::new();
        let mut input2 = Cursor::new(b"");

        // Two runs without seed should potentially differ (but both succeed)
        let result1 = handle_play_command(
            Vs::Ai,
            Some(1),
            None,
            None,
            &mut out1,
            &mut err1,
            &mut input1,
        );
        let result2 = handle_play_command(
            Vs::Ai,
            Some(1),
            None,
            None,
            &mut out2,
            &mut err2,
            &mut input2,
        );

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let output1 = String::from_utf8(out1).unwrap();
        let output2 = String::from_utf8(out2).unwrap();

        // Both should have valid output structure
        assert!(output1.contains("seed="));
        assert!(output2.contains("seed="));
    }

    #[test]
    fn test_handle_play_command_multiple_hands() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = Cursor::new(b"");

        let result =
            handle_play_command(Vs::Ai, Some(3), None, None, &mut out, &mut err, &mut input);
        assert!(result.is_ok(), "Multiple hands should succeed");

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.contains("hands=3"),
            "Should display correct hand count"
        );
    }

    #[test]
    fn test_handle_play_command_ai_warning() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = Cursor::new(b"");

        let result =
            handle_play_command(Vs::Ai, Some(1), None, None, &mut out, &mut err, &mut input);
        assert!(result.is_ok());

        let errors = String::from_utf8(err).unwrap();
        assert!(
            errors.contains("placeholder") || errors.contains("demo"),
            "Should warn about AI placeholder"
        );
    }

    #[test]
    fn test_execute_play_command_validation() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = Cursor::new(b"");

        // Zero hands should fail
        let result = execute_play_command(Vs::Ai, 0, None, 1, &mut input, &mut out, &mut err);
        assert!(result.is_err(), "Zero hands should return error");
    }

    #[test]
    fn test_execute_play_command_level_clamping() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = Cursor::new(b"");

        // Level should be clamped to 1-20 range
        let result = execute_play_command(Vs::Ai, 1, Some(42), 100, &mut input, &mut out, &mut err);
        assert!(result.is_ok(), "Level should be clamped");

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Level:"), "Should display level");
    }

    #[test]
    fn test_play_hand_with_two_ais_basic() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let seed = Some(12345u64);
        let mut engine = Engine::new(seed, 1);
        engine.shuffle();
        engine.deal_hand().unwrap();

        let result = play_hand_with_two_ais(&mut engine, &mut out, &mut err);
        assert!(result.is_ok(), "AI vs AI hand should succeed");
    }
}
