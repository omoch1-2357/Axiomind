use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;
use serde_json::{json, Value};
use std::path::PathBuf;

static DOCTOR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

use std::fs::File;
use std::io::Write;

fn standard_board() -> Vec<Value> {
    vec![
        json!({"rank": "Ace", "suit": "Hearts"}),
        json!({"rank": "King", "suit": "Diamonds"}),
        json!({"rank": "Queen", "suit": "Spades"}),
        json!({"rank": "Jack", "suit": "Clubs"}),
        json!({"rank": "Ten", "suit": "Hearts"}),
    ]
}

fn write_records(tfm: &TempFileManager, name: &str, records: &[Value]) -> PathBuf {
    let serialized = records
        .iter()
        .map(|rec| serde_json::to_string(rec).expect("serialize record"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut content = serialized;
    content.push('\n');
    tfm.create_file(name, &content).expect("create file")
}

#[test]
fn b1_verify_rejects_additional_hands_after_bust() {
    let tfm = TempFileManager::new().expect("temp dir");
    let bust_hand = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p1",
        "showdown": null,
        "net_result": {"p0": -100, "p1": 100},
        "end_reason": "player_bust",
        "ts": "2025-01-01T00:00:00Z"
    });
    let continued_hand = json!({
        "hand_id": "19700101-000002",
        "seed": 2,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 0},
            {"id": "p1", "stack_start": 200}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p1",
        "showdown": null,
        "net_result": {"p0": 0, "p1": 0},
        "end_reason": "continue",
        "ts": "2025-01-01T00:02:00Z"
    });
    let path = write_records(&tfm, "stack_zero.jsonl", &[bust_hand, continued_hand]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when hands continue after bust"
    );
    assert!(
        res.stderr.to_lowercase().contains("stack"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn b2_verify_chip_conservation_passes_when_sum_zero() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 50, "p1": -50},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let path = write_records(&tfm, "ok.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_eq!(res.exit_code, 0, "verify should pass: {}", res.stderr);
}

#[test]
fn b2_verify_chip_conservation_fails_when_sum_nonzero() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 50, "p1": -40},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let path = write_records(&tfm, "bad.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(res.exit_code, 0);
    assert!(
        res.stderr.to_lowercase().contains("chip"),
        "stderr: {}",
        res.stderr
    );
}
#[test]
fn b3_verify_rejects_invalid_chip_unit_bet() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [
            {"player_id": 0, "street": "Preflop", "action": {"Bet": 30}}
        ],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 50, "p1": -50},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let path = write_records(&tfm, "invalid_bet.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(res.exit_code, 0, "verify should fail on invalid bet size");
    assert!(
        res.stderr.to_lowercase().contains("bet"),
        "stderr: {}",
        res.stderr
    );
}
#[test]
fn b4_verify_rejects_under_minimum_raise_delta() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 1000}
        ],
        "actions": [
            {"player_id": 0, "street": "Preflop", "action": {"Bet": 100}},
            {"player_id": 1, "street": "Preflop", "action": {"Raise": 50}}
        ],
        "board": standard_board(),
        "result": "p1",
        "showdown": null,
        "net_result": {"p0": -100, "p1": 100},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let path = write_records(&tfm, "invalid_raise.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(res.exit_code, 0, "verify should fail on short raise");
    assert!(
        res.stderr.to_lowercase().contains("raise"),
        "stderr: {}",
        res.stderr
    );
}
#[test]
fn b5_verify_flags_raise_after_short_all_in() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 250}
        ],
        "actions": [
            {"player_id": 0, "street": "Preflop", "action": {"Bet": 200}},
            {"player_id": 1, "street": "Preflop", "action": "AllIn"},
            {"player_id": 0, "street": "Preflop", "action": {"Raise": 200}}
        ],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 250, "p1": -250},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:05:00Z"
    });
    let path = write_records(&tfm, "reopen_after_allin.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when betting reopens after short all-in"
    );
    assert!(
        res.stderr.to_lowercase().contains("all-in") || res.stderr.to_lowercase().contains("raise"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn b6_verify_rejects_invalid_dealing_sequence() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000003",
        "seed": 3,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "p0",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 1000}
        ],
        "actions": [],
        "board": standard_board(),
        "result": null,
        "showdown": null,
        "net_result": {"p0": 0, "p1": 0},
        "end_reason": "showdown",
        "meta": {
            "small_blind": "p0",
            "big_blind": "p1",
            "deal_sequence": ["p1", "p0", "p0", "p1"],
            "burn_positions": [5, 9, 11]
        },
        "ts": "2025-01-01T00:10:00Z"
    });
    let path = write_records(&tfm, "bad_deal.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail on invalid dealing sequence"
    );
    assert!(
        res.stderr.to_lowercase().contains("deal"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn b6_verify_accepts_valid_dealing_sequence() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000004",
        "seed": 4,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "p0",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 1000}
        ],
        "actions": [],
        "board": standard_board(),
        "result": null,
        "showdown": null,
        "net_result": {"p0": 0, "p1": 0},
        "end_reason": "showdown",
        "meta": {
            "small_blind": "p0",
            "big_blind": "p1",
            "deal_sequence": ["p0", "p1", "p0", "p1"],
            "burn_positions": [5, 9, 11]
        },
        "ts": "2025-01-01T00:11:00Z"
    });
    let path = write_records(&tfm, "good_deal.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_eq!(
        res.exit_code, 0,
        "verify should pass on valid dealing sequence"
    );
}

#[test]
fn j1_verify_rejects_unknown_player_action() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000010",
        "seed": 10,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "p0",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 1000}
        ],
        "actions": [
            {
                "player_id": 9,
                "street": "Preflop",
                "action": {"Bet": 100}
            }
        ],
        "board": standard_board(),
        "result": null,
        "showdown": null,
        "net_result": {"p0": 0, "p1": 0},
        "end_reason": "showdown",
        "meta": {
            "small_blind": "p0",
            "big_blind": "p1",
            "deal_sequence": ["p0", "p1", "p0", "p1"],
            "burn_positions": [5, 9, 11]
        },
        "ts": "2025-01-01T00:20:00Z"
    });
    let path = write_records(&tfm, "unknown_action.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when actions reference unknown players"
    );
    assert!(
        res.stderr.to_lowercase().contains("unknown player"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn j2_verify_rejects_unknown_player_net_result() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000011",
        "seed": 11,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "p0",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 1000}
        ],
        "actions": [],
        "board": standard_board(),
        "result": null,
        "showdown": null,
        "net_result": {"p0": 100, "p9": -100},
        "end_reason": "showdown",
        "meta": {
            "small_blind": "p0",
            "big_blind": "p1",
            "deal_sequence": ["p0", "p1", "p0", "p1"],
            "burn_positions": [5, 9, 11]
        },
        "ts": "2025-01-01T00:30:00Z"
    });
    let path = write_records(&tfm, "unknown_net_result.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail on unknown net_result player"
    );
    assert!(
        res.stderr.to_lowercase().contains("net_result"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn j3_doctor_reports_all_checks_ok() {
    let _guard = DOCTOR_LOCK.lock().expect("doctor lock");
    let tfm = TempFileManager::new().expect("temp dir");
    let sqlite_dir = tfm.create_directory("sqlite_ok").expect("sqlite dir");
    let data_dir = tfm.create_directory("data_ok").expect("data dir");

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
    assert!(
        stdout.contains("\"status\": \"ok\""),
        "stdout: {}",
        res.stdout
    );
    assert!(res.stderr.is_empty(), "stderr: {}", res.stderr);
}

#[test]
fn j4_doctor_reports_sqlite_permission_error() {
    let _guard = DOCTOR_LOCK.lock().expect("doctor lock");
    let tfm = TempFileManager::new().expect("temp dir");
    let blocker = tfm
        .create_file("blocked/location", "content")
        .expect("create file");
    let data_dir = tfm.create_directory("data_ok").expect("data dir");

    let env_pairs = [
        (
            "AXM_DOCTOR_SQLITE_DIR".to_string(),
            blocker.to_string_lossy().into_owned(),
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

    assert_ne!(res.exit_code, 0, "doctor should fail on sqlite check");
    assert!(
        res.stderr.to_lowercase().contains("sqlite"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn j5_doctor_reports_data_dir_error() {
    let _guard = DOCTOR_LOCK.lock().expect("doctor lock");
    let tfm = TempFileManager::new().expect("temp dir");
    let existing_file = tfm
        .create_file("not_a_dir", "content")
        .expect("create file");
    let sqlite_dir = tfm.create_directory("sqlite_ok").expect("sqlite dir");

    let env_pairs = [
        (
            "AXM_DOCTOR_SQLITE_DIR".to_string(),
            sqlite_dir.to_string_lossy().into_owned(),
        ),
        (
            "AXM_DOCTOR_DATA_DIR".to_string(),
            existing_file.to_string_lossy().into_owned(),
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

    assert_ne!(
        res.exit_code, 0,
        "doctor should fail on data directory check"
    );
    assert!(
        res.stderr.to_lowercase().contains("data"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn j6_doctor_reports_locale_error() {
    let _guard = DOCTOR_LOCK.lock().expect("doctor lock");
    let tfm = TempFileManager::new().expect("temp dir");
    let sqlite_dir = tfm.create_directory("sqlite_ok").expect("sqlite dir");
    let data_dir = tfm.create_directory("data_ok").expect("data dir");

    let env_pairs = [
        (
            "AXM_DOCTOR_SQLITE_DIR".to_string(),
            sqlite_dir.to_string_lossy().into_owned(),
        ),
        (
            "AXM_DOCTOR_DATA_DIR".to_string(),
            data_dir.to_string_lossy().into_owned(),
        ),
        ("AXM_DOCTOR_LOCALE_OVERRIDE".to_string(), "C".to_string()),
    ];
    let env_refs: Vec<(&str, &str)> = env_pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run_with_env(&["doctor"], &env_refs);

    assert_ne!(res.exit_code, 0, "doctor should fail on locale check");
    assert!(
        res.stderr.to_lowercase().contains("locale"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn j7_rng_with_seed_is_deterministic() {
    let cli = CliRunner::new().expect("cli runner");
    let first = cli.run(&["rng", "--seed", "42"]);
    assert_eq!(first.exit_code, 0, "first rng run failed: {}", first.stderr);
    let second = cli.run(&["rng", "--seed", "42"]);
    assert_eq!(
        second.exit_code, 0,
        "second rng run failed: {}",
        second.stderr
    );

    let first_out = first.stdout.clone();
    let second_out = second.stdout.clone();
    assert_eq!(
        first_out, second_out,
        "same seed should produce identical RNG output"
    );
    assert!(first_out.contains("RNG sample:"), "stdout: {}", first_out);
    assert!(first.stderr.is_empty(), "stderr: {}", first.stderr);
    assert!(second.stderr.is_empty(), "stderr: {}", second.stderr);
}

#[test]
fn j8_verify_rejects_duplicate_cards() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000012",
        "seed": 12,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "p0",
        "players": [
            {
                "id": "p0",
                "stack_start": 1000,
                "hole_cards": [
                    {"rank": "Ace", "suit": "Clubs"},
                    {"rank": "King", "suit": "Diamonds"}
                ]
            },
            {
                "id": "p1",
                "stack_start": 1000,
                "hole_cards": [
                    {"rank": "Queen", "suit": "Spades"},
                    {"rank": "Jack", "suit": "Hearts"}
                ]
            }
        ],
        "actions": [],
        "board": [
            {"rank": "Ace", "suit": "Hearts"},
            {"rank": "Ace", "suit": "Hearts"},
            {"rank": "Queen", "suit": "Clubs"},
            {"rank": "Jack", "suit": "Spades"},
            {"rank": "Ten", "suit": "Diamonds"}
        ],
        "result": null,
        "showdown": null,
        "net_result": {"p0": 0, "p1": 0},
        "end_reason": "showdown",
        "meta": {
            "small_blind": "p0",
            "big_blind": "p1",
            "deal_sequence": ["p0", "p1", "p0", "p1"],
            "burn_positions": [5, 9, 11]
        },
        "ts": "2025-01-01T00:40:00Z"
    });
    let path = write_records(&tfm, "duplicate_cards.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when duplicate cards appear"
    );
    assert!(
        res.stderr.to_lowercase().contains("duplicate"),
        "stderr: {}",
        res.stderr
    );
    assert!(
        res.stdout.to_lowercase().contains("fail"),
        "stdout: {}",
        res.stdout
    );
}

#[test]
fn j9_verify_rejects_incorrect_burn_positions() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000013",
        "seed": 13,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "p0",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 1000}
        ],
        "actions": [],
        "board": standard_board(),
        "result": null,
        "showdown": null,
        "net_result": {"p0": 0, "p1": 0},
        "end_reason": "showdown",
        "meta": {
            "small_blind": "p0",
            "big_blind": "p1",
            "deal_sequence": ["p0", "p1", "p0", "p1"],
            "burn_positions": [5, 8, 11]
        },
        "ts": "2025-01-01T00:50:00Z"
    });
    let path = write_records(&tfm, "bad_burn_positions.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when burn positions do not match expected"
    );
    assert!(
        res.stderr.to_lowercase().contains("burn"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn j10_verify_rejects_short_board() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000014",
        "seed": 14,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "p0",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 1000}
        ],
        "actions": [],
        "board": [
            {"rank": "Ace", "suit": "Hearts"},
            {"rank": "King", "suit": "Diamonds"},
            {"rank": "Queen", "suit": "Spades"},
            {"rank": "Jack", "suit": "Clubs"}
        ],
        "result": null,
        "showdown": null,
        "net_result": {"p0": 0, "p1": 0},
        "end_reason": "showdown",
        "meta": {
            "small_blind": "p0",
            "big_blind": "p1",
            "deal_sequence": ["p0", "p1", "p0", "p1"],
            "burn_positions": [5, 9, 11]
        },
        "ts": "2025-01-01T01:00:00Z"
    });
    let path = write_records(&tfm, "short_board.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when board has fewer than five cards"
    );
    assert!(
        res.stderr.to_lowercase().contains("board"),
        "stderr: {}",
        res.stderr
    );
    assert!(
        res.stdout.to_lowercase().contains("fail"),
        "stdout: {}",
        res.stdout
    );
}

#[test]
fn l1_verify_rejects_unexpected_player_join() {
    let tfm = TempFileManager::new().expect("temp dir");
    let opening_hand = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 10, "p1": -10},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let roster_change = json!({
        "hand_id": "19700101-000002",
        "seed": 2,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 110},
            {"id": "p2", "stack_start": 90}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p2",
        "showdown": null,
        "net_result": {"p0": -10, "p2": 10},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:02:00Z"
    });
    let path = write_records(
        &tfm,
        "roster_expansion.jsonl",
        &[opening_hand, roster_change],
    );

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when a new player joins mid-match"
    );
    assert!(
        res.stderr
            .to_lowercase()
            .contains("unexpected player p2 at hand 2"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn l2_verify_rejects_missing_player_without_elimination() {
    let tfm = TempFileManager::new().expect("temp dir");
    let first_hand = json!({
        "hand_id": "19700101-000010",
        "seed": 10,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p1",
        "showdown": null,
        "net_result": {"p0": -10, "p1": 10},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let missing_player_hand = json!({
        "hand_id": "19700101-000011",
        "seed": 11,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 90}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 0},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:02:00Z"
    });
    let path = write_records(
        &tfm,
        "roster_missing.jsonl",
        &[first_hand, missing_player_hand],
    );

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when a tracked player disappears without busting"
    );
    assert!(
        res.stderr
            .to_lowercase()
            .contains("missing player p1 at hand 2"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn l3_verify_accepts_stable_roster_with_rotation() {
    let tfm = TempFileManager::new().expect("temp dir");
    let first_hand = json!({
        "hand_id": "19700101-000100",
        "seed": 100,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p1",
        "showdown": null,
        "net_result": {"p0": -25, "p1": 25},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let rotated_hand = json!({
        "hand_id": "19700101-000101",
        "seed": 101,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p1", "stack_start": 125},
            {"id": "p0", "stack_start": 75}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 25, "p1": -25},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:02:00Z"
    });
    let path = write_records(&tfm, "roster_rotation.jsonl", &[first_hand, rotated_hand]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_eq!(
        res.exit_code, 0,
        "verify should pass when roster stays constant"
    );
    assert!(
        res.stdout.to_lowercase().contains("verify: ok (hands=2)"),
        "stdout: {}",
        res.stdout
    );
}

// M-series: Cross-platform compatibility tests
#[test]
fn m1_stats_accepts_utf8_bom_records() {
    let tfm = TempFileManager::new().expect("temp dir");
    let records = [
        json!({
            "hand_id": "19700101-000200",
            "seed": 200,
            "level": 1,
            "blinds": {"sb": 50, "bb": 100},
            "button": "BTN",
            "players": [
                {"id": "p0", "stack_start": 100},
                {"id": "p1", "stack_start": 100}
            ],
            "actions": [],
            "board": standard_board(),
            "result": "p0",
            "showdown": null,
            "net_result": {"p0": 25, "p1": -25},
            "end_reason": "showdown",
            "ts": "2025-01-02T00:00:00Z"
        }),
        json!({
            "hand_id": "19700101-000201",
            "seed": 201,
            "level": 1,
            "blinds": {"sb": 50, "bb": 100},
            "button": "BTN",
            "players": [
                {"id": "p0", "stack_start": 100},
                {"id": "p1", "stack_start": 100}
            ],
            "actions": [],
            "board": standard_board(),
            "result": "p1",
            "showdown": null,
            "net_result": {"p0": -25, "p1": 25},
            "end_reason": "showdown",
            "ts": "2025-01-02T00:01:00Z"
        }),
    ];
    let serialized: Vec<String> = records
        .iter()
        .map(|rec| serde_json::to_string(rec).expect("serialize record"))
        .collect();
    let content = format!("\u{feff}{}\r\n{}\r\n", serialized[0], serialized[1]);
    let path = tfm
        .create_file("stats_bom.jsonl", &content)
        .expect("create file");
    let cli = CliRunner::new().expect("cli runner");
    let input_path = path.to_string_lossy().into_owned();
    let res = cli.run(&["stats", "--input", &input_path]);
    assert_eq!(
        res.exit_code, 0,
        "stats should accept UTF-8 BOM input, stderr: {}",
        res.stderr
    );
    assert!(
        res.stdout.contains("\"hands\": 2"),
        "stdout: {}",
        res.stdout
    );
}

#[test]
fn m2_stats_accepts_utf8_bom_in_compressed_records() {
    let tfm = TempFileManager::new().expect("temp dir");
    let records = [
        json!({
            "hand_id": "19700101-000300",
            "seed": 300,
            "level": 1,
            "blinds": {"sb": 50, "bb": 100},
            "button": "BTN",
            "players": [
                {"id": "p0", "stack_start": 100},
                {"id": "p1", "stack_start": 100}
            ],
            "actions": [],
            "board": standard_board(),
            "result": "p0",
            "showdown": null,
            "net_result": {"p0": 40, "p1": -40},
            "end_reason": "showdown",
            "ts": "2025-01-02T01:00:00Z"
        }),
        json!({
            "hand_id": "19700101-000301",
            "seed": 301,
            "level": 1,
            "blinds": {"sb": 50, "bb": 100},
            "button": "BTN",
            "players": [
                {"id": "p0", "stack_start": 100},
                {"id": "p1", "stack_start": 100}
            ],
            "actions": [],
            "board": standard_board(),
            "result": "p1",
            "showdown": null,
            "net_result": {"p0": -40, "p1": 40},
            "end_reason": "showdown",
            "ts": "2025-01-02T01:01:00Z"
        }),
    ];
    let serialized: Vec<String> = records
        .iter()
        .map(|rec| serde_json::to_string(rec).expect("serialize record"))
        .collect();
    let content = format!("\u{feff}{}\r\n{}\r\n", serialized[0], serialized[1]);
    let dir = tfm.create_directory("compressed").expect("dir");
    let path = dir.join("stats_bom.jsonl.zst");
    let file = File::create(&path).expect("create zst file");
    let mut encoder = zstd::stream::write::Encoder::new(file, 0).expect("create zst encoder");
    encoder
        .write_all(content.as_bytes())
        .expect("write zst contents");
    encoder.finish().expect("finish encoder");

    let cli = CliRunner::new().expect("cli runner");
    let input_path = path.to_string_lossy().into_owned();
    let res = cli.run(&["stats", "--input", &input_path]);
    assert_eq!(
        res.exit_code, 0,
        "stats should accept UTF-8 BOM compressed input, stderr: {}",
        res.stderr
    );
    assert!(
        res.stdout.contains("\"hands\": 2"),
        "stdout: {}",
        res.stdout
    );
}
