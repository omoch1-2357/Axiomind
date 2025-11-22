//! Verify command handler module.
//!
//! Validates hand history integrity and game rules compliance for JSONL hand history files.
//! This module performs comprehensive validation checks including:
//!
//! - Board completeness (exactly 5 cards)
//! - No duplicate cards across board and hole cards
//! - Chip conservation (net_result must sum to zero)
//! - Valid hand IDs (format: YYYYMMDD-NNNNNN)
//! - Betting rules compliance (no illegal reopening after short all-in)
//! - Player roster consistency across hands
//! - Street progression validation (Preflop → Flop → Turn → River)
//! - Stack continuity between hands
//!
//! Errors are collected using the shared `BatchValidationError` pattern for structured reporting.

use crate::error::{BatchValidationError, CliError};
use crate::io_utils::read_text_auto;
use crate::validation::validate_dealing_meta;
use std::collections::{HashMap, HashSet};
use std::io::Write;

/// Type alias for verify-specific batch validation errors.
/// The `usize` context represents the hand index (1-based) for error reporting.
type VerifyError = BatchValidationError<usize>;

/// Handle the verify command - validate hand history integrity.
///
/// Performs comprehensive validation on JSONL hand history files, checking game rules,
/// chip conservation laws, and data integrity. Reports all errors found with hand context.
///
/// # Arguments
///
/// * `input` - Path to JSONL file to verify
/// * `out` - Output stream for verification results (stdout)
/// * `err` - Output stream for error messages (stderr)
///
/// # Returns
///
/// `Result<(), CliError>`: `Ok(())` if all checks pass, otherwise an `Err` that maps to exit code `2`.
///
/// # Example
///
/// ```no_run
/// # use std::io;
/// # use axiomind_cli::commands::handle_verify_command;
/// let input = "data/hands/session.jsonl".to_string();
/// let result = handle_verify_command(input, &mut io::stdout(), &mut io::stderr());
/// ```
pub fn handle_verify_command(
    input: String,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    // Enhanced verification with error collection and comprehensive validations
    let mut errors: Vec<VerifyError> = Vec::new();
    let mut hands = 0u64;
    let mut game_over = false;
    let mut stacks_after_hand: HashMap<String, i64> = HashMap::new();
    const MIN_CHIP_UNIT: i64 = 25;

    let valid_id = |s: &str| -> bool {
        s.len() == 15
            && s[0..8].chars().all(|c| c.is_ascii_digit())
            && &s[8..9] == "-"
            && s[9..].chars().all(|c| c.is_ascii_digit())
    };

    let content = read_text_auto(&input)?;

    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        hands += 1;
        if game_over {
            errors.push(VerifyError {
                item_context: hands as usize,
                message: format!(
                    "Hand {} recorded after player elimination (zero stack)",
                    hands
                ),
            });
            continue;
        }

        // Parse as Value first to validate optional net_result chip conservation
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                errors.push(VerifyError {
                    item_context: hands as usize,
                    message: "Invalid JSON record".to_string(),
                });
                continue;
            }
        };

        // Extract hand_id early for better error reporting
        let _hand_id = v
            .get("hand_id")
            .and_then(|h| h.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Level 1 Validation: Check for missing result field
        let has_result = v.get("result").map(|r| !r.is_null()).unwrap_or(false);
        if !has_result {
            errors.push(VerifyError {
                item_context: hands as usize,
                message: "Missing or null result field".to_string(),
            });
        }

        // Level 2 Validation: Action sequence and street progression
        if let Some(actions) = v.get("actions").and_then(|a| a.as_array()) {
            let mut prev_street: Option<String> = None;
            let streets_order = ["Preflop", "Flop", "Turn", "River"];
            let mut street_index = 0;

            for action in actions {
                if let Some(street_str) = action.get("street").and_then(|s| s.as_str()) {
                    let current_street = street_str.to_string();

                    if let Some(ref prev) = prev_street {
                        if prev != &current_street {
                            // Street changed, validate progression
                            if let Some(current_idx) =
                                streets_order.iter().position(|s| s == &current_street)
                            {
                                if current_idx <= street_index {
                                    errors.push(VerifyError {
                                        item_context: hands as usize,
                                        message: format!(
                                            "Invalid street progression: {} appears after {}",
                                            current_street, prev
                                        ),
                                    });
                                } else if current_idx != street_index + 1 {
                                    errors.push(VerifyError {
                                        item_context: hands as usize,
                                        message: format!(
                                            "Street skipped: jumped from {} to {}",
                                            prev, current_street
                                        ),
                                    });
                                }
                                street_index = current_idx;
                            }
                        }
                    } else {
                        // First street
                        if let Some(idx) = streets_order.iter().position(|s| s == &current_street) {
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
                            errors.push(VerifyError {
                                item_context: hands as usize,
                                message: format!("Stack mismatch for {} at hand {}", id, hands),
                            });
                        }
                        if *prev_stack <= 0 {
                            errors.push(VerifyError {
                                item_context: hands as usize,
                                message: format!("Player {} reappeared after elimination", id),
                            });
                        }
                    } else {
                        errors.push(VerifyError {
                            item_context: hands as usize,
                            message: format!("Unexpected player {} at hand {}", id, hands),
                        });
                    }
                }
                for (id, prev_stack) in prev_map {
                    if !start_map.contains_key(id) && *prev_stack > 0 {
                        errors.push(VerifyError {
                            item_context: hands as usize,
                            message: format!("Missing player {} at hand {}", id, hands),
                        });
                    }
                }
            }
            for (id, stack_start) in &start_map {
                if *stack_start <= 0 {
                    errors.push(VerifyError {
                        item_context: hands as usize,
                        message: format!("Player {} has non-positive starting stack", id),
                    });
                }
            }

            starting_stacks = Some(start_map.clone());
            stacks_after_hand = start_map;
            if let Some(nr_obj) = v.get("net_result").and_then(|x| x.as_object()) {
                for id in nr_obj.keys() {
                    if !stacks_after_hand.contains_key(id) {
                        errors.push(VerifyError {
                            item_context: hands as usize,
                            message: format!("Unknown player {} in net_result", id),
                        });
                    }
                }
            }
        }

        if let (Some(start_map), Some(meta_obj)) = (
            starting_stacks.as_ref(),
            v.get("meta").and_then(|m| m.as_object()),
        ) {
            let button_id = v.get("button").and_then(|b| b.as_str());
            if let Err(msg) = validate_dealing_meta(meta_obj, button_id, start_map, hands) {
                errors.push(VerifyError {
                    item_context: hands as usize,
                    message: msg,
                });
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
            errors.push(VerifyError {
                item_context: hands as usize,
                message: msg,
            });
        }

        if let Some(nr) = v.get("net_result").and_then(|x| x.as_object()) {
            let mut sum: i64 = 0;
            for val in nr.values() {
                if let Some(n) = val.as_i64() {
                    sum += n;
                }
            }
            if sum != 0 {
                errors.push(VerifyError {
                    item_context: hands as usize,
                    message: "Chip conservation violated".to_string(),
                });
            }
            for (id, delta) in nr.iter() {
                if let Some(val) = delta.as_i64() {
                    let entry = stacks_after_hand.entry(id.clone()).or_insert(0);
                    *entry += val;
                }
            }
            if stacks_after_hand.values().any(|stack| *stack <= 0) {
                game_over = true;
            }
        }

        match serde_json::from_value::<axiomind_engine::logger::HandRecord>(v.clone()) {
            Ok(rec) => {
                if rec.board.len() != 5 {
                    errors.push(VerifyError {
                        item_context: hands as usize,
                        message: format!(
                            "Invalid board length: expected 5 cards but found {}",
                            rec.board.len()
                        ),
                    });
                }

                let mut seen_cards: HashSet<axiomind_engine::cards::Card> = HashSet::new();
                let mut duplicate_cards: HashSet<axiomind_engine::cards::Card> = HashSet::new();
                {
                    let mut record_card = |card: axiomind_engine::cards::Card| {
                        if !seen_cards.insert(card) {
                            duplicate_cards.insert(card);
                        }
                    };
                    for card in &rec.board {
                        record_card(*card);
                    }
                    if let Some(players) = v.get("players").and_then(|p| p.as_array()) {
                        for player in players {
                            let pid = player
                                .get("id")
                                .and_then(|x| x.as_str())
                                .unwrap_or("unknown");
                            if let Some(hole_cards) =
                                player.get("hole_cards").and_then(|h| h.as_array())
                            {
                                for card_val in hole_cards {
                                    match serde_json::from_value::<axiomind_engine::cards::Card>(
                                        card_val.clone(),
                                    ) {
                                        Ok(card) => record_card(card),
                                        Err(_) => {
                                            errors.push(VerifyError {
                                                item_context: hands as usize,
                                                message: format!(
                                                    "Invalid card specification for {}",
                                                    pid
                                                ),
                                            });
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
                    errors.push(VerifyError {
                        item_context: hands as usize,
                        message: format!("Duplicate card(s) detected: {}", cards.join(", ")),
                    });
                }

                if !valid_id(&rec.hand_id) {
                    errors.push(VerifyError {
                        item_context: hands as usize,
                        message: "Invalid hand_id format".to_string(),
                    });
                }
            }
            Err(_) => {
                errors.push(VerifyError {
                    item_context: hands as usize,
                    message: "Invalid record structure".to_string(),
                });
            }
        }
    }

    // Output results
    if errors.is_empty() {
        writeln!(out, "Verify: OK (hands={})", hands)?;
        Ok(())
    } else {
        writeln!(out, "Verify: FAIL (hands={})", hands)?;
        writeln!(err)?;
        writeln!(err, "Errors found:")?;
        for error in &errors {
            writeln!(err, "  Hand {}: {}", error.item_context, error.message)?;
        }
        writeln!(err)?;
        let invalid_hand_numbers: HashSet<usize> = errors.iter().map(|e| e.item_context).collect();
        let invalid_hands = invalid_hand_numbers.len() as u64;
        let percentage = if hands > 0 {
            (invalid_hands as f64 / hands as f64 * 100.0).round() as u32
        } else {
            0
        };
        writeln!(
            err,
            "Summary: {} error(s) in {} hands ({} invalid hands, {}% invalid)",
            errors.len(),
            hands,
            invalid_hands,
            percentage
        )?;
        Err(CliError::InvalidInput(format!(
            "{} validation errors found",
            errors.len()
        )))
    }
}

/// Module-private helper: Validate that no illegal reopening occurs after a short all-in.
///
/// In No-Limit Hold'em, a short all-in (less than a full raise) does not reopen betting
/// for players who have already acted. This function checks action sequences to ensure
/// this rule is enforced.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_verify_command_valid_file() {
        let input = "test_data/valid_hands.jsonl".to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Create valid test file
        std::fs::create_dir_all("test_data").ok();
        let valid_hand = r#"{"hand_id":"20250101-000001","board":[{"rank":"Ace","suit":"Spades"},{"rank":"King","suit":"Hearts"},{"rank":"Queen","suit":"Diamonds"},{"rank":"Jack","suit":"Clubs"},{"rank":"Ten","suit":"Spades"}],"result":"Player 0 wins 100","net_result":{"p0":50,"p1":-50},"players":[{"id":"p0","stack_start":1000,"hole_cards":[{"rank":"Ace","suit":"Hearts"},{"rank":"Ace","suit":"Diamonds"}]},{"id":"p1","stack_start":1000,"hole_cards":[{"rank":"Two","suit":"Clubs"},{"rank":"Three","suit":"Clubs"}]}],"actions":[]}"#;
        std::fs::write("test_data/valid_hands.jsonl", valid_hand).ok();

        let result = handle_verify_command(input, &mut out, &mut err);
        assert!(result.is_ok());
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Verify: OK"));

        // Cleanup
        std::fs::remove_file("test_data/valid_hands.jsonl").ok();
    }

    #[test]
    fn test_handle_verify_command_missing_file() {
        let input = "nonexistent.jsonl".to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_verify_command(input, &mut out, &mut err);
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_verify_command_invalid_json() {
        let input = "test_data/invalid.jsonl".to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Create test file with invalid JSON
        std::fs::create_dir_all("test_data").ok();
        std::fs::write("test_data/invalid.jsonl", "not valid json\n").ok();

        let result = handle_verify_command(input, &mut out, &mut err);
        assert!(result.is_err());
        let err_output = String::from_utf8(err).unwrap();
        assert!(err_output.contains("Invalid JSON record"));

        // Cleanup
        std::fs::remove_file("test_data/invalid.jsonl").ok();
    }

    #[test]
    fn test_verify_error_batch_validation_type() {
        // Test that VerifyError uses BatchValidationError<usize> correctly
        let error = VerifyError {
            item_context: 5,
            message: "Test error".to_string(),
        };

        assert_eq!(error.item_context, 5);
        assert_eq!(error.message, "Test error");
        assert_eq!(format!("{}", error), "5: Test error");
    }

    #[test]
    fn test_ensure_no_reopen_after_short_all_in() {
        let actions = vec![];
        let big_blind = 50;
        let min_chip_unit = 25;
        let starting_stacks = HashMap::new();
        let hand_number = 1;

        let result = ensure_no_reopen_after_short_all_in(
            &actions,
            big_blind,
            min_chip_unit,
            &starting_stacks,
            hand_number,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_hand_id_format() {
        let valid_id = |s: &str| -> bool {
            s.len() == 15
                && s[0..8].chars().all(|c| c.is_ascii_digit())
                && &s[8..9] == "-"
                && s[9..].chars().all(|c| c.is_ascii_digit())
        };

        assert!(valid_id("20250101-000001"));
        assert!(valid_id("19700101-123456"));
        assert!(!valid_id("2025-01-01-000001")); // Wrong format
        assert!(!valid_id("20250101000001")); // Missing dash
        assert!(!valid_id("20250101-abcdef")); // Non-digits
    }

    #[test]
    fn test_chip_conservation_validation() {
        // Test that net_result sum validation works
        let input = "test_data/chip_violation.jsonl".to_string();
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Create test file with chip conservation violation
        std::fs::create_dir_all("test_data").ok();
        let invalid_hand = r#"{"hand_id":"20250101-000001","board":[{"rank":"Ace","suit":"Spades"},{"rank":"King","suit":"Hearts"},{"rank":"Queen","suit":"Diamonds"},{"rank":"Jack","suit":"Clubs"},{"rank":"Ten","suit":"Spades"}],"result":"Player 0 wins","net_result":{"p0":100,"p1":-50},"players":[{"id":"p0","stack_start":1000},{"id":"p1","stack_start":1000}],"actions":[]}"#;
        std::fs::write("test_data/chip_violation.jsonl", invalid_hand).ok();

        let result = handle_verify_command(input, &mut out, &mut err);
        assert!(result.is_err());
        let err_output = String::from_utf8(err).unwrap();
        assert!(err_output.contains("Chip conservation violated"));

        // Cleanup
        std::fs::remove_file("test_data/chip_violation.jsonl").ok();
    }
}
