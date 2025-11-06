use axm_engine::logger::HandRecord;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use thiserror::Error;

/// Hand history storage and retrieval system
#[derive(Debug)]
pub struct HistoryStore {
    hands: RwLock<Vec<HandRecord>>,
}

impl HistoryStore {
    pub fn new() -> Self {
        Self {
            hands: RwLock::new(Vec::new()),
        }
    }

    /// Add a hand record to the history
    pub fn add_hand(&self, record: HandRecord) -> Result<(), HistoryError> {
        let mut hands = self
            .hands
            .write()
            .map_err(|_| HistoryError::StoragePoisoned)?;
        hands.push(record);
        Ok(())
    }

    /// Get recent hands with optional limit
    pub fn get_recent_hands(&self, limit: Option<usize>) -> Result<Vec<HandRecord>, HistoryError> {
        let hands = self
            .hands
            .read()
            .map_err(|_| HistoryError::StoragePoisoned)?;
        let limit = limit.unwrap_or(100);
        Ok(hands.iter().rev().take(limit).cloned().collect())
    }

    /// Get a specific hand by ID
    pub fn get_hand(&self, hand_id: &str) -> Result<Option<HandRecord>, HistoryError> {
        let hands = self
            .hands
            .read()
            .map_err(|_| HistoryError::StoragePoisoned)?;
        Ok(hands.iter().find(|h| h.hand_id == hand_id).cloned())
    }

    /// Filter hands by criteria
    pub fn filter_hands(&self, filter: HandFilter) -> Result<Vec<HandRecord>, HistoryError> {
        let hands = self
            .hands
            .read()
            .map_err(|_| HistoryError::StoragePoisoned)?;
        let filtered: Vec<HandRecord> = hands
            .iter()
            .filter(|h| filter.matches(h))
            .cloned()
            .collect();
        Ok(filtered)
    }

    /// Calculate statistics from stored hands
    pub fn calculate_stats(&self) -> Result<HandStatistics, HistoryError> {
        let hands = self
            .hands
            .read()
            .map_err(|_| HistoryError::StoragePoisoned)?;

        if hands.is_empty() {
            return Ok(HandStatistics::default());
        }

        let total_hands = hands.len();
        let mut wins = 0;
        let mut total_pot = 0;

        for hand in hands.iter() {
            // Count wins (simplified - check if result indicates player 0 wins)
            if let Some(result) = &hand.result {
                if result.contains("player 0 wins") {
                    wins += 1;
                }
            }

            // Estimate pot size from actions (simplified)
            total_pot += estimate_pot_size(hand);
        }

        let win_rate = (wins as f64 / total_hands as f64) * 100.0;
        let avg_pot = total_pot as f64 / total_hands as f64;

        Ok(HandStatistics {
            total_hands,
            wins,
            win_rate,
            avg_pot_size: avg_pot,
        })
    }

    /// Get the total number of hands
    pub fn total_hands(&self) -> Result<usize, HistoryError> {
        let hands = self
            .hands
            .read()
            .map_err(|_| HistoryError::StoragePoisoned)?;
        Ok(hands.len())
    }
}

impl Default for HistoryStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter criteria for hand history
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>, // "win", "loss", "tie"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,
}

impl HandFilter {
    fn matches(&self, hand: &HandRecord) -> bool {
        // Filter by result type
        if let Some(result_type) = &self.result_type {
            if let Some(result) = &hand.result {
                if !result.to_lowercase().contains(&result_type.to_lowercase()) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Filter by date range
        if let (Some(ts), Some(date_from)) = (&hand.ts, &self.date_from) {
            if ts < date_from {
                return false;
            }
        }

        if let (Some(ts), Some(date_to)) = (&hand.ts, &self.date_to) {
            if ts > date_to {
                return false;
            }
        }

        true
    }
}

/// Statistics calculated from hand history
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandStatistics {
    pub total_hands: usize,
    pub wins: usize,
    pub win_rate: f64,
    pub avg_pot_size: f64,
}

impl Default for HandStatistics {
    fn default() -> Self {
        Self {
            total_hands: 0,
            wins: 0,
            win_rate: 0.0,
            avg_pot_size: 0.0,
        }
    }
}

/// Estimate pot size from hand record actions (simplified implementation)
fn estimate_pot_size(hand: &HandRecord) -> u32 {
    // For now, return a simple estimate based on number of actions
    // In real implementation, this would parse bet/raise amounts from actions
    (hand.actions.len() as u32) * 50 // Simplified estimation
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("History storage poisoned")]
    StoragePoisoned,
    #[error("Hand not found: {0}")]
    NotFound(String),
}

impl crate::errors::IntoErrorResponse for HistoryError {
    fn status_code(&self) -> warp::http::StatusCode {
        use warp::http::StatusCode;
        match self {
            HistoryError::StoragePoisoned => StatusCode::INTERNAL_SERVER_ERROR,
            HistoryError::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            HistoryError::StoragePoisoned => "history_storage_error",
            HistoryError::NotFound(_) => "hand_not_found",
        }
    }

    fn error_message(&self) -> String {
        self.to_string()
    }

    fn severity(&self) -> crate::errors::ErrorSeverity {
        use crate::errors::ErrorSeverity;
        match self {
            HistoryError::StoragePoisoned => ErrorSeverity::Critical,
            HistoryError::NotFound(_) => ErrorSeverity::Client,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axm_engine::cards::{Card, Rank, Suit};
    use axm_engine::logger::{ActionRecord, Street};
    use axm_engine::player::PlayerAction;

    fn create_test_hand(hand_id: &str, result: Option<&str>, ts: Option<&str>) -> HandRecord {
        HandRecord {
            hand_id: hand_id.to_string(),
            seed: Some(42),
            actions: vec![
                ActionRecord {
                    player_id: 0,
                    street: Street::Preflop,
                    action: PlayerAction::Bet(100),
                },
                ActionRecord {
                    player_id: 1,
                    street: Street::Preflop,
                    action: PlayerAction::Call,
                },
            ],
            board: vec![
                Card {
                    rank: Rank::Ace,
                    suit: Suit::Spades,
                },
                Card {
                    rank: Rank::King,
                    suit: Suit::Hearts,
                },
                Card {
                    rank: Rank::Queen,
                    suit: Suit::Diamonds,
                },
            ],
            result: result.map(String::from),
            ts: ts.map(String::from),
            meta: None,
            showdown: None,
        }
    }

    #[test]
    fn test_add_and_retrieve_hand() {
        let store = HistoryStore::new();
        let hand = create_test_hand(
            "test-001",
            Some("player 0 wins"),
            Some("2025-01-01T12:00:00Z"),
        );

        store.add_hand(hand.clone()).expect("add hand");

        let retrieved = store.get_hand("test-001").expect("get hand");
        assert_eq!(retrieved, Some(hand));
    }

    #[test]
    fn test_get_recent_hands_with_limit() {
        let store = HistoryStore::new();

        for i in 0..10 {
            let hand = create_test_hand(&format!("test-{:03}", i), Some("player 0 wins"), None);
            store.add_hand(hand).expect("add hand");
        }

        let recent = store.get_recent_hands(Some(5)).expect("get recent");
        assert_eq!(recent.len(), 5);

        // Should be in reverse order (most recent first)
        assert_eq!(recent[0].hand_id, "test-009");
        assert_eq!(recent[4].hand_id, "test-005");
    }

    #[test]
    fn test_filter_hands_by_result_type() {
        let store = HistoryStore::new();

        store
            .add_hand(create_test_hand("win-1", Some("player 0 wins"), None))
            .expect("add hand");
        store
            .add_hand(create_test_hand("loss-1", Some("player 1 wins"), None))
            .expect("add hand");
        store
            .add_hand(create_test_hand("win-2", Some("player 0 wins"), None))
            .expect("add hand");

        let filter = HandFilter {
            result_type: Some("player 0 wins".to_string()),
            ..Default::default()
        };

        let filtered = store.filter_hands(filter).expect("filter hands");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].hand_id, "win-1");
        assert_eq!(filtered[1].hand_id, "win-2");
    }

    #[test]
    fn test_filter_hands_by_date_range() {
        let store = HistoryStore::new();

        store
            .add_hand(create_test_hand(
                "old",
                Some("win"),
                Some("2025-01-01T10:00:00Z"),
            ))
            .expect("add hand");
        store
            .add_hand(create_test_hand(
                "mid",
                Some("win"),
                Some("2025-01-02T10:00:00Z"),
            ))
            .expect("add hand");
        store
            .add_hand(create_test_hand(
                "new",
                Some("win"),
                Some("2025-01-03T10:00:00Z"),
            ))
            .expect("add hand");

        let filter = HandFilter {
            date_from: Some("2025-01-02T00:00:00Z".to_string()),
            date_to: Some("2025-01-03T00:00:00Z".to_string()),
            ..Default::default()
        };

        let filtered = store.filter_hands(filter).expect("filter hands");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].hand_id, "mid");
    }

    #[test]
    fn test_calculate_statistics_empty_store() {
        let store = HistoryStore::new();

        let stats = store.calculate_stats().expect("calculate stats");
        assert_eq!(stats.total_hands, 0);
        assert_eq!(stats.wins, 0);
        assert_eq!(stats.win_rate, 0.0);
        assert_eq!(stats.avg_pot_size, 0.0);
    }

    #[test]
    fn test_calculate_statistics_with_hands() {
        let store = HistoryStore::new();

        // Add 3 wins and 2 losses
        for i in 0..3 {
            store
                .add_hand(create_test_hand(
                    &format!("win-{}", i),
                    Some("player 0 wins"),
                    None,
                ))
                .expect("add hand");
        }
        for i in 0..2 {
            store
                .add_hand(create_test_hand(
                    &format!("loss-{}", i),
                    Some("player 1 wins"),
                    None,
                ))
                .expect("add hand");
        }

        let stats = store.calculate_stats().expect("calculate stats");
        assert_eq!(stats.total_hands, 5);
        assert_eq!(stats.wins, 3);
        assert_eq!(stats.win_rate, 60.0);
        assert!(stats.avg_pot_size > 0.0);
    }

    #[test]
    fn test_get_nonexistent_hand_returns_none() {
        let store = HistoryStore::new();

        let result = store.get_hand("nonexistent").expect("get hand");
        assert_eq!(result, None);
    }

    #[test]
    fn test_total_hands_count() {
        let store = HistoryStore::new();

        assert_eq!(store.total_hands().expect("count"), 0);

        store
            .add_hand(create_test_hand("test-1", None, None))
            .expect("add hand");
        assert_eq!(store.total_hands().expect("count"), 1);

        store
            .add_hand(create_test_hand("test-2", None, None))
            .expect("add hand");
        assert_eq!(store.total_hands().expect("count"), 2);
    }
}
