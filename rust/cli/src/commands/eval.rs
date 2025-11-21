//! AI policy evaluation command.
//!
//! This module provides functionality to evaluate AI policies head-to-head across multiple hands,
//! comparing their performance metrics including win rates, chip deltas, and action distributions.

use crate::error::CliError;
use axiomind_ai::create_ai;
use axiomind_engine::cards::Card;
use axiomind_engine::engine::Engine;
use axiomind_engine::hand::{compare_hands, evaluate_hand};
use axiomind_engine::logger::ActionRecord;
use std::io::Write;

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

    fn update_from_actions(&mut self, actions: &[ActionRecord], player_id: usize) {
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

    fn action_percentage(&self, count: u32) -> f64 {
        let total_actions =
            self.folds + self.checks + self.calls + self.bets + self.raises + self.all_ins;
        if total_actions == 0 {
            0.0
        } else {
            (count as f64 / total_actions as f64) * 100.0
        }
    }
}

/// Evaluates two AI policies head-to-head across multiple hands.
///
/// # Arguments
///
/// * `ai_a` - First AI policy identifier
/// * `ai_b` - Second AI policy identifier
/// * `hands` - Number of hands to play
/// * `seed` - Optional seed for reproducibility
/// * `out` - Output stream for evaluation results
///
/// # Returns
///
/// `Result<(), CliError>`: `Ok(())` when evaluation completes successfully.
pub fn handle_eval_command(
    ai_a: &str,
    ai_b: &str,
    hands: u32,
    seed: Option<u64>,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    // Create AI instances
    let ai_policy_a = match std::panic::catch_unwind(|| create_ai(ai_a)) {
        Ok(ai) => ai,
        Err(_) => {
            return Err(CliError::InvalidInput(format!("Unknown AI type: {}", ai_a)));
        }
    };

    let ai_policy_b = match std::panic::catch_unwind(|| create_ai(ai_b)) {
        Ok(ai) => ai,
        Err(_) => {
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
        let (actions, _result_string, showdown, pot) =
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
        } else if _result_string.contains("Player 0 wins") {
            (vec![0], false)
        } else if _result_string.contains("Player 1 wins") {
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

/// Play a hand with two AI players
fn play_hand_with_two_ais(
    engine: &mut Engine,
    ai_0: &dyn axiomind_ai::AIOpponent,
    ai_1: &dyn axiomind_ai::AIOpponent,
) -> (Vec<ActionRecord>, String, Option<serde_json::Value>, u32) {
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
        let winner = 1 - folded;
        (format!("Player {} wins {} (fold)", winner, pot), None)
    } else if engine.reached_showdown() {
        let players = engine.players();
        let board = engine.community_cards();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_basic_execution() {
        let mut out = Vec::new();

        let result = handle_eval_command("baseline", "baseline", 10, Some(12345), &mut out);

        assert!(result.is_ok());
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("AI Comparison Results"));
        assert!(output.contains("Hands played: 10"));
        assert!(output.contains("Seed: 12345"));
    }

    #[test]
    fn test_eval_stats_structure() {
        let stats = EvalStats::new();

        assert_eq!(stats.hands_played, 0);
        assert_eq!(stats.wins, 0);
        assert_eq!(stats.losses, 0);
        assert_eq!(stats.ties, 0);
    }

    #[test]
    fn test_eval_stats_update() {
        let mut stats = EvalStats::new();

        stats.update_result(true, false, 100, 200);

        assert_eq!(stats.hands_played, 1);
        assert_eq!(stats.wins, 1);
        assert_eq!(stats.losses, 0);
        assert_eq!(stats.total_chips_won, 100);
    }

    #[test]
    fn test_eval_stats_tie() {
        let mut stats = EvalStats::new();

        stats.update_result(false, true, 0, 200);

        assert_eq!(stats.hands_played, 1);
        assert_eq!(stats.wins, 0);
        assert_eq!(stats.ties, 1);
    }

    #[test]
    fn test_eval_win_rate_calculation() {
        let mut stats = EvalStats::new();

        stats.update_result(true, false, 100, 200);
        stats.update_result(false, false, -100, 200);
        stats.update_result(true, false, 100, 200);

        assert_eq!(stats.hands_played, 3);
        assert_eq!(stats.wins, 2);
        assert!((stats.win_rate() - 66.7).abs() < 0.1);
    }

    #[test]
    fn test_eval_deterministic() {
        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        let _ = handle_eval_command("baseline", "baseline", 5, Some(999), &mut out1);
        let _ = handle_eval_command("baseline", "baseline", 5, Some(999), &mut out2);

        let output1 = String::from_utf8(out1).unwrap();
        let output2 = String::from_utf8(out2).unwrap();

        // Same seed should produce same results
        assert_eq!(output1, output2);
    }

    #[test]
    fn test_eval_zero_hands() {
        let mut out = Vec::new();

        let result = handle_eval_command("baseline", "baseline", 0, Some(12345), &mut out);

        // Should complete without error
        assert!(result.is_ok());
    }
}
