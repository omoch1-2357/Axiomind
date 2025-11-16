use crate::helpers::cli_runner::CliRunner;

#[test]
fn e1_eval_requires_hands_and_models() {
    let cli = CliRunner::new().unwrap();
    // Missing --hands
    let res = cli.run(&["eval", "--ai-a", "baseline", "--ai-b", "baseline"]);
    assert_ne!(res.exit_code, 0);
    assert!(
        res.stderr.to_lowercase().contains("required"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn e2_eval_works_with_identical_models() {
    let cli = CliRunner::new().unwrap();
    let res = cli.run(&[
        "eval", "--ai-a", "baseline", "--ai-b", "baseline", "--hands", "4",
    ]);
    assert_eq!(res.exit_code, 0);
    assert!(
        res.stdout.contains("AI Comparison Results"),
        "stdout should contain comparison results: {}",
        res.stdout
    );
}

#[test]
fn e3_eval_is_deterministic_with_seed() {
    let cli = CliRunner::new().unwrap();
    let a = cli.run(&[
        "eval", "--ai-a", "baseline", "--ai-b", "baseline", "--hands", "8", "--seed", "42",
    ]);
    let b = cli.run(&[
        "eval", "--ai-a", "baseline", "--ai-b", "baseline", "--hands", "8", "--seed", "42",
    ]);
    assert_eq!(a.exit_code, 0);
    assert_eq!(b.exit_code, 0);
    assert_eq!(
        a.stdout, b.stdout,
        "same seed should produce identical results"
    );
}
