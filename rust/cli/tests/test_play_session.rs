use axiomind_cli::run;

#[test]
fn ai_session_counts_and_level_printed() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind", "play", "--vs", "ai", "--hands", "3", "--level", "2", "--seed", "9",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Level: 2"));
    assert!(stdout.contains("Session hands=3"));
}
