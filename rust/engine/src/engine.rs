use crate::cards::Card;
use crate::deck::Deck;
use crate::errors::GameError;
use crate::logger::{ActionRecord, Street};
use crate::player::{Player, PlayerAction, Position, STARTING_STACK};
use crate::rules::{ValidatedAction, validate_action};

/// Returns the small blind and big blind amounts for a given level.
///
/// # Supported Levels
/// - Levels 1-20: Predefined blind amounts with progressive increases
/// - Level 21+: Treated as level 20 (SB: 4000, BB: 8000)
/// - Level 0: Returns InvalidLevel error
///
/// # Arguments
/// * `level` - The blind level (1-255, where 21+ uses level 20 blinds)
///
/// # Returns
/// Returns a tuple of `(small_blind, big_blind)` amounts.
///
/// # Errors
/// Returns `GameError::InvalidLevel` if level is 0.
///
/// # Examples
/// ```
/// use axiomind_engine::engine::blinds_for_level;
///
/// // Level 1 blinds
/// assert_eq!(blinds_for_level(1).unwrap(), (50, 100));
///
/// // Level 20 blinds (maximum defined level)
/// assert_eq!(blinds_for_level(20).unwrap(), (4000, 8000));
///
/// // Level 21+ uses level 20 blinds
/// assert_eq!(blinds_for_level(21).unwrap(), (4000, 8000));
/// assert_eq!(blinds_for_level(100).unwrap(), (4000, 8000));
///
/// // Level 0 returns error
/// assert!(blinds_for_level(0).is_err());
/// ```
pub fn blinds_for_level(level: u8) -> Result<(u32, u32), GameError> {
    match level {
        0 => Err(GameError::InvalidLevel { level, minimum: 1 }),
        1 => Ok((50, 100)),
        2 => Ok((75, 150)),
        3 => Ok((100, 200)),
        4 => Ok((125, 250)),
        5 => Ok((150, 300)),
        6 => Ok((200, 400)),
        7 => Ok((250, 500)),
        8 => Ok((300, 600)),
        9 => Ok((400, 800)),
        10 => Ok((500, 1000)),
        11 => Ok((600, 1200)),
        12 => Ok((800, 1600)),
        13 => Ok((1000, 2000)),
        14 => Ok((1200, 2400)),
        15 => Ok((1500, 3000)),
        16 => Ok((2000, 4000)),
        17 => Ok((2500, 5000)),
        18 => Ok((3000, 6000)),
        19 => Ok((3500, 7000)),
        20 => Ok((4000, 8000)),
        // Levels 21 and above are treated as level 20 (maximum blind level)
        _ => Ok((4000, 8000)),
    }
}

/// Represents the state of a single betting round within a poker hand.
/// Tracks contributions, current bet level, and player states for one street.
#[derive(Debug, Clone)]
struct BettingRound {
    /// Current betting street (Preflop, Flop, Turn, or River)
    street: Street,
    /// Amount each player has contributed in this betting round
    contributions: [u32; 2],
    /// Current bet level that must be matched
    current_bet: u32,
    /// Minimum raise amount (typically the size of the last raise or big blind)
    min_raise: u32,
    /// Number of actions taken in this round (used to determine if betting is complete)
    actions_this_round: usize,
    /// Whether each player has folded
    folded: [bool; 2],
    /// Whether each player is all-in
    all_in: [bool; 2],
}

impl BettingRound {
    /// Create a new betting round for the specified street.
    /// For preflop, blinds are automatically posted.
    fn new(street: Street, level: u8, button_position: usize) -> Result<Self, GameError> {
        let (sb, bb) = blinds_for_level(level)?;

        // In heads-up poker:
        // - Button (small blind) acts first preflop
        // - Button acts last postflop
        // button_position: 0 = player 0 is button, 1 = player 1 is button

        let (contributions, current_bet, actions_this_round) = if street == Street::Preflop {
            // Post blinds: button posts small blind, other player posts big blind
            let mut contrib = [0u32; 2];
            contrib[button_position] = sb;
            contrib[1 - button_position] = bb;
            (contrib, bb, 0) // BB is the current bet, no actions yet
        } else {
            ([0, 0], 0, 0)
        };

        Ok(Self {
            street,
            contributions,
            current_bet,
            min_raise: bb, // Minimum raise is always at least the big blind
            actions_this_round,
            folded: [false, false],
            all_in: [false, false],
        })
    }

    /// Check if betting is complete for this round.
    /// Betting is complete when:
    /// - One player has folded, OR
    /// - Both players are all-in, OR
    /// - Both players have acted and contributions are equal (or one is all-in with less)
    fn is_complete(&self, active_player_count: usize) -> bool {
        // If someone folded, round is complete
        if self.folded[0] || self.folded[1] {
            return true;
        }

        // If both players are all-in, round is complete
        if self.all_in[0] && self.all_in[1] {
            return true;
        }

        // If only one active player (other is all-in but not folded), check if active player acted
        if active_player_count == 1 && self.actions_this_round > 0 {
            return true;
        }

        // Both players need to have acted at least once
        if self.actions_this_round < 2 {
            return false;
        }

        // Contributions must be equal, or one player must be all-in with less
        if self.contributions[0] == self.contributions[1] {
            return true;
        }

        // If contributions differ, the player with less must be all-in
        if self.contributions[0] < self.contributions[1] {
            self.all_in[0]
        } else {
            self.all_in[1]
        }
    }

    /// Calculate the amount a player needs to call
    fn to_call(&self, player_id: usize) -> u32 {
        self.current_bet
            .saturating_sub(self.contributions[player_id])
    }
}

/// Represents the complete state of a poker hand in progress.
/// Tracks all betting rounds, actions, and determines when the hand is complete.
#[derive(Debug, Clone)]
pub struct HandState {
    /// Current betting round
    betting_round: BettingRound,
    /// Complete history of all actions taken in this hand
    action_history: Vec<ActionRecord>,
    /// Total contributions from each player across all streets
    total_contributions: [u32; 2],
    /// Blind level for this hand
    level: u8,
    /// Button position (0 or 1)
    button_position: usize,
    /// Whether the hand has reached a terminal state
    is_complete: bool,
}

impl HandState {
    /// Create a new hand state, initializing with preflop betting round
    fn new(level: u8, button_position: usize) -> Result<Self, GameError> {
        let betting_round = BettingRound::new(Street::Preflop, level, button_position)?;

        // Initialize total contributions with posted blinds
        let (sb, bb) = blinds_for_level(level)?;
        let mut total_contributions = [0u32; 2];
        total_contributions[button_position] = sb;
        total_contributions[1 - button_position] = bb;

        Ok(Self {
            betting_round,
            action_history: Vec::new(),
            total_contributions,
            level,
            button_position,
            is_complete: false,
        })
    }

    /// Check if this hand has reached a terminal state
    pub fn is_hand_complete(&self) -> bool {
        self.is_complete
    }

    /// Get current street
    pub fn current_street(&self) -> Street {
        self.betting_round.street
    }

    /// Get total pot size
    pub fn pot(&self) -> u32 {
        self.total_contributions[0] + self.total_contributions[1]
    }

    /// Advance to the next betting street
    fn advance_street(&mut self) -> Result<(), GameError> {
        let next_street = match self.betting_round.street {
            Street::Preflop => Street::Flop,
            Street::Flop => Street::Turn,
            Street::Turn => Street::River,
            Street::River => {
                // Hand is complete after river betting
                self.is_complete = true;
                return Ok(());
            }
        };

        self.betting_round = BettingRound::new(next_street, self.level, self.button_position)?;
        Ok(())
    }

    fn current_actor(&self) -> usize {
        let first_to_act = match self.betting_round.street {
            Street::Preflop => self.button_position,
            _ => 1 - self.button_position,
        };
        if self.betting_round.actions_this_round.is_multiple_of(2) {
            first_to_act
        } else {
            1 - first_to_act
        }
    }
}

/// Core game engine that orchestrates poker hand execution for heads-up play.
/// Manages the deck, two players, board cards, and hand dealing logic.
///
/// # Examples
///
/// ```
/// use axiomind_engine::engine::Engine;
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
    level: u8,
    /// Community cards on the board (up to 5 cards: flop, turn, river)
    board: Vec<Card>,
    /// Current hand state (None if no hand in progress)
    hand_state: Option<HandState>,
    /// Button position for current/next hand (0 or 1)
    button_position: usize,
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
            level,
            board: Vec::with_capacity(5),
            hand_state: None,
            button_position: 0, // Player 0 starts as button
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

        // Always reshuffle to ensure a fresh deck for each hand
        self.deck.shuffle();

        // Clear previous hand state
        self.board.clear();
        for p in &mut self.players {
            p.clear_cards();
        }

        // Initialize hand state with preflop betting round
        self.hand_state =
            Some(HandState::new(self.level, self.button_position).map_err(|e| e.to_string())?);

        // Deduct blinds from player stacks
        let (sb, bb) = blinds_for_level(self.level).map_err(|e| e.to_string())?;
        self.players[self.button_position].bet(sb)?;
        self.players[1 - self.button_position].bet(bb)?;

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

    pub fn current_player(&self) -> Result<usize, GameError> {
        match self.hand_state.as_ref() {
            Some(hand_state) => Ok(hand_state.current_actor()),
            None => Err(GameError::NoHandInProgress),
        }
    }

    /// Get the current pot size
    /// Returns 0 if no hand is in progress
    pub fn pot(&self) -> u32 {
        self.hand_state.as_ref().map_or(0, |hs| hs.pot())
    }

    /// Get the current betting street.
    ///
    /// # Returns
    ///
    /// Returns `Some(Street)` if a hand is currently in progress, indicating which
    /// street the action is on (Preflop, Flop, Turn, or River).
    /// Returns `None` if no hand is currently in progress.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = Engine::new(Some(12345), 1);
    /// assert_eq!(engine.current_street(), None); // No hand in progress
    ///
    /// engine.shuffle();
    /// engine.deal_hand()?;
    /// assert_eq!(engine.current_street(), Some(Street::Preflop));
    /// ```
    pub fn current_street(&self) -> Option<Street> {
        self.hand_state.as_ref().map(|hs| hs.current_street())
    }

    /// Get the amount a specific player needs to call to match the current bet.
    ///
    /// # Arguments
    ///
    /// * `player_id` - The player's ID (0 or 1)
    ///
    /// # Returns
    ///
    /// Returns `Some(amount)` if a hand is in progress, indicating how many chips
    /// the specified player needs to contribute to match the current bet.
    /// Returns `None` if no hand is currently in progress.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = Engine::new(Some(12345), 1);
    /// engine.shuffle();
    /// engine.deal_hand()?;
    ///
    /// // Player 0 (button) needs to call 50 to match the big blind
    /// assert_eq!(engine.to_call(0), Some(50));
    /// // Player 1 (big blind) has already posted, so needs to call 0
    /// assert_eq!(engine.to_call(1), Some(0));
    /// ```
    pub fn to_call(&self, player_id: usize) -> Option<u32> {
        if player_id >= self.players.len() {
            return None;
        }
        self.hand_state
            .as_ref()
            .map(|hs| hs.betting_round.to_call(player_id))
    }

    /// Get the minimum raise amount for the current betting round.
    ///
    /// # Returns
    ///
    /// Returns `Some(amount)` if a hand is in progress, indicating the minimum
    /// number of chips that must be added on top of the call amount for a valid raise.
    /// Returns `None` if no hand is currently in progress.
    ///
    /// The minimum raise is typically the size of the last raise, or the big blind
    /// if no raises have occurred yet in the current betting round.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = Engine::new(Some(12345), 1);
    /// engine.shuffle();
    /// engine.deal_hand()?;
    ///
    /// // Minimum raise starts at the big blind (100 at level 1)
    /// assert_eq!(engine.min_raise(), Some(100));
    /// ```
    pub fn min_raise(&self) -> Option<u32> {
        self.hand_state
            .as_ref()
            .map(|hs| hs.betting_round.min_raise)
    }

    /// Get the current bet amount that players must match.
    ///
    /// # Returns
    ///
    /// Returns `Some(amount)` if a hand is in progress, indicating the total
    /// contribution required from each player to stay in the hand (before any raises).
    /// Returns `None` if no hand is currently in progress.
    ///
    /// This represents the current bet level that all active players must match.
    /// On preflop, this starts at the big blind amount.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = Engine::new(Some(12345), 1);
    /// engine.shuffle();
    /// engine.deal_hand()?;
    ///
    /// // Current bet starts at big blind (100 at level 1)
    /// assert_eq!(engine.current_bet(), Some(100));
    /// ```
    pub fn current_bet(&self) -> Option<u32> {
        self.hand_state
            .as_ref()
            .map(|hs| hs.betting_round.current_bet)
    }

    /// Apply a player action to the current hand state.
    /// Validates the action, updates player stacks and betting state, and progresses the hand.
    ///
    /// # Arguments
    ///
    /// * `player_id` - The player making the action (0 or 1)
    /// * `action` - The action to perform
    ///
    /// # Returns
    ///
    /// Returns a reference to the updated `HandState` if successful.
    ///
    /// # Errors
    ///
    /// Returns `GameError` if:
    /// - No hand is in progress
    /// - Action is invalid according to betting rules
    /// - Player has insufficient chips
    /// - Player has already folded
    /// - It's not the player's turn
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = Engine::new(Some(12345), 1);
    /// engine.shuffle();
    /// engine.deal_hand()?;
    ///
    /// // Player 0 (button) calls the big blind
    /// let state = engine.apply_action(0, PlayerAction::Call)?;
    /// assert_eq!(state.pot(), 200); // Both players have 100 in the pot
    /// ```
    pub fn apply_action(
        &mut self,
        player_id: usize,
        action: PlayerAction,
    ) -> Result<&HandState, GameError> {
        // Ensure a hand is in progress and the correct player is acting
        let expected_player = match self.hand_state.as_ref() {
            Some(state) => state.current_actor(),
            None => return Err(GameError::NoHandInProgress),
        };
        if expected_player != player_id {
            return Err(GameError::NotPlayersTurn {
                expected: expected_player,
                actual: player_id,
            });
        }
        let hand_state = self
            .hand_state
            .as_mut()
            .ok_or(GameError::NoHandInProgress)?;

        // Check if hand is already complete
        if hand_state.is_complete {
            return Err(GameError::HandAlreadyComplete);
        }

        // Check if player has folded
        if hand_state.betting_round.folded[player_id] {
            return Err(GameError::PlayerAlreadyFolded);
        }

        let player_stack = self.players[player_id].stack();
        let to_call = hand_state.betting_round.to_call(player_id);
        let min_raise = hand_state.betting_round.min_raise;

        // Validate the action
        let validated_action = validate_action(player_stack, to_call, min_raise, action.clone())?;

        // Apply the validated action
        let amount_contributed = match validated_action {
            ValidatedAction::Fold => {
                hand_state.betting_round.folded[player_id] = true;
                hand_state.is_complete = true; // Hand ends immediately on fold
                0
            }
            ValidatedAction::Check => 0,
            ValidatedAction::Call(amount) => {
                self.players[player_id]
                    .bet(amount)
                    .map_err(|_| GameError::InsufficientChips)?;
                hand_state.betting_round.contributions[player_id] += amount;
                amount
            }
            ValidatedAction::Bet(amount) => {
                self.players[player_id]
                    .bet(amount)
                    .map_err(|_| GameError::InsufficientChips)?;
                hand_state.betting_round.contributions[player_id] += amount;
                hand_state.betting_round.current_bet =
                    hand_state.betting_round.contributions[player_id];
                hand_state.betting_round.min_raise = amount; // Next raise must be at least this size
                amount
            }
            ValidatedAction::Raise(amount) => {
                let total_to_put_in = amount + to_call;
                self.players[player_id]
                    .bet(total_to_put_in)
                    .map_err(|_| GameError::InsufficientChips)?;
                hand_state.betting_round.contributions[player_id] += total_to_put_in;
                hand_state.betting_round.current_bet =
                    hand_state.betting_round.contributions[player_id];
                hand_state.betting_round.min_raise = amount; // Next raise must be at least this size
                total_to_put_in
            }
            ValidatedAction::AllIn(amount) => {
                self.players[player_id]
                    .bet(amount)
                    .map_err(|_| GameError::InsufficientChips)?;
                hand_state.betting_round.contributions[player_id] += amount;
                hand_state.betting_round.all_in[player_id] = true;

                // Update current bet if this all-in is larger
                if hand_state.betting_round.contributions[player_id]
                    > hand_state.betting_round.current_bet
                {
                    let raise_size = hand_state.betting_round.contributions[player_id]
                        - hand_state.betting_round.current_bet;
                    hand_state.betting_round.current_bet =
                        hand_state.betting_round.contributions[player_id];
                    // Only update min_raise if this was a full raise
                    if raise_size >= hand_state.betting_round.min_raise {
                        hand_state.betting_round.min_raise = raise_size;
                    }
                }
                amount
            }
        };

        // Update total contributions
        hand_state.total_contributions[player_id] += amount_contributed;

        // Record the action
        hand_state.action_history.push(ActionRecord {
            player_id,
            street: hand_state.betting_round.street,
            action,
        });

        // Increment action counter
        hand_state.betting_round.actions_this_round += 1;

        // Check if betting round is complete
        let active_player_count = hand_state
            .betting_round
            .folded
            .iter()
            .filter(|&&f| !f)
            .count();

        if hand_state.betting_round.is_complete(active_player_count) {
            // If hand is already complete (fold), don't advance streets
            if !hand_state.is_complete {
                hand_state.advance_street()?;
            }
        }

        self.hand_state.as_ref().ok_or(GameError::NoHandInProgress)
    }

    pub fn set_level(&mut self, level: u8) {
        self.level = level;
    }

    pub fn blinds(&self) -> Result<(u32, u32), GameError> {
        blinds_for_level(self.level)
    }

    /// Get the action history for the current/last hand
    pub fn action_history(&self) -> Vec<ActionRecord> {
        self.hand_state
            .as_ref()
            .map(|hs| hs.action_history.clone())
            .unwrap_or_default()
    }

    /// Get community cards on the board
    pub fn community_cards(&self) -> Vec<Card> {
        self.board.clone()
    }

    /// Check if hand reached showdown (completed without fold)
    pub fn reached_showdown(&self) -> bool {
        self.hand_state
            .as_ref()
            .map(|hs| hs.is_complete && !hs.betting_round.folded[0] && !hs.betting_round.folded[1])
            .unwrap_or(false)
    }

    /// Get the player who folded, if any
    pub fn folded_player(&self) -> Option<usize> {
        self.hand_state.as_ref().and_then(|hs| {
            if hs.betting_round.folded[0] {
                Some(0)
            } else if hs.betting_round.folded[1] {
                Some(1)
            } else {
                None
            }
        })
    }
}
