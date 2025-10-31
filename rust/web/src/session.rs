use crate::events::{EventBus, GameEvent, PlayerInfo};
use axm_engine::cards::Card;
use axm_engine::engine::Engine;
use axm_engine::logger::Street;
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
    session_ttl: Duration,
}

impl SessionManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_bus,
            session_ttl: DEFAULT_SESSION_TTL,
        }
    }

    pub fn with_ttl(event_bus: Arc<EventBus>, ttl: Duration) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_bus,
            session_ttl: ttl,
        }
    }

    pub fn create_session(&self, config: GameConfig) -> Result<SessionId, SessionError> {
        let id = Uuid::new_v4().to_string();
        let session = Arc::new(GameSession::new(id.clone(), config));
        let hand = session.start_new_hand()?;

        {
            let mut guard = self
                .sessions
                .write()
                .map_err(|_| SessionError::StoragePoisoned)?;
            guard.insert(id.clone(), Arc::clone(&session));
        }

        let players = session.snapshot_players();
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
        let player_id = session.current_player()?.unwrap_or(0);
        let event = GameEvent::PlayerAction {
            session_id: session_id.clone(),
            player_id,
            action: action.clone(),
        };
        self.event_bus.broadcast(session_id, event.clone());
        session.advance_turn()?;
        Ok(event)
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

#[derive(Debug)]
#[allow(dead_code)]
pub struct GameSession {
    id: SessionId,
    engine: Mutex<Engine>,
    config: GameConfig,
    state: Mutex<GameSessionState>,
    created_at: Instant,
    last_active: Mutex<Instant>,
    button_tracker: Mutex<usize>,
}

struct HandMetadata {
    hand_id: String,
    button_player: usize,
    player_cards: Vec<(usize, Option<Vec<Card>>)>,
}

impl GameSession {
    fn new(id: SessionId, config: GameConfig) -> Self {
        let engine = Engine::new(config.seed, config.level);
        let now = Instant::now();
        Self {
            id,
            engine: Mutex::new(engine),
            config,
            state: Mutex::new(GameSessionState::WaitingForPlayers),
            created_at: now,
            last_active: Mutex::new(now),
            button_tracker: Mutex::new(0),
        }
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

        self.touch();

        Ok(HandMetadata {
            hand_id,
            button_player,
            player_cards,
        })
    }

    fn snapshot_players(&self) -> Vec<PlayerInfo> {
        let engine = self.engine.lock().expect("engine lock poisoned");
        engine
            .players()
            .iter()
            .enumerate()
            .map(|(idx, player)| PlayerInfo {
                id: idx,
                stack: player.stack(),
                position: SeatPosition::from(player.position()),
                is_human: idx == 0,
            })
            .collect()
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

        Ok(GameStateResponse {
            session_id: self.id.clone(),
            players,
            board,
            pot: 0,
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
