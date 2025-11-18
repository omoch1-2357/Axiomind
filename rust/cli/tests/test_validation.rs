use axiomind_cli::run;
use std::fs;
use std::path::PathBuf;

fn tmp_file(name: &str) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    let _ = fs::create_dir_all(p.parent().unwrap());
    p
}

#[test]
fn stats_invalid_json_line_errors() {
    let path = tmp_file("invalid_json");
    fs::write(&path, "not json\n").unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
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
    assert_ne!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("Invalid record"));
}

#[test]
fn sim_hands_zero_invalid() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(["axiomind", "sim", "--hands", "0"], &mut out, &mut err);
    assert_ne!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("hands must be >= 1"));
}

#[test]
fn play_hands_zero_invalid() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(
        ["axiomind", "play", "--vs", "ai", "--hands", "0"],
        &mut out,
        &mut err,
    );
    assert_ne!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("hands must be >= 1"));
}

#[test]
fn verify_requires_input() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(["axiomind", "verify"], &mut out, &mut err);
    assert_ne!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("input required"));
}

#[test]
fn verify_invalid_hand_id() {
    let path = tmp_file("bad_id");
    fs::write(
        &path,
        "{\"hand_id\":\"BAD\",\"seed\":1,\"actions\":[],\"board\":[],\"result\":null}\n",
    )
    .unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(
        [
            "axiomind",
            "verify",
            "--input",
            path.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_ne!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.to_lowercase().contains("hand_id"));
}
