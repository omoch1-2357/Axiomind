//! Behavioral tests for interactive commands
//!
//! These tests verify actual command behavior using real stdin/stdout,
//! not environment variable bypasses. They test blocking behavior,
//! input parsing, and state changes.

mod helpers;

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Helper to find the axm binary
fn find_axm_binary() -> std::path::PathBuf {
    if let Ok(explicit) = std::env::var("CARGO_BIN_EXE_axm") {
        return std::path::PathBuf::from(explicit);
    }

    // Get the absolute path to the workspace root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root");

    let executable = if cfg!(windows) { "axm.exe" } else { "axm" };
    let mut search_roots = vec![workspace_root.join("target")];

    if let Ok(custom_target) = std::env::var("CARGO_TARGET_DIR") {
        search_roots.insert(0, std::path::PathBuf::from(custom_target));
    }

    for root in &search_roots {
        for profile in ["debug", "release"] {
            let candidate = root.join(profile).join(executable);
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    search_roots[0].join("debug").join(executable)
}

/// Test 4.1: Test human play stdin blocking behavior
/// Verifies that the play command blocks waiting for stdin input
#[test]
fn test_play_human_blocks_waiting_for_stdin() {
    let binary = find_axm_binary();
    eprintln!("Using binary at: {}", binary.display());
    assert!(
        binary.exists(),
        "Binary does not exist at: {}",
        binary.display()
    );

    // Spawn the command with piped stdin, but don't write anything
    // This should cause the command to block waiting for input
    let mut child = Command::new(&binary)
        .args(["play", "--vs", "human", "--hands", "1", "--seed", "42"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn axm command");

    // Keep stdin open but don't write to it
    // The command should block waiting for input

    let start = Instant::now();
    let timeout = Duration::from_millis(500);

    // Poll the child process
    loop {
        if let Ok(Some(_status)) = child.try_wait() {
            // Command completed unexpectedly
            let output = child.wait_with_output().expect("failed to read output");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!(
                "Expected command to block waiting for stdin, but it completed. \
                 stdout: {}\nstderr: {}",
                stdout, stderr
            );
        }

        if start.elapsed() >= timeout {
            // Command is still running after timeout - this is what we expect!
            // Kill the child and verify it was indeed blocking
            let _ = child.kill();
            let _ = child.wait();

            // Success - command was blocking as expected
            assert!(
                start.elapsed() >= Duration::from_millis(450),
                "Command should have been running for at least 450ms"
            );
            return;
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Test 4.1: Verify process continues running after input is provided
#[test]
fn test_play_human_accepts_stdin_and_progresses() {
    let binary = find_axm_binary();

    // Spawn the command with piped stdin
    let mut child = Command::new(&binary)
        .args(["play", "--vs", "human", "--hands", "1", "--seed", "42"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn axm command");

    // Send quit command immediately so the game terminates after starting
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"q\n").expect("Failed to write to stdin");
    }

    // Wait for command to complete
    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Command should complete successfully when stdin is provided
    assert_eq!(
        output.status.code().unwrap_or(1),
        0,
        "Expected exit code 0 when stdin provided, got {}. \
         stdout: {}\nstderr: {}",
        output.status.code().unwrap_or(1),
        stdout,
        stderr
    );

    // Should display game state before accepting quit
    assert!(
        stdout.contains("Hand 1"),
        "Expected game state display in output, got: {}",
        stdout
    );
}

// Task 4.2: Test input parsing and error handling

/// Test 4.2: Test valid action parsing via actual stdin
#[test]
fn test_play_human_parses_valid_actions() {
    let binary = find_axm_binary();

    // Test various valid actions
    let test_cases = vec![
        ("fold\n", "Action: fold"),
        ("check\n", "Action: check"),
        ("call\n", "Action: call"),
        ("bet 100\n", "Action: bet 100"),
        ("raise 50\n", "Action: raise 50"),
    ];

    for (input, expected_output) in test_cases {
        let mut child = Command::new(&binary)
            .args(["play", "--vs", "human", "--hands", "1", "--seed", "42"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn axm command");

        // Send the action followed by quit
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .expect("Failed to write input");
            stdin.write_all(b"q\n").expect("Failed to write quit");
        }

        let output = child
            .wait_with_output()
            .expect("Failed to wait for command");
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            stdout.contains(expected_output),
            "Expected '{}' in output for input '{}', got: {}",
            expected_output,
            input.trim(),
            stdout
        );
    }
}

/// Test 4.2: Test quit commands result in graceful exit
#[test]
fn test_play_human_quit_commands() {
    let binary = find_axm_binary();

    let quit_commands = vec!["q\n", "quit\n", "Q\n", "QUIT\n"];

    for quit_cmd in quit_commands {
        let mut child = Command::new(&binary)
            .args(["play", "--vs", "human", "--hands", "1", "--seed", "42"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn axm command");

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(quit_cmd.as_bytes())
                .expect("Failed to write quit command");
        }

        let output = child
            .wait_with_output()
            .expect("Failed to wait for command");
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert_eq!(
            output.status.code().unwrap_or(1),
            0,
            "Expected successful exit for quit command '{}', stdout: {}",
            quit_cmd.trim(),
            stdout
        );

        assert!(
            stdout.contains("completed"),
            "Expected 'completed' message for quit command '{}', got: {}",
            quit_cmd.trim(),
            stdout
        );
    }
}

/// Test 4.2: Test invalid input triggers error and re-prompt
#[test]
fn test_play_human_handles_invalid_input() {
    let binary = find_axm_binary();

    let mut child = Command::new(&binary)
        .args(["play", "--vs", "human", "--hands", "1", "--seed", "42"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn axm command");

    // Send invalid input, then a valid quit command
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"invalid_action\n")
            .expect("Failed to write invalid input");
        stdin.write_all(b"q\n").expect("Failed to write quit");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show error for invalid action
    assert!(
        stderr.contains("Invalid") || stderr.contains("Unrecognized"),
        "Expected error message for invalid input in stderr, got: {}",
        stderr
    );

    // Should still complete successfully after recovery
    assert_eq!(
        output.status.code().unwrap_or(1),
        0,
        "Expected successful exit after recovering from invalid input, stderr: {}, stdout: {}",
        stderr,
        stdout
    );
}

/// Test 4.2: Test empty input is handled gracefully
#[test]
fn test_play_human_handles_empty_input() {
    let binary = find_axm_binary();

    let mut child = Command::new(&binary)
        .args(["play", "--vs", "human", "--hands", "1", "--seed", "42"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn axm command");

    // Send empty lines followed by valid quit
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"\n\n")
            .expect("Failed to write empty lines");
        stdin.write_all(b"q\n").expect("Failed to write quit");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // Should complete successfully without crashing
    assert_eq!(
        output.status.code().unwrap_or(1),
        0,
        "Expected successful exit when handling empty input, stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// Task 4.5: Comprehensive edge case coverage (optional)

/// Test 4.5: Test multiple consecutive invalid inputs don't crash
#[test]
fn test_play_human_handles_multiple_invalid_inputs() {
    let binary = find_axm_binary();

    let mut child = Command::new(&binary)
        .args(["play", "--vs", "human", "--hands", "1", "--seed", "42"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn axm command");

    // Send multiple invalid inputs followed by valid quit
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"invalid1\n").expect("Failed to write");
        stdin.write_all(b"invalid2\n").expect("Failed to write");
        stdin.write_all(b"badcommand\n").expect("Failed to write");
        stdin.write_all(b"q\n").expect("Failed to write quit");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // Should complete successfully after multiple invalid inputs
    assert_eq!(
        output.status.code().unwrap_or(1),
        0,
        "Expected successful exit after multiple invalid inputs, stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test 4.5: Test negative bet amounts are rejected
#[test]
fn test_play_human_rejects_negative_bet() {
    let binary = find_axm_binary();

    let mut child = Command::new(&binary)
        .args(["play", "--vs", "human", "--hands", "1", "--seed", "42"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn axm command");

    // Send negative bet followed by quit
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"bet -100\n").expect("Failed to write");
        stdin.write_all(b"q\n").expect("Failed to write quit");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show error for negative amount
    assert!(
        stderr.contains("positive") || stderr.contains("invalid") || stderr.contains("Invalid"),
        "Expected error message for negative bet amount in stderr, got: {}",
        stderr
    );

    // Should still complete successfully
    assert_eq!(output.status.code().unwrap_or(1), 0);
}

/// Test 4.5: Test mid-hand quit displays session statistics
#[test]
fn test_play_human_mid_hand_quit_shows_stats() {
    let binary = find_axm_binary();

    let mut child = Command::new(&binary)
        .args(["play", "--vs", "human", "--hands", "3", "--seed", "42"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn axm command");

    // Quit immediately on first hand
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"q\n").expect("Failed to write quit");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should display session statistics even when quitting early
    assert!(
        stdout.contains("Session") || stdout.contains("Hands played"),
        "Expected session statistics in output, got: {}",
        stdout
    );

    assert!(
        stdout.contains("completed"),
        "Expected 'completed' status, got: {}",
        stdout
    );
}
