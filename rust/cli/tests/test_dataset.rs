use axiomind_cli::run;
use axiomind_engine::cards::{Card, Rank as R, Suit as S};
use axiomind_engine::logger::{ActionRecord, HandRecord, Street};
use axiomind_engine::player::PlayerAction as A;
use std::fs;
use std::path::PathBuf;

fn mk_jsonl(name: &str, n: usize) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
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
    let mut s = String::new();
    for i in 0..n {
        let mut r = base.clone();
        r.hand_id = format!("20250102-{:06}", i + 1);
        if i % 3 == 0 {
            r.result = Some("p1".into());
        }
        s.push_str(&serde_json::to_string(&r).unwrap());
        s.push('\n');
    }
    fs::write(&p, s).unwrap();
    p
}

#[test]
fn dataset_random_split_creates_files() {
    let input = mk_jsonl("dataset_in", 10);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let outdir = PathBuf::from("target").join(format!("ds_{}", std::process::id()));
    let code = run(
        [
            "axiomind",
            "dataset",
            "--input",
            input.to_string_lossy().as_ref(),
            "--outdir",
            outdir.to_string_lossy().as_ref(),
            "--train",
            "0.7",
            "--val",
            "0.2",
            "--test",
            "0.1",
            "--seed",
            "7",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let t = fs::read_to_string(outdir.join("train.jsonl")).unwrap();
    let v = fs::read_to_string(outdir.join("val.jsonl")).unwrap();
    let te = fs::read_to_string(outdir.join("test.jsonl")).unwrap();
    let cnt = t.lines().count() + v.lines().count() + te.lines().count();
    assert_eq!(cnt, 10);
}

#[test]
fn dataset_default_split() {
    let input = mk_jsonl("dataset_in_default", 5);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let outdir = PathBuf::from("target").join(format!("dsd_{}", std::process::id()));
    let code = run(
        [
            "axiomind",
            "dataset",
            "--input",
            input.to_string_lossy().as_ref(),
            "--outdir",
            outdir.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let t = fs::read_to_string(outdir.join("train.jsonl")).unwrap();
    assert!(t.lines().count() >= 3); // 80%
}

#[test]
fn dataset_rejects_invalid_percentages() {
    let input = mk_jsonl("dataset_bad_pct", 4);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let outdir = PathBuf::from("target").join(format!("ds_bad_{}", std::process::id()));
    let code = run(
        [
            "axiomind",
            "dataset",
            "--input",
            input.to_string_lossy().as_ref(),
            "--outdir",
            outdir.to_string_lossy().as_ref(),
            "--train",
            "60",
            "--val",
            "20",
            "--test",
            "30",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 2);
    let err_str = String::from_utf8(err).unwrap();
    assert!(
        err_str.contains("Splits must sum to 100%"),
        "unexpected stderr: {}",
        err_str
    );
}

#[test]
fn dataset_accepts_percentage_inputs() {
    let input = mk_jsonl("dataset_pct", 10);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let outdir = PathBuf::from("target").join(format!("ds_pct_{}", std::process::id()));
    let _ = fs::remove_dir_all(&outdir);
    let code = run(
        [
            "axiomind",
            "dataset",
            "--input",
            input.to_string_lossy().as_ref(),
            "--outdir",
            outdir.to_string_lossy().as_ref(),
            "--train",
            "70",
            "--val",
            "20",
            "--test",
            "10",
            "--seed",
            "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&err));
    let train = fs::read_to_string(outdir.join("train.jsonl")).unwrap();
    let val = fs::read_to_string(outdir.join("val.jsonl")).unwrap();
    let test = fs::read_to_string(outdir.join("test.jsonl")).unwrap();
    assert_eq!(train.lines().count(), 7);
    assert_eq!(val.lines().count(), 2);
    assert_eq!(test.lines().count(), 1);
}

#[test]
fn dataset_seed_produces_stable_splits() {
    let input = mk_jsonl("dataset_seed", 12);
    let outdir_a = PathBuf::from("target").join(format!("ds_seed_a_{}", std::process::id()));
    let outdir_b = PathBuf::from("target").join(format!("ds_seed_b_{}", std::process::id()));
    let _ = fs::remove_dir_all(&outdir_a);
    let _ = fs::remove_dir_all(&outdir_b);

    let mut out_a = Vec::new();
    let mut err_a = Vec::new();
    let code_a = run(
        [
            "axiomind",
            "dataset",
            "--input",
            input.to_string_lossy().as_ref(),
            "--outdir",
            outdir_a.to_string_lossy().as_ref(),
            "--train",
            "0.8",
            "--val",
            "0.1",
            "--test",
            "0.1",
            "--seed",
            "99",
        ],
        &mut out_a,
        &mut err_a,
    );
    assert_eq!(code_a, 0, "stderr: {}", String::from_utf8_lossy(&err_a));

    let mut out_b = Vec::new();
    let mut err_b = Vec::new();
    let code_b = run(
        [
            "axiomind",
            "dataset",
            "--input",
            input.to_string_lossy().as_ref(),
            "--outdir",
            outdir_b.to_string_lossy().as_ref(),
            "--train",
            "0.8",
            "--val",
            "0.1",
            "--test",
            "0.1",
            "--seed",
            "99",
        ],
        &mut out_b,
        &mut err_b,
    );
    assert_eq!(code_b, 0, "stderr: {}", String::from_utf8_lossy(&err_b));

    let tr_a = fs::read_to_string(outdir_a.join("train.jsonl")).unwrap();
    let tr_b = fs::read_to_string(outdir_b.join("train.jsonl")).unwrap();
    let va_a = fs::read_to_string(outdir_a.join("val.jsonl")).unwrap();
    let va_b = fs::read_to_string(outdir_b.join("val.jsonl")).unwrap();
    let te_a = fs::read_to_string(outdir_a.join("test.jsonl")).unwrap();
    let te_b = fs::read_to_string(outdir_b.join("test.jsonl")).unwrap();

    assert_eq!(tr_a, tr_b, "train split mismatch");
    assert_eq!(va_a, va_b, "val split mismatch");
    assert_eq!(te_a, te_b, "test split mismatch");
}
