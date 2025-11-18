use axiomind_engine::cards::{Card, Rank as R, Suit as S};
use axiomind_engine::logger::{ActionRecord, HandRecord, ShowdownInfo, Street};
use axiomind_engine::player::PlayerAction as A;

#[test]
fn showdown_info_serializes() {
    let rec = HandRecord {
        hand_id: "20250102-000001".into(),
        seed: Some(1),
        actions: vec![ActionRecord {
            player_id: 0,
            street: Street::River,
            action: A::Check,
        }],
        board: vec![Card {
            suit: S::Clubs,
            rank: R::Ace,
        }],
        result: Some("p0".into()),
        ts: None,
        meta: None,
        showdown: Some(ShowdownInfo {
            winners: vec![0],
            notes: Some("kicker A".into()),
        }),
    };
    let s = serde_json::to_string(&rec).unwrap();
    let back: HandRecord = serde_json::from_str(&s).unwrap();
    assert_eq!(back.showdown.unwrap().winners, vec![0]);
}
