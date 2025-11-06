use axm_engine::cards::{Card, Rank, Suit};
use axm_engine::logger::{ActionRecord, HandRecord, Street};
use axm_engine::player::PlayerAction;
use axm_web::HistoryStore;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

/// Create a test hand record
fn create_test_hand(hand_id: &str) -> HandRecord {
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
        result: Some("player 0 wins".to_string()),
        ts: Some("2025-01-01T12:00:00Z".to_string()),
        meta: None,
        showdown: None,
    }
}

#[test]
fn test_large_history_performance() {
    let store = HistoryStore::new();

    // Add 1000 hands
    let start = Instant::now();
    for i in 0..1000 {
        let hand = create_test_hand(&format!("hand-{:04}", i));
        store.add_hand(hand).expect("add hand");
    }
    let add_duration = start.elapsed();

    println!("Added 1000 hands in {:?}", add_duration);

    // Test retrieval performance
    let start = Instant::now();
    let recent = store.get_recent_hands(Some(100)).expect("get recent");
    let get_duration = start.elapsed();

    assert_eq!(recent.len(), 100);
    println!("Retrieved 100 recent hands in {:?}", get_duration);

    // Retrieval should be fast (less than 10ms for 100 hands)
    assert!(
        get_duration.as_millis() < 10,
        "Retrieval too slow: {:?}",
        get_duration
    );
}

#[test]
fn test_concurrent_read_access() {
    let store = Arc::new(HistoryStore::new());

    // Add some test data
    for i in 0..100 {
        let hand = create_test_hand(&format!("hand-{:04}", i));
        store.add_hand(hand).expect("add hand");
    }

    // Spawn multiple reader threads
    let mut handles = Vec::new();
    for _ in 0..10 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            for _ in 0..50 {
                let _ = store_clone.get_recent_hands(Some(10));
            }
        }));
    }

    // All threads should complete without panicking
    for handle in handles {
        handle.join().expect("thread should not panic");
    }
}

#[test]
fn test_concurrent_write_and_read() {
    let store = Arc::new(HistoryStore::new());

    // Add initial data
    for i in 0..50 {
        let hand = create_test_hand(&format!("initial-{:04}", i));
        store.add_hand(hand).expect("add hand");
    }

    let mut handles = Vec::new();

    // Spawn writer threads
    for thread_id in 0..3 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            for i in 0..20 {
                let hand = create_test_hand(&format!("thread-{}-hand-{:04}", thread_id, i));
                store_clone.add_hand(hand).expect("add hand");
            }
        }));
    }

    // Spawn reader threads
    for _ in 0..5 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            for _ in 0..30 {
                let _ = store_clone.get_recent_hands(Some(20));
            }
        }));
    }

    // All threads should complete successfully
    for handle in handles {
        handle.join().expect("thread should not panic");
    }

    // Verify final count
    let total = store.total_hands().expect("get total");
    assert_eq!(total, 50 + (3 * 20), "Should have all hands added");
}

#[test]
fn test_filter_performance() {
    let store = HistoryStore::new();

    // Add diverse hands
    for i in 0..500 {
        let mut hand = create_test_hand(&format!("hand-{:04}", i));
        if i % 2 == 0 {
            hand.result = Some("player 0 wins".to_string());
        } else {
            hand.result = Some("player 1 wins".to_string());
        }
        store.add_hand(hand).expect("add hand");
    }

    // Test filter performance
    let start = Instant::now();
    let filter = axm_web::HandFilter {
        result_type: Some("player 0 wins".to_string()),
        date_from: None,
        date_to: None,
    };
    let filtered = store.filter_hands(filter).expect("filter hands");
    let filter_duration = start.elapsed();

    assert_eq!(filtered.len(), 250);
    println!("Filtered 500 hands in {:?}", filter_duration);

    // Filter should be reasonably fast (less than 20ms for 500 hands)
    assert!(
        filter_duration.as_millis() < 20,
        "Filter too slow: {:?}",
        filter_duration
    );
}

#[test]
fn test_statistics_calculation_performance() {
    let store = HistoryStore::new();

    // Add hands
    for i in 0..1000 {
        let mut hand = create_test_hand(&format!("hand-{:04}", i));
        if i % 3 == 0 {
            hand.result = Some("player 0 wins".to_string());
        } else {
            hand.result = Some("player 1 wins".to_string());
        }
        store.add_hand(hand).expect("add hand");
    }

    // Test statistics calculation performance
    let start = Instant::now();
    let stats = store.calculate_stats().expect("calculate stats");
    let calc_duration = start.elapsed();

    println!("Calculated stats for 1000 hands in {:?}", calc_duration);

    // Verify stats
    assert_eq!(stats.total_hands, 1000);
    assert!(stats.win_rate > 0.0);
    assert!(stats.avg_pot_size > 0.0);

    // Stats calculation should be fast (less than 50ms for 1000 hands)
    assert!(
        calc_duration.as_millis() < 50,
        "Statistics calculation too slow: {:?}",
        calc_duration
    );
}

#[test]
fn test_memory_usage_with_large_dataset() {
    let store = HistoryStore::new();

    // Add a large number of hands
    for i in 0..5000 {
        let hand = create_test_hand(&format!("hand-{:05}", i));
        store.add_hand(hand).expect("add hand");
    }

    // Verify all hands are stored
    let total = store.total_hands().expect("get total");
    assert_eq!(total, 5000);

    // Retrieve subset efficiently
    let recent = store.get_recent_hands(Some(100)).expect("get recent");
    assert_eq!(recent.len(), 100);

    // Recent hands should be in reverse order
    assert_eq!(recent[0].hand_id, "hand-04999");
    assert_eq!(recent[99].hand_id, "hand-04900");
}
