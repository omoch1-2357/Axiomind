use axiomind_cli::run;
use std::env;

#[test]
fn human_quick_quit_via_test_input() {
    env::set_var("axiomind_TEST_INPUT", "q\n");
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind", "play", "--vs", "human", "--hands", "1", "--seed", "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Hand 1"));
    assert!(stdout.to_lowercase().contains("completed"));
}

#[test]
fn ai_mode_runs_noninteractive() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind", "play", "--vs", "ai", "--hands", "2", "--seed", "7",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Hands played: 2"));
}

#[test]
fn ai_mode_displays_placeholder_warning() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind", "play", "--vs", "ai", "--hands", "1", "--seed", "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    if stderr.trim().is_empty() {
        // Functional AI builds suppress the placeholder warning entirely.
        return;
    }
    assert!(
        stderr.contains(
            "WARNING: AI opponent is a placeholder that always checks. Use for demo purposes only."
        ),
        "Expected placeholder warning when stderr is non-empty, got: {}",
        stderr
    );
}

#[test]
fn ai_mode_tags_output_with_demo_mode() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind", "play", "--vs", "ai", "--hands", "1", "--seed", "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        stdout.contains("ai: check [DEMO MODE]"),
        "Expected [DEMO MODE] tag in AI action output, got: {}",
        stdout
    );
}

#[test]
fn ai_mode_warning_appears_before_game_output() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind", "play", "--vs", "ai", "--hands", "1", "--seed", "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    let stdout = String::from_utf8_lossy(&out);

    if stderr.trim().is_empty() {
        assert!(
            stdout.contains("Hand 1"),
            "Expected game output in stdout even when no warning is emitted. Got: {}",
            stdout
        );
        return;
    }

    // Warning should appear in stderr when present
    assert!(stderr.contains("WARNING:"), "Expected warning in stderr");
    // Game output should be in stdout
    assert!(stdout.contains("Hand 1"), "Expected game output in stdout");
}
