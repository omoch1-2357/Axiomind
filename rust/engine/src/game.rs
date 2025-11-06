use crate::player::{Player, Position};

/// Represents the current state of a poker game including players and button position.
/// Manages button rotation and player position synchronization for heads-up play.
#[derive(Debug, Clone)]
pub struct GameState {
    /// Blind level (determines stakes)
    _level: u8,
    /// Index of the button player (0 or 1)
    button_index: usize,
    /// Array of exactly 2 players
    players: [Player; 2],
}

impl GameState {
    pub fn new(players: [Player; 2], level: u8) -> Self {
        // derive button_index from players' positions, default to 0
        let button_index = if players[1].position() == Position::Button {
            1
        } else {
            0
        };
        let mut gs = Self {
            _level: level,
            button_index,
            players,
        };
        // normalize positions based on button_index
        gs.sync_positions();
        gs
    }

    pub fn button_index(&self) -> usize {
        self.button_index
    }
    pub fn players(&self) -> &[Player; 2] {
        &self.players
    }

    pub fn rotate_button(&mut self) {
        self.button_index = 1 - self.button_index;
        self.sync_positions();
    }

    fn sync_positions(&mut self) {
        match self.button_index {
            0 => {
                self.players[0].set_position(Position::Button);
                self.players[1].set_position(Position::BigBlind);
            }
            _ => {
                self.players[0].set_position(Position::BigBlind);
                self.players[1].set_position(Position::Button);
            }
        }
    }
}
