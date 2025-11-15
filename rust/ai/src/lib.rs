//! # axm-ai: AI Opponent System for Poker
//!
//! Provides AI opponent implementations for Texas Hold'em poker gameplay.
//! Supports multiple AI strategies with a common interface for decision-making.
//!
//! ## Core Components
//!
//! - [`AIOpponent`] - Trait defining the interface for AI decision-making
//! - [`baseline`] - Baseline AI implementation for testing and comparison
//! - [`create_ai`] - Factory function for creating AI opponents
//!
//! ## Quick Start
//!
//! ```rust
//! use axm_ai::{create_ai, AIOpponent};
//! use axm_engine::engine::Engine;
//!
//! // Create a baseline AI opponent
//! let ai = create_ai("baseline");
//!
//! // Use the AI to make decisions during gameplay
//! let mut engine = Engine::new(Some(42), 1);
//! engine.shuffle();
//! engine.deal_hand().expect("Failed to deal hand");
//!
//! let player_id = engine.current_player().expect("No current player");
//! let action = ai.get_action(&engine, player_id);
//! println!("AI chose action: {:?}", action);
//! ```
//!
//! ## AI Types
//!
//! Currently supported AI types:
//! - `"baseline"` - Simple baseline AI for testing and benchmarking

use axm_engine::engine::Engine;
use axm_engine::player::PlayerAction;

pub mod baseline;

/// Trait defining the interface for AI opponents in poker games.
/// Implementors must provide methods for decision-making and identification.
///
/// # Required Methods
///
/// - [`get_action`](AIOpponent::get_action) - Determine the next action based on game state
/// - [`name`](AIOpponent::name) - Return the AI's identifier/name
///
/// # Example Implementation
///
/// ```rust
/// use axm_ai::AIOpponent;
/// use axm_engine::engine::Engine;
/// use axm_engine::player::PlayerAction;
///
/// struct MyAI;
///
/// impl AIOpponent for MyAI {
///     fn get_action(&self, engine: &Engine, player_id: usize) -> PlayerAction {
///         // Simple strategy: always check or call
///         PlayerAction::Call
///     }
///
///     fn name(&self) -> &str {
///         "MyAI"
///     }
/// }
/// ```
pub trait AIOpponent: Send + Sync {
    /// Determine the next action for the AI player based on current game state.
    ///
    /// # Arguments
    ///
    /// * `engine` - Reference to the game engine containing current state
    /// * `player_id` - The player ID for which to determine an action (0 or 1)
    ///
    /// # Returns
    ///
    /// A `PlayerAction` representing the AI's chosen move
    ///
    /// # Example
    ///
    /// ```ignore
    /// let action = ai.get_action(&engine, 0);
    /// match action {
    ///     PlayerAction::Call => println!("AI calls"),
    ///     PlayerAction::Raise(amount) => println!("AI raises {}", amount),
    ///     _ => println!("AI takes other action"),
    /// }
    /// ```
    fn get_action(&self, engine: &Engine, player_id: usize) -> PlayerAction;

    /// Return the name/identifier of this AI implementation.
    ///
    /// # Returns
    ///
    /// A string slice containing the AI's name
    ///
    /// # Example
    ///
    /// ```ignore
    /// println!("Playing against: {}", ai.name());
    /// ```
    fn name(&self) -> &str;
}

/// Factory function to create AI opponents by type string.
///
/// # Arguments
///
/// * `ai_type` - String identifier for the AI type (e.g., "baseline")
///
/// # Returns
///
/// A boxed trait object implementing `AIOpponent`
///
/// # Supported AI Types
///
/// - `"baseline"` - Simple baseline AI for testing
///
/// # Example
///
/// ```rust
/// use axm_ai::create_ai;
///
/// let ai = create_ai("baseline");
/// assert_eq!(ai.name(), "BaselineAI");
/// ```
///
/// # Panics
///
/// Panics if an unknown AI type is requested. Currently only "baseline" is supported.
pub fn create_ai(ai_type: &str) -> Box<dyn AIOpponent> {
    match ai_type {
        "baseline" => Box::new(baseline::BaselineAI::new()),
        _ => panic!("Unknown AI type: {}", ai_type),
    }
}
