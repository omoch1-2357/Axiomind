/// Test suite to verify COMMANDS array is synchronized with Commands enum
///
/// This test ensures that the COMMANDS constant (used for help text) only includes
/// commands that actually exist in the Commands enum, preventing false advertising
/// of non-existent features.
///
/// Related to Requirements 5 & 6: Remove non-existent "serve" and "train" commands
use axm_cli::run;

#[test]
fn commands_array_excludes_serve() {
    // Test that "serve" command is NOT advertised in help text
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();

    let _code = run(["axm", "--help"], &mut out, &mut err);
    let stdout = String::from_utf8_lossy(&out);

    // "serve" should NOT appear in the command list
    assert!(
        !stdout.contains("serve"),
        "help text should NOT list 'serve' command (not implemented)"
    );
}

#[test]
fn commands_array_excludes_train() {
    // Test that "train" command is NOT advertised in help text
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();

    let _code = run(["axm", "--help"], &mut out, &mut err);
    let stdout = String::from_utf8_lossy(&out);

    // "train" should NOT appear in the command list
    assert!(
        !stdout.contains("train"),
        "help text should NOT list 'train' command (not implemented)"
    );
}

#[test]
fn commands_array_includes_only_implemented_commands() {
    // Test that all commands in help text correspond to actual Commands enum variants
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();

    let _code = run(["axm", "--help"], &mut out, &mut err);
    let stdout = String::from_utf8_lossy(&out);

    // All implemented commands should be present
    let implemented_commands = [
        "play", "replay", "stats", "verify", "deal", "bench", "sim", "eval", "export", "dataset",
        "cfg", "doctor", "rng",
    ];

    for cmd in &implemented_commands {
        assert!(
            stdout.contains(cmd),
            "help should list implemented command '{}'",
            cmd
        );
    }

    // Non-existent commands should NOT be present
    let non_existent_commands = ["serve", "train"];

    for cmd in &non_existent_commands {
        assert!(
            !stdout.contains(cmd),
            "help should NOT list non-existent command '{}'",
            cmd
        );
    }
}
