//! Card, board, and action formatters for terminal display.
//!
//! This module provides pure functions for formatting poker game elements
//! (cards, boards, actions) for terminal output. It supports Unicode card
//! symbols with ASCII fallback for terminal environments that don't support
//! Unicode rendering.
//!
//! ## Unicode vs ASCII Fallback
//!
//! The module automatically detects whether the terminal supports Unicode
//! symbols by checking environment variables on Windows (WT_SESSION, TERM_PROGRAM,
//! VSCODE_INJECTION) and assumes Unicode support on Unix-like systems.
//!
//! - **Unicode mode**: Uses ♥ ♦ ♣ ♠ symbols
//! - **ASCII mode**: Uses h d c s letters
//!
//! ## Example
//!
//! ```rust
//! use axiomind_engine::cards::{Card, Rank, Suit};
//! use axiomind_cli::formatters::{format_card, format_board};
//!
//! let ace_spades = Card { rank: Rank::Ace, suit: Suit::Spades };
//! assert!(format_card(&ace_spades) == "A♠" || format_card(&ace_spades) == "As");
//!
//! let board = vec![ace_spades];
//! assert!(format_board(&board).starts_with("[A"));
//! ```

use axiomind_engine::cards::{Card, Rank, Suit};

/// Check if the terminal supports Unicode card symbols by detecting modern terminal environments.
///
/// On Windows, checks for Windows Terminal (WT_SESSION), modern terminals (TERM_PROGRAM),
/// or VS Code (VSCODE_INJECTION). On Unix-like systems, assumes Unicode support.
///
/// # Returns
///
/// `true` if Unicode symbols are supported, `false` for ASCII fallback
pub fn supports_unicode() -> bool {
    if cfg!(windows) {
        std::env::var("WT_SESSION").is_ok()
            || std::env::var("TERM_PROGRAM").is_ok()
            || std::env::var("VSCODE_INJECTION").is_ok()
    } else {
        true
    }
}

/// Format a Suit as a string using Unicode symbols with ASCII fallback.
///
/// # Unicode symbols
/// - Hearts: ♥
/// - Diamonds: ♦
/// - Clubs: ♣
/// - Spades: ♠
///
/// # ASCII fallback
/// - Hearts: h
/// - Diamonds: d
/// - Clubs: c
/// - Spades: s
///
/// # Arguments
///
/// * `suit` - The suit to format
///
/// # Returns
///
/// Formatted suit as a String
pub fn format_suit(suit: &Suit) -> String {
    if supports_unicode() {
        match suit {
            Suit::Hearts => "♥",
            Suit::Diamonds => "♦",
            Suit::Clubs => "♣",
            Suit::Spades => "♠",
        }
        .to_string()
    } else {
        match suit {
            Suit::Hearts => "h",
            Suit::Diamonds => "d",
            Suit::Clubs => "c",
            Suit::Spades => "s",
        }
        .to_string()
    }
}

/// Format a Rank as a string (2-9, T, J, Q, K, A).
///
/// # Arguments
///
/// * `rank` - The rank to format
///
/// # Returns
///
/// Single-character string representation of the rank
pub fn format_rank(rank: &Rank) -> String {
    match rank {
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Eight => "8",
        Rank::Nine => "9",
        Rank::Ten => "T",
        Rank::Jack => "J",
        Rank::Queen => "Q",
        Rank::King => "K",
        Rank::Ace => "A",
    }
    .to_string()
}

/// Format a Card as a string combining rank and suit.
///
/// # Arguments
///
/// * `card` - The card to format
///
/// # Returns
///
/// String like "A♠" (Unicode) or "As" (ASCII)
///
/// # Example
///
/// ```rust
/// use axiomind_engine::cards::{Card, Rank, Suit};
/// # use axiomind_cli::formatters::format_card;
///
/// let ace_spades = Card { rank: Rank::Ace, suit: Suit::Spades };
/// let formatted = format_card(&ace_spades);
/// assert!(formatted == "A♠" || formatted == "As");
/// ```
pub fn format_card(card: &Card) -> String {
    format!("{}{}", format_rank(&card.rank), format_suit(&card.suit))
}

/// Format a board (list of cards) as a string in bracket notation.
///
/// # Arguments
///
/// * `cards` - Slice of cards representing the board
///
/// # Returns
///
/// Formatted board string like "[A♠ K♥ Q♦]" or "[]" if empty
///
/// # Example
///
/// ```rust
/// use axiomind_engine::cards::{Card, Rank, Suit};
/// # use axiomind_cli::formatters::format_board;
///
/// let flop = vec![
///     Card { rank: Rank::Ace, suit: Suit::Spades },
///     Card { rank: Rank::King, suit: Suit::Hearts },
///     Card { rank: Rank::Queen, suit: Suit::Diamonds },
/// ];
/// let formatted = format_board(&flop);
/// assert!(formatted.starts_with("[A"));
/// assert!(formatted.ends_with("]"));
/// ```
pub fn format_board(cards: &[Card]) -> String {
    if cards.is_empty() {
        "[]".to_string()
    } else {
        let formatted_cards: Vec<String> = cards.iter().map(format_card).collect();
        format!("[{}]", formatted_cards.join(" "))
    }
}

/// Format a PlayerAction as a human-readable string.
///
/// # Arguments
///
/// * `action` - The player action to format
///
/// # Returns
///
/// Formatted action string like "fold", "call", "bet 100", etc.
///
/// # Example
///
/// ```rust
/// use axiomind_engine::player::PlayerAction;
/// # use axiomind_cli::formatters::format_action;
///
/// assert_eq!(format_action(&PlayerAction::Fold), "fold");
/// assert_eq!(format_action(&PlayerAction::Bet(100)), "bet 100");
/// assert_eq!(format_action(&PlayerAction::AllIn), "all-in");
/// ```
pub fn format_action(action: &axiomind_engine::player::PlayerAction) -> String {
    match action {
        axiomind_engine::player::PlayerAction::Fold => "fold".to_string(),
        axiomind_engine::player::PlayerAction::Check => "check".to_string(),
        axiomind_engine::player::PlayerAction::Call => "call".to_string(),
        axiomind_engine::player::PlayerAction::Bet(amount) => format!("bet {}", amount),
        axiomind_engine::player::PlayerAction::Raise(amount) => format!("raise {}", amount),
        axiomind_engine::player::PlayerAction::AllIn => "all-in".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test formatter functions
    #[test]
    fn test_format_rank() {
        assert_eq!(format_rank(&Rank::Two), "2");
        assert_eq!(format_rank(&Rank::Ten), "T");
        assert_eq!(format_rank(&Rank::Jack), "J");
        assert_eq!(format_rank(&Rank::Queen), "Q");
        assert_eq!(format_rank(&Rank::King), "K");
        assert_eq!(format_rank(&Rank::Ace), "A");
    }

    #[test]
    fn test_format_suit_unicode_or_ascii() {
        // Test that format_suit returns valid output (either Unicode or ASCII)
        let hearts = format_suit(&Suit::Hearts);
        assert!(hearts == "♥" || hearts == "h");

        let diamonds = format_suit(&Suit::Diamonds);
        assert!(diamonds == "♦" || diamonds == "d");

        let clubs = format_suit(&Suit::Clubs);
        assert!(clubs == "♣" || clubs == "c");

        let spades = format_suit(&Suit::Spades);
        assert!(spades == "♠" || spades == "s");
    }

    #[test]
    fn test_format_card() {
        let ace_spades = Card {
            rank: Rank::Ace,
            suit: Suit::Spades,
        };
        let formatted = format_card(&ace_spades);
        assert!(formatted == "A♠" || formatted == "As");
    }

    #[test]
    fn test_format_board_empty() {
        let empty_board: Vec<Card> = vec![];
        assert_eq!(format_board(&empty_board), "[]");
    }

    #[test]
    fn test_format_board_with_cards() {
        let board = vec![
            Card {
                rank: Rank::Ace,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Hearts,
            },
        ];
        let formatted = format_board(&board);
        assert!(formatted.starts_with("[A"));
        assert!(formatted.contains("K"));
        assert!(formatted.ends_with("]"));
    }

    #[test]
    fn test_format_action_fold() {
        assert_eq!(
            format_action(&axiomind_engine::player::PlayerAction::Fold),
            "fold"
        );
    }

    #[test]
    fn test_format_action_check() {
        assert_eq!(
            format_action(&axiomind_engine::player::PlayerAction::Check),
            "check"
        );
    }

    #[test]
    fn test_format_action_call() {
        assert_eq!(
            format_action(&axiomind_engine::player::PlayerAction::Call),
            "call"
        );
    }

    #[test]
    fn test_format_action_bet() {
        assert_eq!(
            format_action(&axiomind_engine::player::PlayerAction::Bet(100)),
            "bet 100"
        );
    }

    #[test]
    fn test_format_action_raise() {
        assert_eq!(
            format_action(&axiomind_engine::player::PlayerAction::Raise(50)),
            "raise 50"
        );
    }

    #[test]
    fn test_format_action_allin() {
        assert_eq!(
            format_action(&axiomind_engine::player::PlayerAction::AllIn),
            "all-in"
        );
    }
}
