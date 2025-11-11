use crate::cards::Card;
use serde::{Deserialize, Serialize};

/// Represents a player's position at the table in heads-up poker.
/// Button posts the small blind, BigBlind posts the big blind.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Position {
    /// Button position (small blind in heads-up)
    Button,
    /// Big blind position
    BigBlind,
}

/// Represents a player action during a betting round.
/// Actions can involve betting amounts or no-cost moves like check/fold.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlayerAction {
    /// Fold and forfeit the hand
    Fold,
    /// Check (no bet, only valid if no bet to call)
    Check,
    /// Call the current bet
    Call,
    /// Make a bet of specified amount
    Bet(u32),
    /// Raise the current bet by specified amount
    Raise(u32),
    /// Bet all remaining chips
    AllIn,
}

/// Default starting stack size for each player in chips
pub const STARTING_STACK: u32 = 20_000;

/// Represents a poker player with their chip stack, position, and hole cards.
/// Manages chip operations (betting, adding chips) and card management.
#[derive(Debug, Clone)]
pub struct Player {
    /// Player identifier (0 or 1 in heads-up)
    _id: usize,
    /// Current chip stack
    stack: u32,
    /// Table position (Button or BigBlind)
    position: Position,
    /// Hole cards (up to 2 cards)
    hole: [Option<Card>; 2],
}

impl Player {
    pub fn new(id: usize, stack: u32, position: Position) -> Self {
        Self {
            _id: id,
            stack,
            position,
            hole: [None, None],
        }
    }

    pub fn stack(&self) -> u32 {
        self.stack
    }
    pub fn position(&self) -> Position {
        self.position
    }
    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
    }

    pub fn hole_cards(&self) -> [Option<Card>; 2] {
        self.hole
    }

    pub fn give_card(&mut self, c: Card) -> Result<(), String> {
        if self.hole[0].is_none() {
            self.hole[0] = Some(c);
            Ok(())
        } else if self.hole[1].is_none() {
            self.hole[1] = Some(c);
            Ok(())
        } else {
            Err("Hole cards already full".to_string())
        }
    }

    pub fn clear_cards(&mut self) {
        self.hole = [None, None];
    }

    pub fn add_chips(&mut self, amount: u32) {
        self.stack = self.stack.saturating_add(amount);
    }

    pub fn bet(&mut self, amount: u32) -> Result<(), String> {
        if amount == 0 {
            return Ok(());
        }
        if amount > self.stack {
            return Err("Insufficient chips".to_string());
        }
        self.stack -= amount;
        Ok(())
    }
}
