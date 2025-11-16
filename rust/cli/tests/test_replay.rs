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
fn replay_displays_single_hand() {
    let path = tmp_jsonl("replay_single");
    let rec = HandRecord {
        hand_id: "20250102-000001".into(),
        seed: Some(42),
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
    assert!(stdout.contains("Hand #1"));
    assert!(stdout.contains("Seed: 42"));
    assert!(stdout.contains("Level: 1"));
    assert!(stdout.contains("Blinds: SB=50 BB=100"));
    assert!(stdout.contains("Preflop:"));
    assert!(stdout.contains("Player 0: bet 50"));
    assert!(stdout.contains("Replay complete. 1 hands shown."));
}

#[test]
fn replay_displays_speed_parameter_warning() {
    let path = tmp_jsonl("replay_speed_warning");
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
        [
            "axm",
            "replay",
            "--input",
            path.to_string_lossy().as_ref(),
            "--speed",
            "2.0",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("Note: --speed parameter") && stderr.contains("is not yet used"),
        "Expected speed parameter warning in stderr, got: {}",
        stderr
    );
}

#[test]
fn replay_displays_board_cards_by_street() {
    let path = tmp_jsonl("replay_board_streets");
    let rec = HandRecord {
        hand_id: "20250102-000001".into(),
        seed: Some(1),
        actions: vec![
            ActionRecord {
                player_id: 0,
                street: Street::Preflop,
                action: A::Bet(100),
            },
            ActionRecord {
                player_id: 1,
                street: Street::Preflop,
                action: A::Call,
            },
            ActionRecord {
                player_id: 0,
                street: Street::Flop,
                action: A::Bet(200),
            },
            ActionRecord {
                player_id: 1,
                street: Street::Flop,
                action: A::Call,
            },
        ],
        board: vec![
            Card {
                suit: S::Hearts,
                rank: R::Ace,
            },
            Card {
                suit: S::Diamonds,
                rank: R::King,
            },
            Card {
                suit: S::Clubs,
                rank: R::Seven,
            },
        ],
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
    assert!(stdout.contains("Preflop:"), "Should show Preflop street");
    assert!(stdout.contains("Flop:"), "Should show Flop street");
    assert!(
        stdout.contains("Player 0: bet 100"),
        "Should show preflop bet"
    );
    assert!(stdout.contains("Player 1: call"), "Should show call");
    assert!(stdout.contains("Player 0: bet 200"), "Should show flop bet");
}
