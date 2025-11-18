//! # axiomind-engine: Poker Game Engine Core
//!
//! A deterministic Texas Hold'em poker engine for head-to-head (HU) play.
//! Provides game state management, hand evaluation, and comprehensive logging
//! with reproducible RNG for scientific comparison and debugging.
//!
//! ## Core Modules
//!
//! - [`cards`] - Card representation (Suit, Rank, Card) and deck construction
//! - [`deck`] - Deterministic deck shuffling with ChaCha8 RNG
//! - [`engine`] - Main game orchestration and hand execution
//! - [`game`] - Game state management and button rotation
//! - [`hand`] - Poker hand evaluation and strength comparison
//! - [`player`] - Player state, actions, and stack management
//! - [`pot`] - Pot calculation and side pot handling
//! - [`rules`] - Betting validation and blind structure
//! - [`logger`] - Event logging and HandRecord serialization
//! - [`errors`] - Error types for game operations
//!
//! ## Quick Start
//!
//! ```rust
//! use axiomind_engine::cards::{Card, Rank, Suit};
//! use axiomind_engine::hand::evaluate_hand;
//!
//! // Evaluate a 7-card poker hand
//! let cards = [
//!     Card { suit: Suit::Hearts, rank: Rank::Ace },
//!     Card { suit: Suit::Hearts, rank: Rank::King },
//!     Card { suit: Suit::Hearts, rank: Rank::Queen },
//!     Card { suit: Suit::Hearts, rank: Rank::Jack },
//!     Card { suit: Suit::Hearts, rank: Rank::Ten },
//!     Card { suit: Suit::Clubs, rank: Rank::Two },
//!     Card { suit: Suit::Diamonds, rank: Rank::Three },
//! ];
//!
//! let strength = evaluate_hand(&cards);
//! println!("Hand strength: {:?}", strength.category);
//! ```
//!
//! ## Deterministic Gameplay
//!
//! All game outcomes are reproducible using seeded RNG:
//!
//! ```rust
//! use axiomind_engine::deck::Deck;
//!
//! // Same seed produces same shuffle
//! let deck1 = Deck::new_with_seed(42);
//! let deck2 = Deck::new_with_seed(42);
//! // deck1 and deck2 will have identical card order
//! ```
//!
//! ## Action Validation
//!
//! Validate player actions according to betting rules:
//!
//! ```rust
//! use axiomind_engine::rules::validate_action;
//! use axiomind_engine::player::PlayerAction;
//!
//! let stack = 1000;
//! let to_call = 50;
//! let min_raise = 100;
//!
//! match validate_action(stack, to_call, min_raise, PlayerAction::Call) {
//!     Ok(validated) => println!("Valid action: {:?}", validated),
//!     Err(e) => println!("Invalid action: {}", e),
//! }
//! ```

pub mod cards;
pub mod deck;
pub mod engine;
pub mod errors;
pub mod game;
pub mod hand;
pub mod logger;
pub mod player;
pub mod pot;
pub mod rules;
