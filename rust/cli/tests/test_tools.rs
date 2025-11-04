use axm_cli::run;
use axm_engine::cards::{Card, Rank as R, Suit as S};
use axm_engine::logger::{ActionRecord, HandRecord, Street};
use axm_engine::player::PlayerAction as A;
use std::fs;
use std::path::PathBuf;

use crate::helpers::cli_runner::CliRunner;

mod helpers;

fn tmp_jsonl(name: &str) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    p
}

#[test]
fn verify_checks_records() {
    let path = tmp_jsonl("verify");
    // valid completed board (5 cards)
    let rec = HandRecord {
        hand_id: "20250102-000001".into(),
        seed: Some(1),
        actions: vec![ActionRecord {
            player_id: 0,
            street: Street::River,
            action: A::Check,
        }],
        board: vec![
            Card {
                suit: S::Clubs,
                rank: R::Ace,
            },
            Card {
                suit: S::Diamonds,
                rank: R::Two,
            },
            Card {
                suit: S::Hearts,
                rank: R::Three,
            },
            Card {
                suit: S::Spades,
                rank: R::Four,
            },
            Card {
                suit: S::Clubs,
                rank: R::Five,
            },
        ],
        result: Some("p0".into()),
        ts: None,
        meta: None,
        showdown: None,
    };
    let mut s = String::new();
    s.push_str(&serde_json::to_string(&rec).unwrap());
    s.push('\n');
    fs::write(&path, s).unwrap();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        ["axm", "verify", "--input", path.to_string_lossy().as_ref()],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Verify: OK"));
    assert!(stdout.contains("hands=1"));
}

#[test]
fn doctor_reports_ok() {
    let _guard = helpers::cli_runner::DOCTOR_LOCK
        .lock()
        .expect("doctor lock");

    let base = PathBuf::from("target").join(format!("doctor_ok_{}", std::process::id()));
    let sqlite_dir = base.join("sqlite");
    let data_dir = base.join("data");
    fs::create_dir_all(&sqlite_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    let env_pairs = [
        (
            "AXM_DOCTOR_SQLITE_DIR".to_string(),
            sqlite_dir.to_string_lossy().into_owned(),
        ),
        (
            "AXM_DOCTOR_DATA_DIR".to_string(),
            data_dir.to_string_lossy().into_owned(),
        ),
        (
            "AXM_DOCTOR_LOCALE_OVERRIDE".to_string(),
            "en_US.UTF-8".to_string(),
        ),
    ];
    let env_refs: Vec<(&str, &str)> = env_pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run_with_env(&["doctor"], &env_refs);

    assert_eq!(res.exit_code, 0, "doctor should succeed: {}", res.stderr);
    let stdout = res.stdout.to_lowercase();
    assert!(stdout.contains("\"sqlite\""), "stdout: {}", res.stdout);
    assert!(stdout.contains("\"data_dir\""), "stdout: {}", res.stdout);
    assert!(stdout.contains("\"locale\""), "stdout: {}", res.stdout);
    assert!(
        stdout.contains("\"status\": \"ok\""),
        "stdout: {}",
        res.stdout
    );
    assert!(res.stderr.is_empty(), "stderr: {}", res.stderr);
}

#[test]
fn bench_runs_quickly() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm", "bench"], &mut out, &mut err);
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Benchmark:"));
}
