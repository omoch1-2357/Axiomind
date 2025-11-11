use crate::cards::Card;
use crate::deck::Deck;
use crate::player::{Player, Position, STARTING_STACK};

/// Core game engine that orchestrates poker hand execution for heads-up play.
/// Manages the deck, two players, board cards, and hand dealing logic.
///
/// # Examples
///
/// ```
/// use axm_engine::engine::Engine;
///
/// // Create a new engine with a specific seed and blind level
/// let mut engine = Engine::new(Some(12345), 1);
///
/// // Shuffle the deck before starting a hand
/// engine.shuffle();
///
/// // Deal a complete hand (hole cards + flop + turn + river)
/// match engine.deal_hand() {
///     Ok(_) => {
///         // Hand dealt successfully
///         assert_eq!(engine.board().len(), 5);
///         assert!(engine.is_hand_complete());
///     }
///     Err(e) => println!("Failed to deal hand: {}", e),
/// }
/// ```
#[derive(Debug)]
pub struct Engine {
    /// The deck used for dealing cards
    deck: Deck,
    /// Array of exactly 2 players (heads-up poker)
    players: [Player; 2],
    /// Blind level (determines small blind and big blind amounts)
    _level: u8,
    /// Community cards on the board (up to 5 cards: flop, turn, river)
    board: Vec<Card>,
}

impl Engine {
    pub fn new(seed: Option<u64>, level: u8) -> Self {
        let seed = seed.unwrap_or(0xA1A2_A3A4);
        let deck = Deck::new_with_seed(seed);
        let players = [
            Player::new(0, STARTING_STACK, Position::Button),
            Player::new(1, STARTING_STACK, Position::BigBlind),
        ];
        Self {
            deck,
            players,
            _level: level,
            board: Vec::with_capacity(5),
        }
    }

    pub fn players(&self) -> &[Player; 2] {
        &self.players
    }
    pub fn players_mut(&mut self) -> &mut [Player; 2] {
        &mut self.players
    }

    pub fn shuffle(&mut self) {
        self.deck.shuffle();
    }

    pub fn draw_n(&mut self, n: usize) -> Vec<Card> {
        (0..n).filter_map(|_| self.deck.deal_card()).collect()
    }

    pub fn deal_hand(&mut self) -> Result<(), String> {
        // refuse to start a hand if any player's stack is zero
        if self.players.iter().any(|p| p.stack() == 0) {
            return Err("Player stack zero".to_string());
        }
        self.board.clear();
        // preflop: 2 cards each
        for _ in 0..2 {
            for p in &mut self.players {
                let c = self
                    .deck
                    .deal_card()
                    .ok_or_else(|| "deck empty".to_string())?;
                p.give_card(c)?;
            }
        }
        // flop
        self.deck.burn_card();
        for _ in 0..3 {
            let c = self
                .deck
                .deal_card()
                .ok_or_else(|| "deck empty".to_string())?;
            self.board.push(c);
        }
        // turn
        self.deck.burn_card();
        self.board.push(
            self.deck
                .deal_card()
                .ok_or_else(|| "deck empty".to_string())?,
        );
        // river
        self.deck.burn_card();
        self.board.push(
            self.deck
                .deal_card()
                .ok_or_else(|| "deck empty".to_string())?,
        );
        Ok(())
    }

    pub fn board(&self) -> &Vec<Card> {
        &self.board
    }

    pub fn is_hand_complete(&self) -> bool {
        self.board.len() == 5
    }

    pub fn deck_remaining(&self) -> usize {
        self.deck.remaining()
    }
}
