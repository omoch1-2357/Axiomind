use axiomind_cli::run;

#[test]
fn eval_reports_comparison_results() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind", "eval", "--ai-a", "baseline", "--ai-b", "baseline", "--hands", "10",
            "--seed", "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(
        s.contains("AI Comparison Results"),
        "Expected 'AI Comparison Results' header"
    );
    assert!(
        s.contains("Hands played: 10"),
        "Expected hands played count"
    );
    assert!(s.contains("Seed: 42"), "Expected seed in output");
    assert!(s.contains("AI-A (baseline):"), "Expected AI-A section");
    assert!(s.contains("AI-B (baseline):"), "Expected AI-B section");
    assert!(s.contains("Wins:"), "Expected win statistics");
    assert!(s.contains("Losses:"), "Expected loss statistics");
    assert!(s.contains("Avg chip delta:"), "Expected chip delta");
    assert!(s.contains("Actions:"), "Expected action statistics");
}

#[test]
fn eval_is_deterministic_with_same_seed() {
    // Run eval twice with the same seed
    let mut out1: Vec<u8> = Vec::new();
    let mut err1: Vec<u8> = Vec::new();
    let code1 = run(
        [
            "axiomind", "eval", "--ai-a", "baseline", "--ai-b", "baseline", "--hands", "5",
            "--seed", "100",
        ],
        &mut out1,
        &mut err1,
    );
    assert_eq!(code1, 0);

    let mut out2: Vec<u8> = Vec::new();
    let mut err2: Vec<u8> = Vec::new();
    let code2 = run(
        [
            "axiomind", "eval", "--ai-a", "baseline", "--ai-b", "baseline", "--hands", "5",
            "--seed", "100",
        ],
        &mut out2,
        &mut err2,
    );
    assert_eq!(code2, 0);

    // Results should be identical
    let s1 = String::from_utf8_lossy(&out1);
    let s2 = String::from_utf8_lossy(&out2);
    assert_eq!(s1, s2, "Same seed should produce identical results");
}

#[test]
fn eval_handles_unknown_ai_type_for_ai_a() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind",
            "eval",
            "--ai-a",
            "unknown_ai",
            "--ai-b",
            "baseline",
            "--hands",
            "5",
            "--seed",
            "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 2, "Should return error code 2 for unknown AI");
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("Unknown AI type: unknown_ai"),
        "Expected error message for unknown AI, got: {}",
        stderr
    );
}

#[test]
fn eval_handles_unknown_ai_type_for_ai_b() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind",
            "eval",
            "--ai-a",
            "baseline",
            "--ai-b",
            "unknown_ai",
            "--hands",
            "5",
            "--seed",
            "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 2, "Should return error code 2 for unknown AI");
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("Unknown AI type: unknown_ai"),
        "Expected error message for unknown AI, got: {}",
        stderr
    );
}

#[test]
fn eval_tracks_action_statistics() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind", "eval", "--ai-a", "baseline", "--ai-b", "baseline", "--hands", "20",
            "--seed", "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);

    // Check that action percentages are present
    assert!(s.contains("Fold"), "Expected Fold action statistics");
    assert!(s.contains("Check"), "Expected Check action statistics");
    assert!(s.contains("Call"), "Expected Call action statistics");
    assert!(s.contains("Bet"), "Expected Bet action statistics");
    assert!(s.contains("Raise"), "Expected Raise action statistics");

    // Verify format includes percentages
    assert!(
        s.contains("%"),
        "Expected percentage symbols in action stats"
    );
}
