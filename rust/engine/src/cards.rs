use serde::{Deserialize, Serialize};

/// Represents one of the four suits in a standard 52-card deck.
/// Used as a component of [`Card`] to fully define a playing card.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Suit {
    /// Clubs suit (♣)
    Clubs,
    /// Diamonds suit (♦)
    Diamonds,
    /// Hearts suit (♥)
    Hearts,
    /// Spades suit (♠)
    Spades,
}

/// Represents the rank (face value) of a playing card from Two through Ace.
/// Numeric values are assigned for comparison and hand evaluation purposes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Rank {
    /// Rank 2
    Two = 2,
    /// Rank 3
    Three,
    /// Rank 4
    Four,
    /// Rank 5
    Five,
    /// Rank 6
    Six,
    /// Rank 7
    Seven,
    /// Rank 8
    Eight,
    /// Rank 9
    Nine,
    /// Rank 10
    Ten,
    /// Jack (11)
    Jack,
    /// Queen (12)
    Queen,
    /// King (13)
    King,
    /// Ace (14)
    Ace,
}

impl Rank {
    pub fn from_u8(v: u8) -> Rank {
        match v {
            2 => Rank::Two,
            3 => Rank::Three,
            4 => Rank::Four,
            5 => Rank::Five,
            6 => Rank::Six,
            7 => Rank::Seven,
            8 => Rank::Eight,
            9 => Rank::Nine,
            10 => Rank::Ten,
            11 => Rank::Jack,
            12 => Rank::Queen,
            13 => Rank::King,
            _ => Rank::Ace,
        }
    }
}

/// Represents a single playing card with a suit and rank.
/// Cards are the fundamental unit of the poker game, used in player hands, the board, and the deck.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Card {
    /// The suit of the card (Clubs, Diamonds, Hearts, or Spades)
    pub suit: Suit,
    /// The rank of the card (Two through Ace)
    pub rank: Rank,
}

pub fn all_suits() -> [Suit; 4] {
    [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
}

pub fn all_ranks() -> [Rank; 13] {
    [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ]
}

pub fn full_deck() -> Vec<Card> {
    let mut v = Vec::with_capacity(52);
    for &s in &all_suits() {
        for &r in &all_ranks() {
            v.push(Card { suit: s, rank: r });
        }
    }
    v
}
