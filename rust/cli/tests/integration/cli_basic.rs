// A-series: Basic CLI functionality tests
// Intentionally references helpers that don't exist yet (Red phase)

use crate::helpers::cli_runner::CliRunner;

#[test]
fn a1_help_lists_all_commands() {
    // GIVEN a CLI runner
    let cli = CliRunner::new().expect("CliRunner should initialize with binary path");

    // WHEN running `axm --help`
    let res = cli.run(&["--help"]);

    // THEN it should exit with 0 and include all commands in help text
    assert_eq!(res.exit_code, 0, "--help should exit with code 0");
    let out = res.stdout;
    for cmd in [
        "play", "replay", "sim", "eval", "stats", "verify", "deal", "bench", "rng", "cfg",
        "doctor", "export",
        "dataset",
        // Note: "serve" and "train" removed per Requirements 5 & 6 (not implemented)
    ] {
        assert!(out.contains(cmd), "help should list `{}`", cmd);
    }
}

#[test]
fn a2_version_prints_version_and_exits_zero() {
    let cli = CliRunner::new().expect("CliRunner should initialize");
    let res = cli.run(&["--version"]);
    assert_eq!(res.exit_code, 0, "--version should exit 0");
    assert!(
        !res.stdout.trim().is_empty(),
        "version should print some text"
    );
}

#[test]
fn a3_unknown_subcommand_shows_help_excerpt_on_stderr() {
    let cli = CliRunner::new().expect("CliRunner should initialize");
    let res = cli.run(&["unknown"]);
    assert_ne!(res.exit_code, 0, "unknown subcommand should be non-zero");
    // Expect help excerpt to include the Commands list (not just Usage)
    let err = res.stderr;
    assert!(
        err.contains("Commands:"),
        "stderr should contain help Commands section\n---stderr---\n{}\n-----------",
        err
    );
    assert!(
        err.contains("play"),
        "stderr help excerpt should list 'play'"
    );
}
