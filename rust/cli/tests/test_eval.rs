use axm_cli::run;

#[test]
fn eval_reports_summary_for_two_ais() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axm", "eval", "--ai-a", "random", "--ai-b", "random", "--hands", "10", "--seed", "2",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Eval: hands=10"));
    assert!(s.contains("A:"));
    assert!(s.contains("B:"));
}

#[test]
fn eval_displays_placeholder_warning() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axm", "eval", "--ai-a", "policy1", "--ai-b", "policy2", "--hands", "5", "--seed", "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("WARNING: This is a placeholder returning random results. AI parameters are not used. For real simulations, use 'axm sim' command."),
        "Expected placeholder warning in stderr, got: {}",
        stderr
    );
}

#[test]
fn eval_warns_about_unused_parameters() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axm", "eval", "--ai-a", "policy1", "--ai-b", "policy2", "--hands", "5", "--seed", "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("--ai-a") && stderr.contains("--ai-b"),
        "Expected warning about unused AI parameters in stderr, got: {}",
        stderr
    );
}

#[test]
fn eval_tags_output_with_random_results_warning() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axm", "eval", "--ai-a", "policy1", "--ai-b", "policy2", "--hands", "5", "--seed", "42",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        stdout.contains("[RANDOM RESULTS - NOT REAL AI COMPARISON]"),
        "Expected random results tag in stdout, got: {}",
        stdout
    );
}
