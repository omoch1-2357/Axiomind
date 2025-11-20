//! Deal command handler for single hand dealing and display.
//!
//! This module provides the `deal` command which deals a single poker hand
//! and displays the hole cards for both players and the complete board.
//! The command supports optional seeding for deterministic dealing.

use crate::error::CliError;
use axiomind_engine::engine::Engine;
use std::io::Write;

/// Handle the deal command.
///
/// Deals a single poker hand and displays the hole cards for both players
/// and the complete 5-card board. Supports optional seeding for deterministic
/// dealing and reproducibility.
///
/// # Arguments
///
/// * `seed` - Optional RNG seed for deterministic dealing
/// * `out` - Output stream for command results
///
/// # Returns
///
/// Returns `Ok(())` on success, or `CliError` on I/O errors.
///
/// # Examples
///
/// ```ignore
/// // Internal command handler - not part of public API
/// use axiomind_cli::commands::deal::handle_deal_command;
/// let mut out = Vec::new();
/// handle_deal_command(Some(42), &mut out).unwrap();
/// ```
pub fn handle_deal_command(seed: Option<u64>, out: &mut dyn Write) -> Result<(), CliError> {
    let base_seed = seed.unwrap_or_else(rand::random);
    let mut eng = Engine::new(Some(base_seed), 1);
    eng.shuffle();
    // Return value intentionally unused - engine state is what matters
    eng.deal_hand()?;
    let p = eng.players();
    let hc1 = p[0].hole_cards();
    let hc2 = p[1].hole_cards();

    let fmt = |c: axiomind_engine::cards::Card| format!("{:?}{:?}", c.rank, c.suit);

    let card1_p1 = hc1[0].ok_or_else(|| CliError::Internal("P1 hole card 1 missing".into()))?;
    let card2_p1 = hc1[1].ok_or_else(|| CliError::Internal("P1 hole card 2 missing".into()))?;
    let card1_p2 = hc2[0].ok_or_else(|| CliError::Internal("P2 hole card 1 missing".into()))?;
    let card2_p2 = hc2[1].ok_or_else(|| CliError::Internal("P2 hole card 2 missing".into()))?;

    writeln!(out, "Hole P1: {} {}", fmt(card1_p1), fmt(card2_p1))?;
    writeln!(out, "Hole P2: {} {}", fmt(card1_p2), fmt(card2_p2))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deal_command_with_seed() {
        // Test that deal command produces deterministic output with a seed
        let mut out = Vec::new();
        let result = handle_deal_command(Some(42), &mut out);

        assert!(result.is_ok(), "Deal command should succeed");

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.contains("Hole P1:"),
            "Output should contain P1 hole cards"
        );
        assert!(
            output.contains("Hole P2:"),
            "Output should contain P2 hole cards"
        );
        assert!(
            output.contains("Board:"),
            "Output should contain board cards"
        );
    }

    #[test]
    fn test_deal_command_deterministic() {
        // Test that same seed produces same output
        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        handle_deal_command(Some(12345), &mut out1).unwrap();
        handle_deal_command(Some(12345), &mut out2).unwrap();

        assert_eq!(out1, out2, "Same seed should produce identical output");
    }

    #[test]
    fn test_deal_command_without_seed() {
        // Test that deal command works without explicit seed
        let mut out = Vec::new();
        let result = handle_deal_command(None, &mut out);

        assert!(result.is_ok(), "Deal command should succeed without seed");

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.contains("Hole P1:"),
            "Output should contain P1 hole cards"
        );
        assert!(
            output.contains("Hole P2:"),
            "Output should contain P2 hole cards"
        );
        assert!(
            output.contains("Board:"),
            "Output should contain board cards"
        );
    }

    #[test]
    fn test_deal_command_output_format() {
        // Test that output contains exactly 3 lines (P1, P2, Board)
        let mut out = Vec::new();
        handle_deal_command(Some(999), &mut out).unwrap();

        let output = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 3, "Output should have exactly 3 lines");
        assert!(
            lines[0].starts_with("Hole P1:"),
            "First line should be P1 hole cards"
        );
        assert!(
            lines[1].starts_with("Hole P2:"),
            "Second line should be P2 hole cards"
        );
        assert!(lines[2].starts_with("Board:"), "Third line should be board");
    }
}
