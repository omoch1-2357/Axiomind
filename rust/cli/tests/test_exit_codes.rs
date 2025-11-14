//! Tests for exit code standardization and error handling consistency
//!
//! This test suite verifies Requirement 10: CLI User Experience Consistency
//! - All successful operations return exit code 0 (including placeholder commands)
//! - File errors and validation errors return exit code 2
//! - Invalid user input triggers re-prompt, not program exit
//! - EOF on stdin results in graceful exit with code 0
//! - All errors are written to stderr, not stdout

/// Test that successful play command returns exit code 0
#[test]
fn test_play_ai_success_returns_zero() {
    let args = vec!["axm", "play", "--vs", "ai", "--hands", "1", "--seed", "42"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 0, "Successful play command should return exit code 0");
}

/// Test that successful play human with quit returns exit code 0
#[test]
fn test_play_human_quit_returns_zero() {
    let args = vec![
        "axm", "play", "--vs", "human", "--hands", "1", "--seed", "42",
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(
        code, 0,
        "Play command with graceful quit should return exit code 0"
    );
}

/// Test that EOF on stdin results in graceful exit with code 0
#[test]
fn test_play_human_eof_returns_zero() {
    // This test simulates EOF by providing no input
    // The execute_play_command should detect EOF and exit gracefully
    let args = vec![
        "axm", "play", "--vs", "human", "--hands", "1", "--seed", "42",
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(
        code, 0,
        "EOF on stdin should result in graceful exit with code 0"
    );
}

/// Test that invalid hands parameter returns exit code 2
#[test]
fn test_play_invalid_hands_returns_two() {
    let args = vec!["axm", "play", "--vs", "ai", "--hands", "0"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 2, "Invalid hands parameter should return exit code 2");
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("hands must be >= 1"),
        "Error message should be written to stderr"
    );
}

/// Test that placeholder commands return exit code 0
#[test]
fn test_eval_placeholder_returns_zero() {
    let args = vec![
        "axm", "eval", "--ai-a", "test", "--ai-b", "test", "--hands", "1",
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(
        code, 0,
        "Placeholder eval command should return exit code 0"
    );
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("WARNING"),
        "Placeholder warning should be written to stderr"
    );
}

/// Test that file read errors return exit code 2
#[test]
fn test_replay_nonexistent_file_returns_two() {
    let args = vec!["axm", "replay", "--input", "/nonexistent/file.jsonl"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 2, "File read error should return exit code 2");
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        !err_str.is_empty(),
        "Error message should be written to stderr"
    );
}

/// Test that validation errors return exit code 2
#[test]
fn test_dataset_invalid_splits_returns_two() {
    let args = vec![
        "axm",
        "dataset",
        "--input",
        "test.jsonl",
        "--outdir",
        "/tmp/test",
        "--train",
        "0.5",
        "--val",
        "0.3",
        "--test",
        "0.3", // Sum > 1.0
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 2, "Validation error should return exit code 2");
}

/// Test that errors are written to stderr, not stdout
#[test]
fn test_errors_written_to_stderr_not_stdout() {
    let args = vec!["axm", "play", "--vs", "ai", "--hands", "0"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 2);
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("hands must be >= 1"),
        "Error should be in stderr"
    );
    assert!(
        out.is_empty() || !String::from_utf8_lossy(&out).contains("hands must be >= 1"),
        "Error should not be in stdout"
    );
}

/// Test that all successful commands return 0
#[test]
fn test_deal_success_returns_zero() {
    let args = vec!["axm", "deal", "--seed", "42"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 0, "Successful deal command should return exit code 0");
}

/// Test that bench command returns 0
#[test]
fn test_bench_success_returns_zero() {
    let args = vec!["axm", "bench"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 0, "Bench command should return exit code 0");
}

/// Test that rng command returns 0
#[test]
fn test_rng_success_returns_zero() {
    let args = vec!["axm", "rng", "--seed", "42"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 0, "RNG command should return exit code 0");
}

/// Test that cfg command returns 0
#[test]
fn test_cfg_success_returns_zero() {
    let args = vec!["axm", "cfg"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 0, "Config command should return exit code 0");
}

/// Test that doctor command returns appropriate exit code based on checks
#[test]
fn test_doctor_returns_appropriate_code() {
    let args = vec!["axm", "doctor"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    // Doctor returns 0 if all checks pass, 2 if any fail
    assert!(
        code == 0 || code == 2,
        "Doctor should return 0 or 2, got {}",
        code
    );
}

/// Test that replay command with missing file returns exit code 2
#[test]
fn test_replay_missing_file_error_code() {
    let args = vec![
        "axm",
        "replay",
        "--input",
        "/definitely/does/not/exist.jsonl",
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 2, "Missing file should return exit code 2");
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("Failed to read"),
        "Error should mention failed read"
    );
}

/// Test that sim command with invalid hands returns exit code 2
#[test]
fn test_sim_invalid_hands_returns_two() {
    let args = vec!["axm", "sim", "--hands", "0", "--output", "test.jsonl"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(code, 2, "Invalid hands for sim should return exit code 2");
}

/// Test that export with missing input file returns exit code 2
#[test]
fn test_export_missing_input_returns_two() {
    let args = vec![
        "axm",
        "export",
        "--input",
        "/nonexistent/input.jsonl",
        "--format",
        "csv",
        "--output",
        "test.csv",
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(
        code, 2,
        "Export with missing input should return exit code 2"
    );
}

/// Test that stats with missing input returns exit code 2
#[test]
fn test_stats_missing_input_returns_two() {
    let args = vec!["axm", "stats", "--input", "/nonexistent/path"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(
        code, 2,
        "Stats with missing input should return exit code 2"
    );
}

/// Test that verify with missing input returns exit code 2
#[test]
fn test_verify_missing_input_returns_two() {
    let args = vec!["axm", "verify", "--input", "/nonexistent/file.jsonl"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = axm_cli::run(args, &mut out, &mut err);

    assert_eq!(
        code, 2,
        "Verify with missing input should return exit code 2"
    );
}

/// Test that all error messages go to stderr consistently
#[test]
fn test_all_errors_to_stderr() {
    let test_cases = vec![
        (
            vec!["axm", "play", "--vs", "ai", "--hands", "0"],
            "hands must be >= 1",
        ),
        (
            vec!["axm", "sim", "--hands", "0", "--output", "test.jsonl"],
            "hands must be >= 1",
        ),
        (
            vec!["axm", "replay", "--input", "/nonexistent.jsonl"],
            "Failed to read",
        ),
    ];

    for (args, expected_error) in test_cases {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let code = axm_cli::run(args.clone(), &mut out, &mut err);

        assert_eq!(
            code, 2,
            "Error case should return exit code 2 for {:?}",
            args
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains(expected_error),
            "Error message '{}' should be in stderr for {:?}",
            expected_error,
            args
        );

        let out_str = String::from_utf8_lossy(&out);
        assert!(
            !out_str.contains(expected_error),
            "Error message should NOT be in stdout for {:?}",
            args
        );
    }
}

/// Test exit code consistency: successful operations return 0
#[test]
fn test_successful_commands_return_zero() {
    let test_cases = vec![
        vec!["axm", "deal", "--seed", "42"],
        vec!["axm", "bench"],
        vec!["axm", "rng", "--seed", "42"],
        vec!["axm", "cfg"],
        vec!["axm", "play", "--vs", "ai", "--hands", "1", "--seed", "42"],
        vec![
            "axm", "eval", "--ai-a", "test", "--ai-b", "test", "--hands", "1",
        ],
    ];

    for args in test_cases {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let code = axm_cli::run(args.clone(), &mut out, &mut err);

        assert_eq!(code, 0, "Successful command should return 0 for {:?}", args);
    }
}
