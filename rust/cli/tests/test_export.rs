use axiomind_cli::run;
use axiomind_engine::cards::{Card, Rank as R, Suit as S};
use axiomind_engine::logger::{ActionRecord, HandRecord, Street};
use axiomind_engine::player::PlayerAction as A;
use std::fs;
use std::path::PathBuf;

use rusqlite::Connection;
use std::collections::HashMap;

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
        s.push_str(&serde_json::to_string(&r).unwrap());
        s.push('\n');
    }
    fs::write(&p, s).unwrap();
    p
}

#[test]
fn export_to_csv() {
    let input = mk_jsonl("export_in", 3);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let output = input.with_extension("csv");
    let code = run(
        [
            "axiomind",
            "export",
            "--input",
            input.to_string_lossy().as_ref(),
            "--format",
            "csv",
            "--output",
            output.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let csv = fs::read_to_string(&output).unwrap();
    let mut lines = csv.lines();
    let header = lines.next().unwrap();
    assert!(header.contains("hand_id"));
    assert_eq!(lines.count(), 3);
}

#[test]
fn export_to_json_array() {
    let input = mk_jsonl("export_in_json", 2);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let output = input.with_extension("json");
    let code = run(
        [
            "axiomind",
            "export",
            "--input",
            input.to_string_lossy().as_ref(),
            "--format",
            "json",
            "--output",
            output.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let txt = fs::read_to_string(&output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    assert!(v.is_array());
    assert_eq!(v.as_array().unwrap().len(), 2);
}

#[test]
fn export_to_sqlite_creates_schema() {
    let input = mk_jsonl("export_in_sqlite", 3);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let output = input.with_extension("sqlite");
    let code = run(
        [
            "axiomind",
            "export",
            "--input",
            input.to_string_lossy().as_ref(),
            "--format",
            "sqlite",
            "--output",
            output.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(
        code,
        0,
        "expected success, stderr: {}",
        String::from_utf8_lossy(&err)
    );

    let conn = Connection::open(&output).unwrap();

    let mut stmt = conn.prepare("PRAGMA table_info(hands)").unwrap();
    let mut rows = stmt.query([]).unwrap();
    let mut columns: HashMap<String, (String, bool, bool)> = HashMap::new();
    while let Some(row) = rows.next().unwrap() {
        let name: String = row.get(1).unwrap();
        let data_type: String = row.get(2).unwrap_or_default();
        let notnull: i32 = row.get(3).unwrap();
        let pk: i32 = row.get(5).unwrap();
        columns.insert(name, (data_type, notnull != 0, pk != 0));
    }

    let expect_notnull = |field: &str, ty: &str| {
        let entry = columns
            .get(field)
            .unwrap_or_else(|| panic!("missing column {}", field));
        assert_eq!(entry.0, ty);
        assert!(entry.1, "{} should be NOT NULL", field);
        entry.2
    };

    let hand_id_pk = expect_notnull("hand_id", "TEXT");
    assert!(hand_id_pk, "hand_id should be primary key");
    expect_notnull("actions", "INTEGER");
    expect_notnull("board", "INTEGER");
    expect_notnull("raw_json", "TEXT");

    for optional in ["seed", "result", "ts"] {
        let entry = columns
            .get(optional)
            .unwrap_or_else(|| panic!("missing column {}", optional));
        assert!(!entry.1, "{} should allow NULL", optional);
    }

    let raw: String = conn
        .query_row(
            "SELECT raw_json FROM hands WHERE hand_id = ?1",
            [String::from("20250102-000001")],
            |row| row.get(0),
        )
        .unwrap();
    let raw_value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        raw_value.get("hand_id").and_then(|v| v.as_str()),
        Some("20250102-000001"),
    );

    let (actions, board): (i64, i64) = conn
        .query_row(
            "SELECT actions, board FROM hands WHERE hand_id = ?1",
            [&"20250102-000001".to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(actions, 1);
    assert_eq!(board, 1);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM hands", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 3);
}

#[test]
fn export_rejects_unknown_format() {
    let input = mk_jsonl("export_in_invalid", 1);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let output = input.with_extension("bin");
    let code = run(
        [
            "axiomind",
            "export",
            "--input",
            input.to_string_lossy().as_ref(),
            "--format",
            "xml",
            "--output",
            output.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 2);
    let stderr = String::from_utf8(err).unwrap();
    assert!(stderr.contains("Unsupported format"));
    assert!(!output.exists());
}
