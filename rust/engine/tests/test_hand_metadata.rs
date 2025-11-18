use axiomind_engine::cards::{Card, Rank as R, Suit as S};
use axiomind_engine::logger::HandRecord;

#[test]
fn hand_record_supports_timestamp_and_metadata() {
    let rec = HandRecord {
        hand_id: "20250102-000001".to_string(),
        seed: Some(1),
        actions: vec![],
        board: vec![Card {
            suit: S::Clubs,
            rank: R::Ace,
        }],
        result: None,
        ts: Some("2025-01-02T03:04:05Z".to_string()),
        meta: Some(serde_json::json!({"note":"test"})),
        showdown: None,
    };
    let s = serde_json::to_string(&rec).unwrap();
    assert!(s.contains("\"ts\":"));
    assert!(s.contains("\"note\":"));
    let back: HandRecord = serde_json::from_str(&s).unwrap();
    assert_eq!(back.ts.as_deref(), Some("2025-01-02T03:04:05Z"));
}
