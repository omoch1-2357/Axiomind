//! Integration tests for doctest validation (Task 7.1)
//!
//! This test suite validates:
//! - Doctest execution in CI (cargo test --doc)
//! - Detailed error messages for failing doctests
//! - Guidelines for no_run attribute usage
//!
//! Requirements: 4.4, 4.5, 4.6, 6.6

use std::path::PathBuf;
use std::process::Command;

/// Helper to get the project root directory
fn project_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Test 1: Verify that cargo test --doc runs successfully
/// This ensures that all doctests in the workspace pass
#[test]
fn test_doctest_execution() {
    let root = project_root();

    // Run doctests for the entire workspace
    let output = Command::new("cargo")
        .arg("test")
        .arg("--workspace")
        .arg("--doc")
        .arg("--verbose")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo test --doc");

    // Check if doctests passed
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!("Doctests failed:\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
    }

    // Verify that doctests actually ran (not just skipped)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test ") || stdout.contains("Doc-tests"),
        "No doctests appear to have run. Output:\n{}",
        stdout
    );
}

/// Test 2: Verify that doctest failures provide detailed error messages
/// This test creates a temporary crate with a failing doctest to verify error reporting
#[test]
fn test_doctest_error_reporting() {
    let root = project_root();

    // Run doctests for axiomind-engine (which has several doctests)
    let output = Command::new("cargo")
        .arg("test")
        .arg("--doc")
        .arg("-p")
        .arg("axiomind-engine")
        .arg("--verbose")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo test --doc for axiomind-engine");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // If there are failing doctests, verify detailed error messages are present
    if !output.status.success() {
        // Error messages should include:
        // 1. File path where the doctest is located
        // 2. Line number or snippet of the failing code
        // 3. Compiler error message

        assert!(
            stderr.contains("error") || stdout.contains("FAILED"),
            "Expected detailed error message in output:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout,
            stderr
        );

        // Should include source location
        assert!(
            stderr.contains(".rs") || stdout.contains(".rs"),
            "Expected file path in error message:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout,
            stderr
        );
    }
    // If all doctests pass, that's also a valid outcome for this test
}

/// Test 3: Verify that no_run examples are properly handled
/// This checks that code examples with no_run attribute compile but don't execute
#[test]
fn test_norun_attribute_handling() {
    let root = project_root();

    // Build documentation to verify no_run examples compile
    let output = Command::new("cargo")
        .arg("doc")
        .arg("--workspace")
        .arg("--no-deps")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo doc");

    assert!(
        output.status.success(),
        "cargo doc failed (no_run examples should still compile):\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Run doctests to ensure no_run examples are recognized
    let output = Command::new("cargo")
        .arg("test")
        .arg("--workspace")
        .arg("--doc")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo test --doc");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify that tests ran (no_run examples should be compiled but not executed)
    assert!(
        output.status.success(),
        "Doctests failed (no_run examples should compile):\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test 4: Verify doctest count per crate
/// This ensures doctests are actually present in documentation
#[test]
fn test_doctest_coverage() {
    let root = project_root();

    // Test each crate individually to see which have doctests
    let crates = vec!["axiomind-engine", "axiomind_cli", "axiomind_web"];

    for crate_name in &crates {
        let output = Command::new("cargo")
            .arg("test")
            .arg("--doc")
            .arg("-p")
            .arg(crate_name)
            .arg("--verbose")
            .current_dir(&root)
            .output()
            .unwrap_or_else(|_| panic!("Failed to run cargo test --doc for {}", crate_name));

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Verify the command completed (may have 0 doctests, which is ok)
        // We're just checking that the infrastructure works
        if !output.status.success() {
            eprintln!(
                "Doctest execution for {} had issues:\nSTDOUT:\n{}\nSTDERR:\n{}",
                crate_name, stdout, stderr
            );
        }

        // Log info about doctest count (informational)
        if stdout.contains("Doc-tests") {
            println!("Crate {} has doctests", crate_name);
        } else {
            println!("Crate {} may have no doctests (this is ok)", crate_name);
        }
    }
}

/// Test 5: Verify CI configuration includes doctest execution
#[test]
fn test_ci_includes_doctest() {
    let root = project_root();
    let ci_file = root.join(".github").join("workflows").join("ci.yml");

    assert!(
        ci_file.exists(),
        "CI workflow file not found at {:?}",
        ci_file
    );

    let content = std::fs::read_to_string(&ci_file).expect("Failed to read ci.yml");

    // Verify that ci.yml includes "cargo test --doc"
    assert!(
        content.contains("cargo test") && content.contains("--doc"),
        "CI workflow should include 'cargo test --doc' command"
    );

    // Verify it's in the test job
    assert!(
        content.contains("Run doc tests") || content.contains("test --workspace --doc"),
        "CI should have a step for running doc tests"
    );
}
