use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GameError {
    #[error("Invalid bet amount: {amount}, minimum: {minimum}")]
    InvalidBetAmount { amount: u32, minimum: u32 },
    #[error("Insufficient chips for action")]
    InsufficientChips,
    #[error("No hand in progress")]
    NoHandInProgress,
    #[error("Hand already complete")]
    HandAlreadyComplete,
    #[error("Player already folded")]
    PlayerAlreadyFolded,
    #[error("It's not player {actual}'s turn (expected player {expected})")]
    NotPlayersTurn { expected: usize, actual: usize },
}
