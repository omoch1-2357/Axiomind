use std::fs;
use std::path::PathBuf;

use axiomind_cli::run;
use axiomind_engine::cards::{Card, Rank as R, Suit as S};
use axiomind_engine::logger::{ActionRecord, HandRecord, Street};
use axiomind_engine::player::PlayerAction as A;

fn tmp_jsonl(name: &str) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    p
}

#[test]
fn stats_outputs_summary_json() {
    let path = tmp_jsonl("stats");
    let base = HandRecord {
        hand_id: "20250102-000001".into(),
        seed: Some(1),
        actions: vec![ActionRecord {
            player_id: 0,
            street: Street::Preflop,
            action: A::Bet(10),
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
    let r2 = HandRecord {
        hand_id: "20250102-000002".into(),
        result: Some("p1".into()),
        ..base.clone()
    };
    let r3 = HandRecord {
        hand_id: "20250102-000003".into(),
        result: Some("p0".into()),
        ..base.clone()
    };
    let mut s = String::new();
    for rec in [base, r2, r3] {
        s.push_str(&serde_json::to_string(&rec).unwrap());
        s.push('\n');
    }
    fs::write(&path, s).unwrap();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind",
            "stats",
            "--input",
            path.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    // expect JSON with hands and winners map
    assert!(stdout.contains("\"hands\": 3"));
    assert!(stdout.contains("\"p0\": 2"));
    assert!(stdout.contains("\"p1\": 1"));
}
