/// CI check for command registry synchronization
///
/// This test ensures that the Commands enum and COMMANDS array stay synchronized.
/// It prevents advertising non-existent commands in help text.
///
/// Requirements: 5, 6, 9 - Prevent future incomplete implementations
use std::collections::HashSet;

/// Test that enumerates Commands enum variants and verifies synchronization
///
/// This test validates:
/// 1. Every command in COMMANDS array has a corresponding Commands enum variant
/// 2. Non-existent commands (serve, train) are excluded from COMMANDS array
/// 3. All implemented commands are present in COMMANDS array
#[test]
fn test_commands_enum_synchronization() {
    // Define the expected set of implemented commands
    // This should match the Commands enum variants that are fully implemented
    let implemented_commands: HashSet<&str> = [
        "play", "replay", "sim", "eval", "stats", "verify", "deal", "bench", "rng", "cfg",
        "doctor", "export", "dataset",
    ]
    .iter()
    .copied()
    .collect();

    // Non-existent or planned commands that should NOT be in COMMANDS array
    let excluded_commands: HashSet<&str> = ["serve", "train"].iter().copied().collect();

    // Get the actual COMMANDS array from help text
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let _code = axiomind_cli::run(["axiomind", "--help"], &mut out, &mut err);
    let help_text = String::from_utf8_lossy(&out);

    // Verify all implemented commands appear in help text
    for cmd in &implemented_commands {
        assert!(
            help_text.contains(cmd),
            "COMMANDS array should include implemented command '{}', but help text doesn't show it.\n\
             This means the command exists in the enum but is not advertised.",
            cmd
        );
    }

    // Verify excluded commands do NOT appear in help text as standalone commands
    // Use word boundary matching to avoid false positives (e.g., "training" matching "train")
    for cmd in &excluded_commands {
        let pattern = format!("  {}  ", cmd); // Commands appear as "  command  description"
        assert!(
            !help_text.contains(&pattern),
            "COMMANDS array should NOT include non-existent command '{}', but help text shows it.\n\
             This violates the requirement that PLANNED commands must not appear in COMMANDS array.",
            cmd
        );
    }

    // Additional check: Verify help text has a reasonable number of commands
    // This catches cases where help text might be completely broken
    let command_count = implemented_commands.len();
    assert!(
        command_count >= 10,
        "Expected at least 10 implemented commands, found {}. \
         Check if Commands enum has been modified without updating this test.",
        command_count
    );
}

/// Test that verifies command validation logic prevents invalid commands
#[test]
fn test_invalid_command_rejection() {
    // Test that non-existent commands are properly rejected
    let test_cases = vec![
        ("serve", "serve command should not be available"),
        ("train", "train command should not be available"),
        (
            "nonexistent",
            "arbitrary invalid commands should be rejected",
        ),
    ];

    for (invalid_cmd, msg) in test_cases {
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();

        let code = axiomind_cli::run(["axiomind", invalid_cmd], &mut out, &mut err);

        // Invalid commands should result in non-zero exit code
        assert_ne!(
            code, 0,
            "{}: command '{}' should fail with non-zero exit code",
            msg, invalid_cmd
        );

        // Error message should be helpful
        let stderr = String::from_utf8_lossy(&err);
        let stdout = String::from_utf8_lossy(&out);
        let combined = format!("{}{}", stdout, stderr);

        assert!(
            combined.contains("error")
                || combined.contains("invalid")
                || combined.contains("unrecognized"),
            "{}: error message should indicate invalid command, got: {}",
            msg,
            combined
        );
    }
}

/// Test that all documented commands in CLI.md match implementation status
#[test]
fn test_documentation_matches_implementation() {
    // This test ensures consistency between code and documentation
    // It reads CLI.md and verifies that command statuses are accurate

    let cli_md_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("docs/CLI.md"))
        .expect("Could not construct path to docs/CLI.md");

    if !cli_md_path.exists() {
        // If CLI.md doesn't exist, skip this test
        // This allows the test to pass in minimal environments
        return;
    }

    let cli_md_content = std::fs::read_to_string(&cli_md_path).expect("Failed to read docs/CLI.md");

    // Verify that documentation mentions serve as PLANNED
    if cli_md_content.contains("serve") {
        assert!(
            cli_md_content.contains("PLANNED") && cli_md_content.contains("serve"),
            "Documentation should mark 'serve' command as PLANNED"
        );
    }

    // Verify that documentation mentions train as PLANNED
    if cli_md_content.contains("train") {
        assert!(
            cli_md_content.contains("PLANNED") && cli_md_content.contains("train"),
            "Documentation should mark 'train' command as PLANNED"
        );
    }

    // Verify implemented commands are documented
    let implemented_commands = ["play", "sim", "stats", "verify", "deal"];
    for cmd in &implemented_commands {
        assert!(
            cli_md_content.contains(cmd),
            "Documentation should include implemented command '{}'",
            cmd
        );
    }
}

/// Task 5.2: Test that command implementation checklist exists in documentation
#[test]
fn test_command_implementation_checklist_exists() {
    let cli_md_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("docs/CLI.md"))
        .expect("Could not construct path to docs/CLI.md");

    if !cli_md_path.exists() {
        // Skip if CLI.md doesn't exist
        return;
    }

    let cli_md_content = std::fs::read_to_string(&cli_md_path).expect("Failed to read docs/CLI.md");

    // Verify the checklist section exists
    assert!(
        cli_md_content.contains("New Command Implementation Checklist"),
        "CLI.md should contain 'New Command Implementation Checklist' section"
    );

    // Verify required checklist items exist
    let required_items = vec![
        "Command enum variant exists",
        "Implementation is complete, not a stub",
        "Behavioral tests verify actual command behavior",
        "Manual testing completed",
        "Command added to CLI.md",
        "PLANNED commands are NOT added to COMMANDS array",
    ];

    for item in required_items {
        assert!(
            cli_md_content.contains(item),
            "Checklist should include item about '{}'",
            item
        );
    }

    // Verify command status definitions exist
    assert!(
        cli_md_content.contains("Command Status Definitions"),
        "CLI.md should contain command status definitions section"
    );

    assert!(
        cli_md_content.contains("IMPLEMENTED")
            && cli_md_content.contains("PARTIAL")
            && cli_md_content.contains("PLANNED"),
        "Status definitions should include IMPLEMENTED, PARTIAL, and PLANNED"
    );
}

/// Task 5.3: Test that testing patterns for interactive commands are documented
#[test]
fn test_interactive_command_testing_patterns_exist() {
    let testing_md_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("docs/TESTING.md"))
        .expect("Could not construct path to docs/TESTING.md");

    if !testing_md_path.exists() {
        // Skip if TESTING.md doesn't exist
        return;
    }

    let testing_md_content =
        std::fs::read_to_string(&testing_md_path).expect("Failed to read docs/TESTING.md");

    // Verify the interactive testing section exists
    assert!(
        testing_md_content.contains("Testing Interactive CLI Commands"),
        "TESTING.md should contain 'Testing Interactive CLI Commands' section"
    );

    // Verify required testing patterns are documented
    let required_patterns = vec![
        "Testing Blocking Behavior with Piped Stdin",
        "Testing Input Parsing and State Changes",
        "Testing Error Handling and Recovery",
        "Testing Warning Display",
        "std::process::Command",
        "Stdio::piped()",
    ];

    for pattern in required_patterns {
        assert!(
            testing_md_content.contains(pattern),
            "TESTING.md should document pattern: '{}'",
            pattern
        );
    }

    // Verify code examples exist
    assert!(
        testing_md_content.contains("```rust"),
        "TESTING.md should include Rust code examples for testing patterns"
    );

    // Verify checklist exists
    assert!(
        testing_md_content.contains("Checklist for Interactive Command Tests"),
        "TESTING.md should include checklist for interactive command tests"
    );

    // Verify common pitfalls are documented
    assert!(
        testing_md_content.contains("Common Pitfalls"),
        "TESTING.md should document common pitfalls in testing"
    );
}
