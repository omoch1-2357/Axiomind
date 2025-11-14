use axm_cli::run;

#[test]
fn level_progresses_over_session_and_blinds_printed() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(
        [
            "axm", "play", "--vs", "ai", "--hands", "16", "--level", "1", "--seed", "1",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Level: 1"));
    assert!(s.contains("Level: 2"));
    assert!(s.contains("Blinds:"));
}
