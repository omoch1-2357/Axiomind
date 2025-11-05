use crate::session::{SeatPosition, SessionId};
use axm_engine::cards::Card;
use axm_engine::logger::Street;
use axm_engine::player::PlayerAction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

// Use bounded channel with reasonable buffer size to prevent memory exhaustion
// If all subscribers are slow, events will be dropped (backpressure)
const EVENT_CHANNEL_BUFFER: usize = 1000;

pub type EventSender = mpsc::Sender<GameEvent>;
pub type EventReceiver = mpsc::Receiver<GameEvent>;

pub struct EventSubscription {
    bus: EventBus,
    session_id: SessionId,
    subscriber_id: usize,
    pub receiver: EventReceiver,
}

impl EventSubscription {
    pub fn receiver(&mut self) -> &mut EventReceiver {
        &mut self.receiver
    }
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        self.bus.unsubscribe(&self.session_id, self.subscriber_id);
    }
}

#[derive(Debug, Clone, Default)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

#[derive(Debug, Default)]
struct EventBusInner {
    subscribers: RwLock<HashMap<SessionId, Vec<(usize, EventSender)>>>,
    next_id: AtomicUsize,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, session_id: SessionId) -> EventSubscription {
        let (subscriber_id, receiver) = self.subscribe_raw(session_id.clone());
        EventSubscription {
            bus: self.clone(),
            session_id,
            subscriber_id,
            receiver,
        }
    }

    fn subscribe_raw(&self, session_id: SessionId) -> (usize, EventReceiver) {
        let (tx, rx) = mpsc::channel(EVENT_CHANNEL_BUFFER);
        let id = self.inner.next_id.fetch_add(1, Ordering::AcqRel);
        let mut guard = self
            .inner
            .subscribers
            .write()
            .expect("subscriber lock poisoned");
        guard.entry(session_id.clone()).or_default().push((id, tx));

        tracing::info!(
            session_id = %session_id,
            subscriber_id = id,
            "client subscribed to game events"
        );

        (id, rx)
    }

    pub fn broadcast(&self, session_id: &SessionId, event: GameEvent) {
        // Log game event for debugging and analysis
        tracing::debug!(
            session_id = %session_id,
            event_type = ?event,
            "broadcasting game event"
        );

        let subscribers = {
            let guard = self
                .inner
                .subscribers
                .read()
                .expect("subscriber lock poisoned");
            guard.get(session_id).cloned()
        };

        if let Some(list) = subscribers {
            let subscriber_count = list.len();
            tracing::trace!(
                session_id = %session_id,
                subscriber_count = subscriber_count,
                "sending event to subscribers"
            );

            let mut failed = Vec::new();
            for (id, sender) in list {
                // Use try_send to avoid blocking on full channels
                // This implements backpressure by dropping events for slow subscribers
                if let Err(e) = sender.try_send(event.clone()) {
                    tracing::warn!(
                        session_id = %session_id,
                        subscriber_id = id,
                        error = ?e,
                        "failed to send event to subscriber"
                    );
                    failed.push(id);
                }
            }
            if !failed.is_empty() {
                self.remove_subscribers(session_id, &failed);
            }
        } else {
            tracing::debug!(
                session_id = %session_id,
                "no subscribers for session"
            );
        }
    }

    pub fn unsubscribe(&self, session_id: &SessionId, subscriber_id: usize) {
        self.remove_subscribers(session_id, &[subscriber_id]);
    }

    pub fn drop_session(&self, session_id: &SessionId) {
        let mut guard = self
            .inner
            .subscribers
            .write()
            .expect("subscriber lock poisoned");
        guard.remove(session_id);
    }

    pub fn subscriber_count(&self) -> usize {
        let guard = self
            .inner
            .subscribers
            .read()
            .expect("subscriber lock poisoned");
        guard.values().map(|list| list.len()).sum()
    }

    fn remove_subscribers(&self, session_id: &SessionId, ids: &[usize]) {
        let mut guard = self
            .inner
            .subscribers
            .write()
            .expect("subscriber lock poisoned");
        if let Some(list) = guard.get_mut(session_id) {
            list.retain(|(id, _)| !ids.contains(id));
            if list.is_empty() {
                guard.remove(session_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_drop_unsubscribes() {
        let bus = EventBus::new();
        let session = "s".to_string();
        {
            let _sub = bus.subscribe(session.clone());
            assert_eq!(bus.subscriber_count(), 1);
        }
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn broadcast_reaches_all_subscribers() {
        let bus = EventBus::new();
        let session = "s".to_string();
        let mut sub1 = bus.subscribe(session.clone());
        let mut sub2 = bus.subscribe(session.clone());

        bus.broadcast(
            &session,
            GameEvent::Error {
                session_id: session.clone(),
                message: "ping".into(),
            },
        );

        let ev1 = sub1.receiver.try_recv().expect("sub1 event");
        let ev2 = sub2.receiver.try_recv().expect("sub2 event");
        assert!(matches!(ev1, GameEvent::Error { .. }));
        assert!(matches!(ev2, GameEvent::Error { .. }));
    }

    #[test]
    fn stale_receiver_is_pruned() {
        let bus = EventBus::new();
        let session = "s".to_string();
        let (id, rx) = bus.subscribe_raw(session.clone());
        drop(rx);
        bus.broadcast(
            &session,
            GameEvent::Error {
                session_id: session.clone(),
                message: "gone".into(),
            },
        );
        assert_eq!(bus.subscriber_count(), 0);
        bus.unsubscribe(&session, id); // ensure no panic when unsub after removal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    GameStarted {
        session_id: SessionId,
        players: Vec<PlayerInfo>,
    },
    HandStarted {
        session_id: SessionId,
        hand_id: String,
        button_player: usize,
    },
    CardsDealt {
        session_id: SessionId,
        player_id: usize,
        cards: Option<Vec<Card>>,
    },
    CommunityCards {
        session_id: SessionId,
        cards: Vec<Card>,
        street: Street,
    },
    PlayerAction {
        session_id: SessionId,
        player_id: usize,
        action: PlayerAction,
    },
    HandCompleted {
        session_id: SessionId,
        result: HandResult,
    },
    GameEnded {
        session_id: SessionId,
        winner: Option<usize>,
        reason: String,
    },
    Error {
        session_id: SessionId,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: usize,
    pub stack: u32,
    pub position: SeatPosition,
    pub is_human: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandResult {
    pub winner_ids: Vec<usize>,
    pub pot: u32,
}
