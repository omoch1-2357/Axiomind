use axiomind_engine::cards::{Card, Rank, Suit};
use axiomind_engine::logger::{ActionRecord, HandRecord, Street, format_hand_id};
use axiomind_engine::player::PlayerAction;

#[test]
fn hand_record_serializes_and_deserializes() {
    let rec = HandRecord {
        hand_id: "20250102-000123".to_string(),
        seed: Some(42),
        actions: vec![
            ActionRecord {
                player_id: 0,
                street: Street::Preflop,
                action: PlayerAction::Bet(50),
            },
            ActionRecord {
                player_id: 1,
                street: Street::Preflop,
                action: PlayerAction::Call,
            },
        ],
        board: vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ace,
            },
        ],
        result: Some("p0 wins".to_string()),
        ts: None,
        meta: None,
        showdown: None,
    };

    let s = serde_json::to_string(&rec).expect("serialize");
    let back: HandRecord = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(rec, back);
}

#[test]
fn id_format_matches_spec() {
    let id = format_hand_id("20251231", 42);
    assert_eq!(id, "20251231-000042");
}
