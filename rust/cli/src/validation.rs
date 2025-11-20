//! Input parsing and validation for interactive commands.
//!
//! This module provides functions for parsing and validating user input in
//! interactive CLI commands. It handles:
//! - Player action parsing (fold, call, bet, raise, etc.)
//! - Replay speed validation
//! - Dealing metadata validation for game setup
//!
//! ## Error Handling
//!
//! Validation functions return structured `Result` types or custom enums
//! (like `ParseResult`) to provide clear error messages to users.

use std::collections::{HashMap, HashSet};

/// Result type for parsing user input into player actions.
///
/// This enum represents the three possible outcomes when parsing user input
/// in interactive gameplay commands:
/// - Valid action (fold, call, bet, etc.)
/// - Quit command (user wants to exit)
/// - Invalid input with error message
#[derive(Debug, PartialEq)]
pub enum ParseResult {
    /// Valid player action parsed from input
    Action(axiomind_engine::player::PlayerAction),
    /// User entered quit command (q or quit)
    Quit,
    /// Invalid input with error message
    Invalid(String),
}

/// Parse user input string into a PlayerAction or special commands.
///
/// Accepts the following input formats (case-insensitive):
/// - "f" or "fold" → Fold
/// - "c", "call", or "check" → Call/Check
/// - "bet X" → Bet with amount X
/// - "raise X" → Raise with amount X
/// - "allin" or "all-in" → All-in
/// - "q" or "quit" → Quit command
///
/// # Arguments
///
/// * `input` - User input string to parse
///
/// # Returns
///
/// `ParseResult` indicating success, quit, or error with message
///
/// # Example
///
/// ```rust
/// # use axiomind_cli::validation::{parse_player_action, ParseResult};
/// use axiomind_engine::player::PlayerAction;
///
/// assert_eq!(
///     parse_player_action("fold"),
///     ParseResult::Action(PlayerAction::Fold)
/// );
///
/// assert_eq!(
///     parse_player_action("bet 100"),
///     ParseResult::Action(PlayerAction::Bet(100))
/// );
///
/// assert_eq!(parse_player_action("q"), ParseResult::Quit);
///
/// match parse_player_action("invalid") {
///     ParseResult::Invalid(msg) => assert!(msg.contains("Unrecognized")),
///     _ => panic!("Expected Invalid"),
/// }
/// ```
pub fn parse_player_action(input: &str) -> ParseResult {
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
        "fold" | "f" => ParseResult::Action(axiomind_engine::player::PlayerAction::Fold),
        "check" | "c" => ParseResult::Action(axiomind_engine::player::PlayerAction::Check),
        "call" => ParseResult::Action(axiomind_engine::player::PlayerAction::Call),
        "allin" | "all-in" => ParseResult::Action(axiomind_engine::player::PlayerAction::AllIn),
        "bet" => {
            if parts.len() < 2 {
                return ParseResult::Invalid(
                    "Bet requires an amount (e.g., 'bet 100')".to_string(),
                );
            }
            match parts[1].parse::<u32>() {
                Ok(amount) if amount > 0 => {
                    ParseResult::Action(axiomind_engine::player::PlayerAction::Bet(amount))
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
                    ParseResult::Action(axiomind_engine::player::PlayerAction::Raise(amount))
                }
                Ok(_) => ParseResult::Invalid("Raise amount must be positive".to_string()),
                Err(_) => ParseResult::Invalid("Invalid raise amount".to_string()),
            }
        }
        _ => ParseResult::Invalid(format!(
            "Unrecognized action '{}'. Valid actions: fold, check, call, bet <amount>, raise <amount>, allin, q",
            parts[0]
        )),
    }
}

/// Validate replay speed value (milliseconds).
///
/// Ensures the speed parameter is positive. Used by replay command to
/// validate user-provided speed parameter.
///
/// # Arguments
///
/// * `speed` - Optional speed value in arbitrary units (typically milliseconds)
///
/// # Returns
///
/// * `Ok(())` - Speed is valid (None or positive value)
/// * `Err(String)` - Speed is invalid (zero or negative) with error message
///
/// # Example
///
/// ```rust
/// # use axiomind_cli::validation::validate_speed;
///
/// assert!(validate_speed(Some(100.0)).is_ok());
/// assert!(validate_speed(None).is_ok());
/// assert!(validate_speed(Some(0.0)).is_err());
/// assert!(validate_speed(Some(-1.0)).is_err());
/// ```
pub fn validate_speed(speed: Option<f64>) -> Result<(), String> {
    if let Some(s) = speed
        && s <= 0.0
    {
        return Err("speed must be > 0".into());
    }
    Ok(())
}

/// Validate dealing metadata for game setup.
///
/// Ensures dealing order and blind positions are consistent for a poker hand.
/// This is used during hand verification to check that:
/// - Small blind and big blind are valid player IDs
/// - Button matches small blind (heads-up poker convention)
/// - Small blind and big blind are different players
/// - Big blind is the opponent in heads-up games
///
/// # Arguments
///
/// * `meta` - JSON metadata containing small_blind and big_blind fields
/// * `button` - Optional button player ID
/// * `starting_stacks` - Map of player IDs to their starting chip stacks
/// * `hand_index` - Hand number for error messages
///
/// # Returns
///
/// * `Ok(())` - Dealing metadata is valid
/// * `Err(String)` - Invalid dealing order with detailed error message
pub fn validate_dealing_meta(
    meta: &serde_json::Map<String, serde_json::Value>,
    button: Option<&str>,
    starting_stacks: &HashMap<String, i64>,
    hand_index: u64,
) -> Result<(), String> {
    if starting_stacks.is_empty() {
        return Ok(());
    }
    let player_count = starting_stacks.len();
    let sb = meta.get("small_blind").and_then(|v| v.as_str());
    let bb = meta.get("big_blind").and_then(|v| v.as_str());
    if let Some(sb_id) = sb
        && !starting_stacks.contains_key(sb_id)
    {
        return Err(format!(
            "Invalid dealing order at hand {}: unknown small blind {}",
            hand_index, sb_id
        ));
    }
    if let Some(bb_id) = bb
        && !starting_stacks.contains_key(bb_id)
    {
        return Err(format!(
            "Invalid dealing order at hand {}: unknown big blind {}",
            hand_index, bb_id
        ));
    }
    if let (Some(btn), Some(sb_id)) = (button, sb)
        && sb_id != btn
    {
        return Err(format!(
            "Invalid dealing order at hand {}: button {} must match small blind {}",
            hand_index, btn, sb_id
        ));
    }
    if let (Some(sb_id), Some(bb_id)) = (sb, bb) {
        if sb_id == bb_id {
            return Err(format!(
                "Invalid dealing order at hand {}: small blind and big blind must differ",
                hand_index
            ));
        }
        if player_count == 2
            && let Some(expected_bb) = starting_stacks
                .keys()
                .find(|id| id.as_str() != sb_id)
                .map(|s| s.as_str())
            && bb_id != expected_bb
        {
            return Err(format!(
                "Invalid dealing order at hand {}: big blind must be {}, got {}",
                hand_index, expected_bb, bb_id
            ));
        }
    }
    let rounds = 2; // Texas Hold'em: two hole cards per player
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
        if let Some(sb_id) = sb
            && first_round.first().copied() != Some(sb_id)
        {
            return Err(format!(
                "Invalid dealing order at hand {}: expected {} to receive the first card",
                hand_index, sb_id
            ));
        }
        if let Some(bb_id) = bb
            && player_count >= 2
            && first_round.get(1).copied() != Some(bb_id)
        {
            return Err(format!(
                "Invalid dealing order at hand {}: expected {} to receive the second card",
                hand_index, bb_id
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use axiomind_engine::player::PlayerAction;

    #[test]
    fn test_parse_fold() {
        assert_eq!(
            parse_player_action("fold"),
            ParseResult::Action(PlayerAction::Fold)
        );
        assert_eq!(
            parse_player_action("f"),
            ParseResult::Action(PlayerAction::Fold)
        );
    }

    #[test]
    fn test_parse_check_case_insensitive() {
        assert_eq!(
            parse_player_action("CHECK"),
            ParseResult::Action(PlayerAction::Check)
        );
        assert_eq!(
            parse_player_action("c"),
            ParseResult::Action(PlayerAction::Check)
        );
    }

    #[test]
    fn test_parse_call() {
        assert_eq!(
            parse_player_action("call"),
            ParseResult::Action(PlayerAction::Call)
        );
    }

    #[test]
    fn test_parse_bet_with_amount() {
        assert_eq!(
            parse_player_action("bet 100"),
            ParseResult::Action(PlayerAction::Bet(100))
        );
    }

    #[test]
    fn test_parse_raise_with_amount() {
        assert_eq!(
            parse_player_action("raise 50"),
            ParseResult::Action(PlayerAction::Raise(50))
        );
    }

    #[test]
    fn test_parse_quit_lowercase() {
        assert_eq!(parse_player_action("q"), ParseResult::Quit);
    }

    #[test]
    fn test_parse_quit_full() {
        assert_eq!(parse_player_action("quit"), ParseResult::Quit);
    }

    #[test]
    fn test_parse_quit_uppercase() {
        assert_eq!(parse_player_action("Q"), ParseResult::Quit);
    }

    #[test]
    fn test_parse_invalid_action() {
        match parse_player_action("invalid") {
            ParseResult::Invalid(msg) => assert!(msg.contains("Unrecognized")),
            _ => panic!("Expected Invalid result"),
        }
    }

    #[test]
    fn test_parse_bet_no_amount() {
        match parse_player_action("bet") {
            ParseResult::Invalid(msg) => assert!(msg.contains("requires an amount")),
            _ => panic!("Expected Invalid result"),
        }
    }

    #[test]
    fn test_parse_bet_negative_amount() {
        match parse_player_action("bet -100") {
            ParseResult::Invalid(_) => {} // Expected
            _ => panic!("Expected Invalid result for negative amount"),
        }
    }

    #[test]
    fn test_parse_bet_invalid_amount() {
        match parse_player_action("bet abc") {
            ParseResult::Invalid(msg) => assert!(msg.contains("Invalid bet amount")),
            _ => panic!("Expected Invalid result"),
        }
    }

    #[test]
    fn test_validate_speed_positive() {
        assert!(validate_speed(Some(100.0)).is_ok());
        assert!(validate_speed(Some(0.1)).is_ok());
    }

    #[test]
    fn test_validate_speed_none() {
        assert!(validate_speed(None).is_ok());
    }

    #[test]
    fn test_validate_speed_zero() {
        assert!(validate_speed(Some(0.0)).is_err());
    }

    #[test]
    fn test_validate_speed_negative() {
        assert!(validate_speed(Some(-1.0)).is_err());
    }

    #[test]
    fn test_validate_dealing_meta_empty_stacks() {
        let meta = serde_json::Map::new();
        let stacks = HashMap::new();
        assert!(validate_dealing_meta(&meta, None, &stacks, 0).is_ok());
    }

    #[test]
    fn test_validate_dealing_meta_valid_headsup() {
        let mut meta = serde_json::Map::new();
        meta.insert(
            "small_blind".to_string(),
            serde_json::Value::String("p0".to_string()),
        );
        meta.insert(
            "big_blind".to_string(),
            serde_json::Value::String("p1".to_string()),
        );

        let mut stacks = HashMap::new();
        stacks.insert("p0".to_string(), 1000);
        stacks.insert("p1".to_string(), 1000);

        assert!(validate_dealing_meta(&meta, Some("p0"), &stacks, 1).is_ok());
    }

    #[test]
    fn test_validate_dealing_meta_button_mismatch() {
        let mut meta = serde_json::Map::new();
        meta.insert(
            "small_blind".to_string(),
            serde_json::Value::String("p0".to_string()),
        );

        let mut stacks = HashMap::new();
        stacks.insert("p0".to_string(), 1000);
        stacks.insert("p1".to_string(), 1000);

        let result = validate_dealing_meta(&meta, Some("p1"), &stacks, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("button"));
    }

    #[test]
    fn test_validate_dealing_meta_same_sb_bb() {
        let mut meta = serde_json::Map::new();
        meta.insert(
            "small_blind".to_string(),
            serde_json::Value::String("p0".to_string()),
        );
        meta.insert(
            "big_blind".to_string(),
            serde_json::Value::String("p0".to_string()),
        );

        let mut stacks = HashMap::new();
        stacks.insert("p0".to_string(), 1000);

        let result = validate_dealing_meta(&meta, None, &stacks, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must differ"));
    }
}
