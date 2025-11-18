use std::fs;
use std::path::PathBuf;

use axiomind_engine::cards::{Card, Rank as R, Suit as S};
use axiomind_engine::logger::{ActionRecord, HandLogger, HandRecord, Street};
use axiomind_engine::player::PlayerAction;

fn tmp_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    p
}

#[test]
fn writes_jsonl_with_lf_only() {
    let path = tmp_path("handlog");
    let mut logger = HandLogger::create(&path).expect("create logger");
    let rec = HandRecord {
        hand_id: "20250102-000001".to_string(),
        seed: Some(1),
        actions: vec![ActionRecord {
            player_id: 0,
            street: Street::Preflop,
            action: PlayerAction::Check,
        }],
        board: vec![Card {
            suit: S::Clubs,
            rank: R::Ace,
        }],
        result: Some("p0".to_string()),
        ts: None,
        meta: None,
        showdown: None,
    };
    logger.write(&rec).expect("write");
    let bytes = fs::read(&path).expect("read file");
    assert!(bytes.ends_with(b"\n"));
    assert!(!bytes.contains(&b'\r'));
}

#[test]
fn sequential_ids_increment() {
    let mut logger = HandLogger::with_seq_for_test("20251231");
    assert_eq!(logger.next_id(), "20251231-000001");
    assert_eq!(logger.next_id(), "20251231-000002");
}

#[test]
fn ts_is_generated_when_missing_and_preserved_when_present() {
    let path = tmp_path("handlog_ts");
    let mut logger = HandLogger::create(&path).expect("create logger");
    // missing ts -> logger should inject it
    let rec = HandRecord {
        hand_id: "20250102-000010".to_string(),
        seed: Some(7),
        actions: vec![],
        board: vec![Card {
            suit: S::Clubs,
            rank: R::Ace,
        }],
        result: None,
        ts: None,
        meta: None,
        showdown: None,
    };
    logger.write(&rec).expect("write");
    let line = String::from_utf8(fs::read(&path).unwrap()).unwrap();
    assert!(line.contains("\"ts\":"), "ts should be injected");

    // preset ts should be preserved
    let preset = "2030-01-01T00:00:00Z".to_string();
    let rec2 = HandRecord {
        ts: Some(preset.clone()),
        ..rec
    };
    logger.write(&rec2).expect("write2");
    let content = String::from_utf8(fs::read(&path).unwrap()).unwrap();
    assert!(content.contains(&preset), "preset ts must be kept");
}
