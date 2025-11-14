use std::fs;
use std::path::PathBuf;

use axm_cli::run;
use axm_engine::cards::{Card, Rank as R, Suit as S};
use axm_engine::logger::{ActionRecord, HandRecord, Street};
use axm_engine::player::PlayerAction as A;

fn tmp_jsonl(name: &str) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    p
}

#[test]
fn replay_counts_hands_and_prints_summary() {
    let path = tmp_jsonl("replay");
    // build two records
    let rec1 = HandRecord {
        hand_id: "20250102-000001".into(),
        seed: Some(1),
        actions: vec![ActionRecord {
            player_id: 0,
            street: Street::Preflop,
            action: A::Bet(50),
        }],
        board: vec![Card {
            suit: S::Clubs,
            rank: R::Ace,
        }],
        result: Some("p0".into()),
        ts: None,
        meta: None,
        showdown: None,
    };
    let rec2 = HandRecord {
        hand_id: "20250102-000002".into(),
        ..rec1.clone()
    };
    let mut s = String::new();
    s.push_str(&serde_json::to_string(&rec1).unwrap());
    s.push('\n');
    s.push_str(&serde_json::to_string(&rec2).unwrap());
    s.push('\n');
    fs::write(&path, s).unwrap();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        ["axm", "replay", "--input", path.to_string_lossy().as_ref()],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Counted: 2 hands in file"));
}

#[test]
fn replay_displays_warning_about_missing_functionality() {
    let path = tmp_jsonl("replay_warning");
    let rec = HandRecord {
        hand_id: "20250102-000001".into(),
        seed: Some(1),
        actions: vec![ActionRecord {
            player_id: 0,
            street: Street::Preflop,
            action: A::Bet(50),
        }],
        board: vec![Card {
            suit: S::Clubs,
            rank: R::Ace,
        }],
        result: Some("p0".into()),
        ts: None,
        meta: None,
        showdown: None,
    };
    fs::write(&path, serde_json::to_string(&rec).unwrap() + "\n").unwrap();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        ["axm", "replay", "--input", path.to_string_lossy().as_ref()],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("Note: Full visual replay not yet implemented. This command only counts hands in the file."),
        "Expected missing functionality note in stderr, got: {}",
        stderr
    );
}

#[test]
fn replay_outputs_counted_not_replayed() {
    let path = tmp_jsonl("replay_counted");
    let rec = HandRecord {
        hand_id: "20250102-000001".into(),
        seed: Some(1),
        actions: vec![ActionRecord {
            player_id: 0,
            street: Street::Preflop,
            action: A::Bet(50),
        }],
        board: vec![Card {
            suit: S::Clubs,
            rank: R::Ace,
        }],
        result: Some("p0".into()),
        ts: None,
        meta: None,
        showdown: None,
    };
    fs::write(&path, serde_json::to_string(&rec).unwrap() + "\n").unwrap();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        ["axm", "replay", "--input", path.to_string_lossy().as_ref()],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        stdout.contains("Counted: 1 hands in file"),
        "Expected 'Counted: N hands in file' message, got: {}",
        stdout
    );
    assert!(
        !stdout.contains("Replayed:"),
        "Should not contain 'Replayed:', got: {}",
        stdout
    );
}
