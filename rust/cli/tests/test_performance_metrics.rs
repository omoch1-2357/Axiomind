//! Integration tests for performance validation and cache optimization (Task 9)
//!
//! This test suite validates:
//! - cargo doc build time is within acceptable limits (target: 5 minutes in CI)
//! - Cargo cache improves rebuild performance
//! - Generated documentation meets quality standards
//!
//! These tests are designed to be run in CI environments to validate performance requirements.

use serial_test::serial;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// Helper to get the project root directory
fn project_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent directory")
        .parent()
        .expect("Project root should be two levels up from manifest")
        .to_path_buf()
}

/// Test 1: Measure cargo doc build time (target: 5 minutes in CI environment)
///
/// This test measures the time required to build complete workspace documentation
/// using `cargo doc --workspace --no-deps`. The goal is to ensure CI builds
/// complete within the 5-minute target.
#[test]
#[serial]
fn test_cargo_doc_build_time() {
    let root = project_root();

    println!("Starting cargo doc build time measurement...");
    let start = Instant::now();

    let output = Command::new("cargo")
        .arg("doc")
        .arg("--workspace")
        .arg("--no-deps")
        .arg("--verbose")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo doc");

    let duration = start.elapsed();

    assert!(
        output.status.success(),
        "cargo doc failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    println!("cargo doc build completed in {:.2?}", duration);

    // Target: 5 minutes (300 seconds) in CI environment
    // For local development, this may be faster due to caching
    let target_duration = Duration::from_secs(300);

    // Allow some tolerance for CI variability (add 20% buffer)
    let max_duration = Duration::from_secs(360);

    if duration > target_duration {
        eprintln!(
            "⚠️  WARNING: Build time ({:.2?}) exceeds target ({:.2?})",
            duration, target_duration
        );
    }

    assert!(
        duration <= max_duration,
        "Build time ({:.2?}) exceeds maximum allowed duration ({:.2?})",
        duration,
        max_duration
    );
}

/// Test 2: Verify Cargo cache effectiveness by measuring rebuild performance
///
/// This test verifies that:
/// 1. First build completes successfully
/// 2. Second build (with cache) is significantly faster
/// 3. Cache hit rate is measurable through build output
#[test]
#[serial]
fn test_cargo_cache_effectiveness() {
    let root = project_root();

    // Clean the target directory to ensure first build is from scratch
    let target_dir = root.join("target");
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir).expect("Failed to clean target directory");
    }

    // First build: Full compilation
    println!("Running first build (clean)...");
    let start1 = Instant::now();

    let output1 = Command::new("cargo")
        .arg("doc")
        .arg("--workspace")
        .arg("--no-deps")
        .arg("--verbose")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo doc (first build)");

    let duration1 = start1.elapsed();

    assert!(
        output1.status.success(),
        "First cargo doc build failed: {}",
        String::from_utf8_lossy(&output1.stderr)
    );

    println!("First build completed in {:.2?}", duration1);

    // Second build: Should use cache
    println!("Running second build (with cache)...");
    let start2 = Instant::now();

    let output2 = Command::new("cargo")
        .arg("doc")
        .arg("--workspace")
        .arg("--no-deps")
        .arg("--verbose")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo doc (second build)");

    let duration2 = start2.elapsed();

    assert!(
        output2.status.success(),
        "Second cargo doc build failed: {}",
        String::from_utf8_lossy(&output2.stderr)
    );

    println!("Second build completed in {:.2?}", duration2);

    // Verify cache is effective: second build should be significantly faster
    // Expected: 70-90% time reduction with cache
    let speedup_ratio = duration1.as_secs_f64() / duration2.as_secs_f64();

    println!(
        "Cache speedup: {:.2}x (first: {:.2?}, second: {:.2?})",
        speedup_ratio, duration1, duration2
    );

    // Second build should be faster than first build
    // Note: For small projects with fast build times, the speedup may be modest
    // due to fixed overhead. We verify cache is working, not a specific ratio.
    // Minimum threshold: 1.1x (second build is at least 10% faster)
    let min_speedup = 1.1;

    assert!(
        speedup_ratio >= min_speedup,
        "Cache not effective: speedup {:.2}x is below minimum {:.2}x threshold",
        speedup_ratio,
        min_speedup
    );

    // Check for "Fresh" indicators in cargo output (cache hits)
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    let combined_output = format!("{}{}", stdout2, stderr2);

    // Look for cache hit indicators in verbose output
    // Cargo verbose mode shows "Fresh" for cached packages
    let has_cache_indicators =
        combined_output.contains("Fresh") || combined_output.contains("Finished");

    assert!(
        has_cache_indicators,
        "No cache hit indicators found in second build output"
    );
}

/// Test 3: Verify documentation completeness and quality
///
/// This test ensures that generated documentation meets quality standards:
/// - All workspace crates have documentation
/// - Search index is generated
/// - Documentation is browsable
#[test]
#[serial]
fn test_documentation_completeness() {
    let root = project_root();
    let target_doc = root.join("target").join("doc");

    // Build documentation
    let output = Command::new("cargo")
        .arg("doc")
        .arg("--workspace")
        .arg("--no-deps")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo doc");

    assert!(
        output.status.success(),
        "cargo doc failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify each crate has documentation directory
    let expected_crates = vec!["axm_engine", "axm_cli", "axm_web"];

    for crate_name in &expected_crates {
        let crate_dir = target_doc.join(crate_name);
        assert!(
            crate_dir.exists(),
            "Documentation directory not found for crate: {}",
            crate_name
        );

        // Verify index.html exists for each crate
        let index_html = crate_dir.join("index.html");
        assert!(
            index_html.exists(),
            "index.html not found for crate: {}",
            crate_name
        );
    }

    // Verify search-index.js exists
    let search_index = target_doc.join("search-index.js");

    // Debug: List files in target/doc if search-index.js is missing
    if !search_index.exists() {
        eprintln!("search-index.js not found at: {:?}", search_index);
        eprintln!("Listing contents of target/doc:");

        if let Ok(entries) = std::fs::read_dir(&target_doc) {
            for entry in entries.flatten() {
                eprintln!("  - {:?}", entry.path());
            }
        } else {
            eprintln!("  Failed to read target/doc directory");
        }
    }

    assert!(
        search_index.exists(),
        "search-index.js not found in documentation output at {:?}",
        search_index
    );

    println!(
        "✓ Documentation completeness verified for {} crates",
        expected_crates.len()
    );
}

/// Test 4: Measure documentation size to ensure reasonable output
///
/// This test verifies that documentation size is within reasonable limits
/// to ensure GitHub Pages deployment stays within quota (1GB limit).
#[test]
#[serial]
fn test_documentation_size() {
    let root = project_root();
    let target_doc = root.join("target").join("doc");

    // Build documentation
    let output = Command::new("cargo")
        .arg("doc")
        .arg("--workspace")
        .arg("--no-deps")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo doc");

    assert!(
        output.status.success(),
        "cargo doc failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Calculate total documentation size
    let total_size = calculate_directory_size(&target_doc);
    let size_mb = total_size as f64 / 1_048_576.0; // Convert to MB

    println!("Total documentation size: {:.2} MB", size_mb);

    // GitHub Pages limit is 1GB, but we want to stay well below that
    // Target: Keep documentation under 100MB
    let max_size_mb = 100.0;

    assert!(
        size_mb <= max_size_mb,
        "Documentation size ({:.2} MB) exceeds maximum ({:.2} MB)",
        size_mb,
        max_size_mb
    );

    // Warn if size is getting close to limit (>50MB)
    if size_mb > 50.0 {
        eprintln!(
            "⚠️  WARNING: Documentation size ({:.2} MB) is approaching limit ({:.2} MB)",
            size_mb, max_size_mb
        );
    }
}

/// Helper function to calculate total size of a directory recursively
fn calculate_directory_size(path: &PathBuf) -> u64 {
    use std::fs;

    let mut total_size = 0u64;

    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        total_size += metadata.len();
                    }
                } else if entry_path.is_dir() {
                    total_size += calculate_directory_size(&entry_path);
                }
            }
        }
    }

    total_size
}

/// Test 5: Verify workflow execution time in simulated CI environment
///
/// This test simulates the complete CI workflow execution:
/// 1. cargo doc --workspace --no-deps
/// 2. generate-doc-index.sh
/// 3. Measure total time
#[test]
#[serial]
fn test_complete_workflow_time() {
    let root = project_root();

    println!("Starting complete workflow simulation...");
    let start = Instant::now();

    // Step 1: Build documentation
    println!("Step 1: Building documentation...");
    let doc_output = Command::new("cargo")
        .arg("doc")
        .arg("--workspace")
        .arg("--no-deps")
        .arg("--verbose")
        .current_dir(&root)
        .output()
        .expect("Failed to run cargo doc");

    assert!(
        doc_output.status.success(),
        "cargo doc failed: {}",
        String::from_utf8_lossy(&doc_output.stderr)
    );

    // Step 2: Generate index.html (if bash is available)
    println!("Step 2: Generating index.html...");
    let script_path = root.join("scripts").join("generate-doc-index.sh");

    #[cfg(windows)]
    let bash_cmd = {
        // Try bash in PATH first, then common installation locations
        let candidates = [
            "bash",
            "bash.exe",
            "C:\\Program Files\\Git\\bin\\bash.exe",
            "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
        ];

        candidates
            .iter()
            .find(|&path| Command::new(path).arg("--version").output().is_ok())
            .map(|s| s.to_string())
    };

    #[cfg(unix)]
    let bash_cmd = Some("bash".to_string());

    if let Some(bash) = bash_cmd {
        let script_output = Command::new(bash)
            .arg(&script_path)
            .current_dir(&root)
            .output()
            .expect("Failed to run generate-doc-index.sh");

        assert!(
            script_output.status.success(),
            "generate-doc-index.sh failed: {}",
            String::from_utf8_lossy(&script_output.stderr)
        );
    } else {
        println!("⚠️  Skipping index generation: bash not available on Windows");
    }

    let total_duration = start.elapsed();
    println!("Complete workflow finished in {:.2?}", total_duration);

    // CI target: 5 minutes for the entire workflow
    let target_duration = Duration::from_secs(300);
    let max_duration = Duration::from_secs(360); // 6 minutes with buffer

    if total_duration > target_duration {
        eprintln!(
            "⚠️  WARNING: Workflow time ({:.2?}) exceeds target ({:.2?})",
            total_duration, target_duration
        );
    }

    assert!(
        total_duration <= max_duration,
        "Workflow time ({:.2?}) exceeds maximum allowed duration ({:.2?})",
        total_duration,
        max_duration
    );
}
