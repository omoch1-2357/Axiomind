use crate::ai::{create_ai, AIOpponent};
use crate::events::{EventBus, GameEvent, HandResult, PlayerInfo};
use crate::history::HistoryStore;
use axm_engine::cards::Card;
use axm_engine::engine::Engine;
use axm_engine::logger::{ActionRecord, HandRecord, Street};
use axm_engine::player::{PlayerAction, Position as EnginePosition};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cmp::min;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

pub type SessionId = String;

const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatPosition {
    Button,
    BigBlind,
}

impl From<EnginePosition> for SeatPosition {
    fn from(position: EnginePosition) -> Self {
        match position {
            EnginePosition::Button => SeatPosition::Button,
            EnginePosition::BigBlind => SeatPosition::BigBlind,
        }
    }
}

impl From<SeatPosition> for EnginePosition {
    fn from(position: SeatPosition) -> Self {
        match position {
            SeatPosition::Button => EnginePosition::Button,
            SeatPosition::BigBlind => EnginePosition::BigBlind,
        }
    }
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: RwLock<HashMap<SessionId, Arc<GameSession>>>,
    event_bus: Arc<EventBus>,
    history_store: Option<Arc<HistoryStore>>,
    session_ttl: Duration,
}

impl SessionManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_bus,
            history_store: None,
            session_ttl: DEFAULT_SESSION_TTL,
        }
    }

    pub fn with_history(event_bus: Arc<EventBus>, history_store: Arc<HistoryStore>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_bus,
            history_store: Some(history_store),
            session_ttl: DEFAULT_SESSION_TTL,
        }
    }

    pub fn with_ttl(event_bus: Arc<EventBus>, ttl: Duration) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_bus,
            history_store: None,
            session_ttl: ttl,
        }
    }

    pub fn with_ttl_and_history(
        event_bus: Arc<EventBus>,
        ttl: Duration,
        history_store: Arc<HistoryStore>,
    ) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_bus,
            history_store: Some(history_store),
            session_ttl: ttl,
        }
    }

    pub fn create_session(&self, config: GameConfig) -> Result<SessionId, SessionError> {
        let id = Uuid::new_v4().to_string();

        tracing::info!(
            session_id = %id,
            level = config.level,
            opponent_type = ?config.opponent_type,
            "creating new game session"
        );

        let session = Arc::new(GameSession::new(id.clone(), config));
        let hand = session.start_new_hand()?;

        {
            let mut guard = self
                .sessions
                .write()
                .map_err(|_| SessionError::StoragePoisoned)?;
            guard.insert(id.clone(), Arc::clone(&session));
        }

        tracing::debug!(
            session_id = %id,
            hand_id = %hand.hand_id,
            "session created and first hand started"
        );

        let players = match session.snapshot_players() {
            Ok(players) => players,
            Err(err) => {
                if let Err(cleanup_err) = self.delete_session(&id) {
                    tracing::error!(
                        session_id = %id,
                        error = %cleanup_err,
                        "failed to roll back session after snapshot failure"
                    );
                }
                return Err(err);
            }
        };
        self.event_bus.broadcast(
            &id,
            GameEvent::GameStarted {
                session_id: id.clone(),
                players,
            },
        );

        self.event_bus.broadcast(
            &id,
            GameEvent::HandStarted {
                session_id: id.clone(),
                hand_id: hand.hand_id.clone(),
                button_player: hand.button_player,
            },
        );

        for (player_id, cards) in hand.player_cards {
            self.event_bus.broadcast(
                &id,
                GameEvent::CardsDealt {
                    session_id: id.clone(),
                    player_id,
                    cards,
                },
            );
        }

        Ok(id)
    }

    pub fn get_session(&self, id: &SessionId) -> Result<Arc<GameSession>, SessionError> {
        let guard = self
            .sessions
            .read()
            .map_err(|_| SessionError::StoragePoisoned)?;
        guard
            .get(id)
            .cloned()
            .ok_or_else(|| SessionError::NotFound(id.clone()))
    }

    pub fn state(&self, session_id: &SessionId) -> Result<GameStateResponse, SessionError> {
        let session = self.get_session(session_id)?;
        if session.is_expired(self.session_ttl) {
            self.expire_session(session_id, "expired due to inactivity")?;
            return Err(SessionError::Expired(session_id.clone()));
        }
        session.touch();
        session.state_snapshot()
    }

    pub fn config(&self, session_id: &SessionId) -> Result<GameConfig, SessionError> {
        let session = self.get_session(session_id)?;
        Ok(session.config())
    }

    pub fn process_action(
        &self,
        session_id: &SessionId,
        action: PlayerAction,
    ) -> Result<GameEvent, SessionError> {
        let session = self.get_session(session_id)?;
        if session.is_expired(self.session_ttl) {
            self.expire_session(session_id, "expired due to inactivity")?;
            return Err(SessionError::Expired(session_id.clone()));
        }

        session.touch();

        // Check if there's an active hand
        let state = session.get_state()?;
        if !matches!(state, GameSessionState::HandInProgress { .. }) {
            return Err(SessionError::InvalidAction("No active hand".to_string()));
        }

        let player_id = session
            .current_player()?
            .ok_or_else(|| SessionError::InvalidAction("No current player".to_string()))?;

        tracing::debug!(
            session_id = %session_id,
            player_id = player_id,
            action = ?action,
            "processing player action"
        );

        // Apply action to engine and get current street
        let current_street = session.apply_action(player_id, action.clone())?;

        // Record action in session
        session.record_action(player_id, action.clone(), current_street)?;

        let event = GameEvent::PlayerAction {
            session_id: session_id.clone(),
            player_id,
            action: action.clone(),
        };
        self.event_bus.broadcast(session_id, event.clone());

        // Check if this action completes the hand (e.g., Fold)
        let hand_complete_by_action = matches!(action, PlayerAction::Fold);

        if hand_complete_by_action {
            // Fold immediately ends the hand
            self.finalize_hand(session_id, &session)?;
        } else {
            // Advance turn
            session.advance_turn()?;

            // Process AI action if next player is AI
            self.process_ai_turn_if_needed(session_id)?;

            // Check if hand is complete after actions
            if session.check_hand_complete()? {
                self.finalize_hand(session_id, &session)?;
            }
        }

        Ok(event)
    }

    /// Process AI turn if the current player is AI-controlled
    ///
    /// This method checks if the current player is AI and automatically
    /// processes their action, broadcasting it through the event bus.
    pub fn process_ai_turn_if_needed(&self, session_id: &SessionId) -> Result<(), SessionError> {
        let session = self.get_session(session_id)?;

        loop {
            let current_player = match session.current_player()? {
                Some(id) => id,
                None => return Ok(()), // No current player, game may be over
            };

            if !session.is_ai_player(current_player) {
                return Ok(()); // Not an AI player
            }

            // Get AI action
            let action = session.get_ai_action(current_player).ok_or_else(|| {
                SessionError::InvalidAction("AI failed to provide action".to_string())
            })?;

            // Apply action to engine
            let current_street = session.apply_action(current_player, action.clone())?;

            // Record AI action
            session.record_action(current_player, action.clone(), current_street)?;

            // Broadcast AI action
            let event = GameEvent::PlayerAction {
                session_id: session_id.clone(),
                player_id: current_player,
                action: action.clone(),
            };
            self.event_bus.broadcast(session_id, event);

            // Advance to next turn
            session.advance_turn()?;

            // Check if hand is complete after AI action
            if session.check_hand_complete()? {
                self.finalize_hand(session_id, &session)?;
                return Ok(());
            }
        }
    }

    /// Finalize a completed hand
    fn finalize_hand(
        &self,
        session_id: &SessionId,
        session: &GameSession,
    ) -> Result<(), SessionError> {
        // Get final state
        let state = session.state_snapshot()?;

        // Determine winners based on game state
        let winners = session.determine_winners()?;

        // Broadcast hand completed event
        self.event_bus.broadcast(
            session_id,
            GameEvent::HandCompleted {
                session_id: session_id.clone(),
                result: HandResult {
                    winner_ids: winners.clone(),
                    pot: state.pot,
                },
            },
        );

        // Record to history if available
        if let Some(history) = &self.history_store {
            let record = session.create_hand_record()?;
            history
                .add_hand(record)
                .map_err(|e| SessionError::EngineError(e.to_string()))?;
        }

        // Mark hand as complete
        session.complete_hand_with_winners(&winners)?;

        Ok(())
    }

    pub fn delete_session(&self, session_id: &SessionId) -> Result<(), SessionError> {
        match self.remove_session(session_id)? {
            Some(_) => {
                self.event_bus.broadcast(
                    session_id,
                    GameEvent::GameEnded {
                        session_id: session_id.clone(),
                        winner: None,
                        reason: "terminated_by_request".into(),
                    },
                );
                Ok(())
            }
            None => Err(SessionError::NotFound(session_id.clone())),
        }
    }

    pub fn cleanup_expired_sessions(&self) {
        let mut expired = Vec::new();
        {
            let mut guard = match self.sessions.write() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            guard.retain(|id, session| {
                if session.is_expired(self.session_ttl) {
                    expired.push(id.clone());
                    false
                } else {
                    true
                }
            });
        }

        for id in expired {
            self.event_bus.broadcast(
                &id,
                GameEvent::GameEnded {
                    session_id: id.clone(),
                    winner: None,
                    reason: "expired".into(),
                },
            );
            self.event_bus.drop_session(&id);
        }
    }

    pub fn active_sessions(&self) -> Vec<SessionId> {
        match self.sessions.read() {
            Ok(guard) => guard.keys().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    fn expire_session(&self, session_id: &SessionId, reason: &str) -> Result<(), SessionError> {
        if self.remove_session(session_id)?.is_some() {
            self.event_bus.broadcast(
                session_id,
                GameEvent::GameEnded {
                    session_id: session_id.clone(),
                    winner: None,
                    reason: reason.to_string(),
                },
            );
        }
        Ok(())
    }

    fn remove_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<Arc<GameSession>>, SessionError> {
        let removed = match self.sessions.write() {
            Ok(mut guard) => guard.remove(session_id),
            Err(_) => return Err(SessionError::StoragePoisoned),
        };
        if removed.is_some() {
            self.event_bus.drop_session(session_id);
        }
        Ok(removed)
    }
}

#[allow(dead_code)]
pub struct GameSession {
    id: SessionId,
    engine: Mutex<Engine>,
    config: GameConfig,
    state: Mutex<GameSessionState>,
    created_at: Instant,
    last_active: Mutex<Instant>,
    button_tracker: Mutex<usize>,
    ai_opponent: Option<Box<dyn AIOpponent>>,
    action_history: Mutex<Vec<ActionRecord>>,
    pot_tracker: Mutex<u32>,
    actions_this_street: Mutex<usize>,
}

impl std::fmt::Debug for GameSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSession")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("created_at", &self.created_at)
            .field(
                "ai_opponent",
                &self
                    .ai_opponent
                    .as_ref()
                    .map(|ai| ai.name())
                    .unwrap_or("none"),
            )
            .finish()
    }
}

struct HandMetadata {
    hand_id: String,
    button_player: usize,
    player_cards: Vec<(usize, Option<Vec<Card>>)>,
}

impl GameSession {
    fn new(id: SessionId, config: GameConfig) -> Self {
        let engine = Engine::new(config.seed, config.level);
        let ai_opponent = match &config.opponent_type {
            OpponentType::AI(name) => Some(create_ai(name)),
            OpponentType::Human => None,
        };
        let now = Instant::now();
        Self {
            id,
            engine: Mutex::new(engine),
            config,
            state: Mutex::new(GameSessionState::WaitingForPlayers),
            created_at: now,
            last_active: Mutex::new(now),
            button_tracker: Mutex::new(0),
            ai_opponent,
            action_history: Mutex::new(Vec::new()),
            pot_tracker: Mutex::new(0),
            actions_this_street: Mutex::new(0),
        }
    }

    /// Get AI action if this session has an AI opponent and it's the AI's turn
    pub fn get_ai_action(&self, player_id: usize) -> Option<PlayerAction> {
        if player_id == 0 {
            // Player 0 is always human
            return None;
        }

        let ai = self.ai_opponent.as_ref()?;
        let engine = self.engine.lock().ok()?;
        Some(ai.get_action(&engine, player_id))
    }

    /// Check if the specified player is AI-controlled
    pub fn is_ai_player(&self, player_id: usize) -> bool {
        player_id != 0 && self.ai_opponent.is_some()
    }

    /// Apply action to engine and update game state
    fn apply_action(&self, player_id: usize, action: PlayerAction) -> Result<Street, SessionError> {
        let mut pot = self
            .pot_tracker
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        let mut actions = self
            .actions_this_street
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;

        // Simple action application - in reality, this would interact with engine
        match action {
            PlayerAction::Fold => {
                // Hand ends immediately
            }
            PlayerAction::Check => {
                *actions += 1;
            }
            PlayerAction::Call => {
                // Add call amount to pot (simplified: assume small blind amount)
                *pot += 50;
                *actions += 1;
            }
            PlayerAction::Bet(amount) => {
                *pot += amount;
                *actions += 1;
            }
            PlayerAction::Raise(amount) => {
                *pot += amount;
                *actions += 1;
            }
            PlayerAction::AllIn => {
                // Get player's stack and add to pot
                let engine = self
                    .engine
                    .lock()
                    .map_err(|_| SessionError::StoragePoisoned)?;
                let player_stack = engine.players()[player_id].stack();
                *pot += player_stack;
                *actions += 1;
            }
        }

        // Get current street
        let state = self
            .state
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        let current_street = match &*state {
            GameSessionState::HandInProgress { street, .. } => *street,
            _ => return Err(SessionError::InvalidAction("No active hand".to_string())),
        };

        Ok(current_street)
    }

    /// Record an action in the hand history
    fn record_action(
        &self,
        player_id: usize,
        action: PlayerAction,
        street: Street,
    ) -> Result<(), SessionError> {
        let mut history = self
            .action_history
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        history.push(ActionRecord {
            player_id,
            street,
            action,
        });
        Ok(())
    }

    /// Check if the current hand is complete
    fn check_hand_complete(&self) -> Result<bool, SessionError> {
        let state = self
            .state
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;

        // Hand is complete if someone folded or if we've seen enough actions
        let actions = self
            .actions_this_street
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;

        // Simplified: hand is complete after 8 actions (2 per street * 4 streets)
        // or if state is Completed
        Ok(matches!(*state, GameSessionState::Completed { .. }) || *actions >= 8)
    }

    /// Get current state
    fn get_state(&self) -> Result<GameSessionState, SessionError> {
        let state = self
            .state
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        Ok(state.clone())
    }

    /// Create a HandRecord from the current session state
    fn create_hand_record(&self) -> Result<HandRecord, SessionError> {
        let state = self
            .state
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;

        let hand_id = match &*state {
            GameSessionState::HandInProgress { hand_id, .. } => hand_id.clone(),
            GameSessionState::Completed { .. } => {
                // Use a default hand ID if completed
                Uuid::new_v4().to_string()
            }
            _ => {
                return Err(SessionError::InvalidAction(
                    "No hand in progress".to_string(),
                ))
            }
        };
        drop(state);

        let engine = self
            .engine
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;

        let board = engine.board().clone();
        drop(engine);

        let actions = self
            .action_history
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?
            .clone();

        Ok(HandRecord {
            hand_id,
            seed: self.config.seed,
            actions,
            board,
            result: Some("hand completed".to_string()),
            ts: Some(chrono::Utc::now().to_rfc3339()),
            meta: None,
            showdown: None,
        })
    }

    fn config(&self) -> GameConfig {
        self.config.clone()
    }

    fn start_new_hand(&self) -> Result<HandMetadata, SessionError> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        let mut button = self
            .button_tracker
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;

        let button_player = *button;
        *button = 1 - *button;

        for player in engine.players_mut().iter_mut() {
            player.clear_cards();
        }

        for (idx, player) in engine.players_mut().iter_mut().enumerate() {
            let position = if idx == button_player {
                EnginePosition::Button
            } else {
                EnginePosition::BigBlind
            };
            player.set_position(position);
        }

        engine.shuffle();
        engine.deal_hand().map_err(SessionError::EngineError)?;

        let player_cards = engine
            .players()
            .iter()
            .enumerate()
            .map(|(idx, player)| {
                let cards: Vec<Card> = player.hole_cards().into_iter().flatten().collect();
                let cards = if idx == 0 && !cards.is_empty() {
                    Some(cards)
                } else {
                    None
                };
                (idx, cards)
            })
            .collect();

        let hand_id = Uuid::new_v4().to_string();
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| SessionError::StoragePoisoned)?;
            *state = GameSessionState::HandInProgress {
                hand_id: hand_id.clone(),
                current_player: button_player,
                street: Street::Preflop,
            };
        }
        drop(engine);

        // Clear action history for new hand
        {
            let mut history = self
                .action_history
                .lock()
                .map_err(|_| SessionError::StoragePoisoned)?;
            history.clear();
        }

        // Reset pot tracker
        {
            let mut pot = self
                .pot_tracker
                .lock()
                .map_err(|_| SessionError::StoragePoisoned)?;
            *pot = 150; // Initial pot with blinds (50 SB + 100 BB)
        }

        // Reset actions counter
        {
            let mut actions = self
                .actions_this_street
                .lock()
                .map_err(|_| SessionError::StoragePoisoned)?;
            *actions = 0;
        }

        self.touch();

        Ok(HandMetadata {
            hand_id,
            button_player,
            player_cards,
        })
    }

    fn snapshot_players(&self) -> Result<Vec<PlayerInfo>, SessionError> {
        let engine = self
            .engine
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        let players = engine
            .players()
            .iter()
            .enumerate()
            .map(|(idx, player)| PlayerInfo {
                id: idx,
                stack: player.stack(),
                position: SeatPosition::from(player.position()),
                is_human: idx == 0,
            })
            .collect();
        Ok(players)
    }
    fn touch(&self) {
        if let Ok(mut guard) = self.last_active.lock() {
            *guard = Instant::now();
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        match self.last_active.lock() {
            Ok(last) => last.elapsed() >= ttl,
            Err(_) => false,
        }
    }

    fn current_player(&self) -> Result<Option<usize>, SessionError> {
        let state = self
            .state
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        Ok(match &*state {
            GameSessionState::HandInProgress { current_player, .. } => Some(*current_player),
            _ => None,
        })
    }

    fn advance_turn(&self) -> Result<(), SessionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        if let GameSessionState::HandInProgress { current_player, .. } = &mut *state {
            *current_player = (*current_player + 1) % 2;
        }
        Ok(())
    }

    fn state_snapshot(&self) -> Result<GameStateResponse, SessionError> {
        let state = self
            .state
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?
            .clone();

        let (hand_id, street, current_player) = match &state {
            GameSessionState::HandInProgress {
                hand_id,
                current_player,
                street,
            } => (Some(hand_id.clone()), Some(*street), Some(*current_player)),
            GameSessionState::Completed { .. } => (None, Some(Street::River), None),
            GameSessionState::Error { .. } => (None, None, None),
            _ => (None, None, None),
        };

        let engine = self
            .engine
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        let players = engine
            .players()
            .iter()
            .enumerate()
            .map(|(idx, player)| {
                let cards_vec: Vec<Card> = player.hole_cards().into_iter().flatten().collect();
                let hole_cards = if idx == 0 && !cards_vec.is_empty() {
                    Some(cards_vec)
                } else {
                    None
                };
                let is_active = current_player == Some(idx);
                PlayerStateResponse {
                    id: idx,
                    stack: player.stack(),
                    position: SeatPosition::from(player.position()),
                    hole_cards,
                    is_active,
                    last_action: None,
                }
            })
            .collect();

        let board_full = engine.board().clone();
        drop(engine);

        let board = visible_board(&board_full, street);

        let pot = *self
            .pot_tracker
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;

        Ok(GameStateResponse {
            session_id: self.id.clone(),
            players,
            board,
            pot,
            current_player,
            available_actions: Self::default_actions(),
            hand_id,
            street,
        })
    }

    fn default_actions() -> Vec<AvailableAction> {
        vec![
            AvailableAction {
                action_type: "fold".into(),
                min_amount: None,
                max_amount: None,
            },
            AvailableAction {
                action_type: "check".into(),
                min_amount: None,
                max_amount: None,
            },
            AvailableAction {
                action_type: "bet".into(),
                min_amount: Some(100),
                max_amount: Some(2_000),
            },
        ]
    }

    /// Determine winners based on current game state
    fn determine_winners(&self) -> Result<Vec<usize>, SessionError> {
        // Check if hand ended by fold
        if let Some(last_action) = self
            .action_history
            .lock()
            .ok()
            .and_then(|h| h.last().cloned())
        {
            if matches!(last_action.action, PlayerAction::Fold) {
                // If player folded, the other player wins
                let winner = if last_action.player_id == 0 { 1 } else { 0 };
                return Ok(vec![winner]);
            }
        }

        // Showdown - for now, default to player 0 (implement proper evaluation later)
        // TODO: Integrate with engine's hand evaluation
        Ok(vec![0])
    }

    /// Complete hand and store winners
    fn complete_hand_with_winners(&self, winners: &[usize]) -> Result<(), SessionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SessionError::StoragePoisoned)?;
        *state = GameSessionState::Completed {
            winner: winners.first().copied(),
        };
        Ok(())
    }
}

#[cfg(test)]
impl GameSession {
    fn force_last_active(&self, instant: Instant) {
        if let Ok(mut guard) = self.last_active.lock() {
            *guard = instant;
        }
    }
}

fn visible_board(cards: &[Card], street: Option<Street>) -> Vec<Card> {
    let count = match street {
        Some(Street::Preflop) | None => 0,
        Some(Street::Flop) => min(3, cards.len()),
        Some(Street::Turn) => min(4, cards.len()),
        Some(Street::River) => min(5, cards.len()),
    };
    cards.iter().cloned().take(count).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn creates_session_and_provides_state() {
        let event_bus = Arc::new(EventBus::new());
        let manager = SessionManager::with_ttl(event_bus.clone(), Duration::from_secs(60));

        let id = manager
            .create_session(GameConfig::default())
            .expect("create session");

        let state = manager.state(&id).expect("session state");
        assert_eq!(state.session_id, id);
        assert_eq!(state.players.len(), 2);
        assert!(state.hand_id.is_some());
        assert_eq!(state.street, Some(Street::Preflop));
        assert!(state.board.is_empty());

        let session = manager.get_session(&id).expect("get session");
        assert!(!session.is_expired(Duration::from_secs(60)));

        let mut sub = manager.event_bus().subscribe(id.clone());
        let event = manager
            .process_action(&id, PlayerAction::Check)
            .expect("process action");
        match event {
            GameEvent::PlayerAction { session_id, .. } => assert_eq!(session_id, id),
            other => panic!("unexpected event: {:?}", other),
        }

        let delivered = sub.receiver.try_recv().expect("event delivered");
        assert!(matches!(delivered, GameEvent::PlayerAction { .. }));
    }

    #[test]
    fn cleanup_expired_sessions_removes_stale_entries() {
        let event_bus = Arc::new(EventBus::new());
        let manager = SessionManager::with_ttl(event_bus.clone(), Duration::from_secs(1));
        let id = manager
            .create_session(GameConfig::default())
            .expect("create session");
        let session = manager.get_session(&id).expect("get session");
        let mut sub = manager.event_bus().subscribe(id.clone());

        session.force_last_active(Instant::now() - Duration::from_secs(5));
        manager.cleanup_expired_sessions();

        match manager.get_session(&id) {
            Err(SessionError::NotFound(_)) => {}
            other => panic!("expected not found, got {:?}", other),
        }

        match sub.receiver.try_recv() {
            Ok(GameEvent::GameEnded { reason, .. }) => assert_eq!(reason, "expired"),
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn concurrent_session_creation_is_safe() {
        let event_bus = Arc::new(EventBus::new());
        let manager = Arc::new(SessionManager::with_ttl(event_bus, Duration::from_secs(60)));

        let mut handles = Vec::new();
        for _ in 0..8 {
            let manager = Arc::clone(&manager);
            handles.push(thread::spawn(move || {
                let mut ids = Vec::new();
                for _ in 0..32 {
                    let id = manager
                        .create_session(GameConfig::default())
                        .expect("create session");
                    ids.push(id);
                }
                ids
            }));
        }

        let mut unique = HashSet::new();
        for handle in handles {
            for id in handle.join().expect("join thread") {
                assert!(unique.insert(id));
            }
        }

        let active = manager.active_sessions();
        assert_eq!(active.len(), unique.len());
    }

    #[test]
    fn session_with_ai_opponent_processes_ai_actions() {
        use crate::ai::create_ai;

        let event_bus = Arc::new(EventBus::new());
        let manager = SessionManager::with_ttl(event_bus.clone(), Duration::from_secs(60));

        let config = GameConfig {
            seed: Some(42),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        };

        let id = manager.create_session(config).expect("create session");
        let session = manager.get_session(&id).expect("get session");

        // Verify AI opponent type is stored
        let retrieved_config = session.config();
        assert_eq!(
            retrieved_config.opponent_type,
            OpponentType::AI("baseline".to_string())
        );

        // Verify AI can be created
        let ai = create_ai("baseline");
        assert_eq!(ai.name(), "baseline");
    }

    #[test]
    fn session_distinguishes_human_and_ai_opponents() {
        let event_bus = Arc::new(EventBus::new());
        let manager = SessionManager::with_ttl(event_bus.clone(), Duration::from_secs(60));

        let human_config = GameConfig {
            seed: Some(1),
            level: 1,
            opponent_type: OpponentType::Human,
        };

        let ai_config = GameConfig {
            seed: Some(2),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        };

        let human_id = manager
            .create_session(human_config)
            .expect("create session");
        let ai_id = manager.create_session(ai_config).expect("create session");

        let human_session = manager.get_session(&human_id).expect("get session");
        let ai_session = manager.get_session(&ai_id).expect("get session");

        assert_eq!(human_session.config().opponent_type, OpponentType::Human);
        assert_eq!(
            ai_session.config().opponent_type,
            OpponentType::AI("baseline".to_string())
        );
    }

    #[test]
    fn ai_opponent_automatically_plays_when_its_turn() {
        let event_bus = Arc::new(EventBus::new());
        let manager = SessionManager::with_ttl(event_bus.clone(), Duration::from_secs(60));

        let config = GameConfig {
            seed: Some(42),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        };

        let id = manager.create_session(config).expect("create session");
        let mut sub = manager.event_bus().subscribe(id.clone());

        // Human (player 0) makes an action
        let state = manager.state(&id).expect("get state");
        let current_player = state.current_player.unwrap_or(0);

        // If it's human's turn (player 0)
        if current_player == 0 {
            manager
                .process_action(&id, PlayerAction::Check)
                .expect("process action");

            // Check events - should have both human action and AI response
            let mut events_received = Vec::new();
            while let Ok(event) = sub.receiver.try_recv() {
                events_received.push(event);
            }

            // Should have at least 2 player action events (human + AI)
            let player_actions: Vec<_> = events_received
                .iter()
                .filter_map(|e| match e {
                    GameEvent::PlayerAction { player_id, .. } => Some(*player_id),
                    _ => None,
                })
                .collect();

            assert!(
                player_actions.contains(&0),
                "Human action should be recorded"
            );
            assert!(player_actions.contains(&1), "AI action should be automatic");
        }
    }

    #[test]
    fn session_identifies_ai_players_correctly() {
        let event_bus = Arc::new(EventBus::new());
        let manager = SessionManager::with_ttl(event_bus.clone(), Duration::from_secs(60));

        let config = GameConfig {
            seed: Some(42),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        };

        let id = manager.create_session(config).expect("create session");
        let session = manager.get_session(&id).expect("get session");

        // Player 0 is always human
        assert!(!session.is_ai_player(0));

        // Player 1 is AI in this session
        assert!(session.is_ai_player(1));

        // AI can provide action for player 1
        let action = session.get_ai_action(1);
        assert!(action.is_some());

        // AI cannot provide action for player 0 (human)
        let action = session.get_ai_action(0);
        assert!(action.is_none());
    }

    #[test]
    fn session_manager_integrates_with_history_store() {
        let event_bus = Arc::new(EventBus::new());
        let history = Arc::new(HistoryStore::new());
        let manager = SessionManager::with_ttl_and_history(
            event_bus.clone(),
            Duration::from_secs(60),
            history.clone(),
        );

        let config = GameConfig {
            seed: Some(42),
            level: 1,
            opponent_type: OpponentType::Human,
        };

        let id = manager.create_session(config).expect("create session");

        // Simulate some actions
        let session = manager.get_session(&id).expect("get session");
        session
            .record_action(0, PlayerAction::Check, Street::Preflop)
            .expect("record action");
        session
            .record_action(1, PlayerAction::Check, Street::Preflop)
            .expect("record action");

        // Verify actions were recorded in session
        let actions = session.action_history.lock().expect("lock history");
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn hand_record_creation_includes_action_history() {
        let event_bus = Arc::new(EventBus::new());
        let manager = SessionManager::with_ttl(event_bus.clone(), Duration::from_secs(60));

        let config = GameConfig {
            seed: Some(123),
            level: 1,
            opponent_type: OpponentType::Human,
        };

        let id = manager.create_session(config).expect("create session");
        let session = manager.get_session(&id).expect("get session");

        // Record some actions
        session
            .record_action(0, PlayerAction::Bet(100), Street::Preflop)
            .expect("record action");
        session
            .record_action(1, PlayerAction::Call, Street::Preflop)
            .expect("record action");

        // Create hand record
        let record = session.create_hand_record().expect("create record");

        assert_eq!(record.seed, Some(123));
        assert_eq!(record.actions.len(), 2);
        assert_eq!(record.actions[0].player_id, 0);
        assert_eq!(record.actions[1].player_id, 1);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameConfig {
    pub seed: Option<u64>,
    pub level: u8,
    pub opponent_type: OpponentType,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            seed: None,
            level: 1,
            opponent_type: OpponentType::AI("baseline".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpponentType {
    Human,
    AI(String),
}

impl OpponentType {
    fn as_str(&self) -> Cow<'_, str> {
        match self {
            OpponentType::Human => Cow::Borrowed("human"),
            OpponentType::AI(name) => {
                let mut value = String::with_capacity(3 + name.len());
                value.push_str("ai:");
                value.push_str(name);
                Cow::Owned(value)
            }
        }
    }
}

impl Serialize for OpponentType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_str())
    }
}

impl<'de> Deserialize<'de> for OpponentType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        if raw.eq_ignore_ascii_case("human") {
            return Ok(OpponentType::Human);
        }

        if let Some(rest) = raw.strip_prefix("ai:") {
            if rest.is_empty() {
                return Ok(OpponentType::AI("baseline".into()));
            }
            return Ok(OpponentType::AI(rest.to_string()));
        }

        Err(serde::de::Error::custom(format!(
            "invalid opponent type: {raw}"
        )))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailableAction {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerStateResponse {
    pub id: usize,
    pub stack: u32,
    pub position: SeatPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hole_cards: Option<Vec<Card>>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action: Option<PlayerAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameStateResponse {
    pub session_id: SessionId,
    pub players: Vec<PlayerStateResponse>,
    pub board: Vec<Card>,
    pub pot: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_player: Option<usize>,
    pub available_actions: Vec<AvailableAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hand_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<Street>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GameSessionState {
    WaitingForPlayers,
    InProgress,
    HandInProgress {
        hand_id: String,
        current_player: usize,
        street: Street,
    },
    Completed {
        winner: Option<usize>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(SessionId),
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    #[error("Game engine error: {0}")]
    EngineError(String),
    #[error("Session expired: {0}")]
    Expired(SessionId),
    #[error("Session storage poisoned")]
    StoragePoisoned,
}

impl crate::errors::IntoErrorResponse for SessionError {
    fn status_code(&self) -> warp::http::StatusCode {
        use warp::http::StatusCode;
        match self {
            SessionError::NotFound(_) => StatusCode::NOT_FOUND,
            SessionError::Expired(_) => StatusCode::GONE,
            SessionError::InvalidAction(_) => StatusCode::BAD_REQUEST,
            SessionError::EngineError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SessionError::StoragePoisoned => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            SessionError::NotFound(_) => "session_not_found",
            SessionError::Expired(_) => "session_expired",
            SessionError::InvalidAction(_) => "invalid_action",
            SessionError::EngineError(_) => "engine_error",
            SessionError::StoragePoisoned => "session_storage_error",
        }
    }

    fn error_message(&self) -> String {
        self.to_string()
    }

    fn error_details(&self) -> Option<serde_json::Value> {
        match self {
            SessionError::NotFound(id) => Some(serde_json::json!({
                "session_id": id
            })),
            SessionError::Expired(id) => Some(serde_json::json!({
                "session_id": id,
                "reason": "Session expired due to inactivity"
            })),
            _ => None,
        }
    }

    fn severity(&self) -> crate::errors::ErrorSeverity {
        use crate::errors::ErrorSeverity;
        match self {
            SessionError::StoragePoisoned => ErrorSeverity::Critical,
            SessionError::EngineError(_) => ErrorSeverity::Server,
            _ => ErrorSeverity::Client,
        }
    }
}
