//! Bench command handler for hand evaluation performance benchmarking.
//!
//! This module provides the `bench` command which performs a quick benchmark
//! of the hand evaluation system by evaluating 200 unique 7-card hands from
//! a shuffled deck and reporting the execution time.

use crate::error::CliError;
use axiomind_engine::cards::Card;
use axiomind_engine::deck::Deck;
use std::io::Write;

/// Handle the bench command.
///
/// Performs a quick benchmark by evaluating 200 unique 7-card hands from
/// a shuffled deck using deterministic seed 1 for reproducibility. Reports
/// the number of iterations and total execution time.
///
/// # Arguments
///
/// * `out` - Output stream for benchmark results
///
/// # Returns
///
/// Returns `Ok(())` on success, or `CliError` on I/O errors.
///
/// # Examples
///
/// ```ignore
/// // Internal command handler - not part of public API
/// use axiomind_cli::commands::bench::handle_bench_command;
/// let mut out = Vec::new();
/// handle_bench_command(&mut out).unwrap();
/// ```
pub fn handle_bench_command(out: &mut dyn Write) -> Result<(), CliError> {
    // quick bench: evaluate 200 unique 7-card draws from shuffled deck
    let start = std::time::Instant::now();
    let mut cnt = 0u64;
    let mut deck = Deck::new_with_seed(1);
    deck.shuffle();
    for _ in 0..200 {
        if deck.remaining() < 7 {
            deck.shuffle();
        }
        let mut arr: [Card; 7] = [deck.deal_card().unwrap(); 7];
        for item in arr.iter_mut().skip(1) {
            *item = deck.deal_card().unwrap();
        }
        // Result intentionally unused - benchmark only measures performance
        let _ = axiomind_engine::hand::evaluate_hand(&arr);
        cnt += 1;
    }
    let dur = start.elapsed();
    writeln!(out, "Benchmark: {} iters in {:?}", cnt, dur)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bench_command_succeeds() {
        // Test that bench command completes successfully
        let mut out = Vec::new();
        let result = handle_bench_command(&mut out);

        assert!(result.is_ok(), "Bench command should succeed");
    }

    #[test]
    fn test_bench_command_output_format() {
        // Test that output contains expected benchmark report format
        let mut out = Vec::new();
        handle_bench_command(&mut out).unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.contains("Benchmark:"),
            "Output should contain 'Benchmark:'"
        );
        assert!(
            output.contains("iters"),
            "Output should contain iteration count"
        );
    }

    #[test]
    fn test_bench_command_reports_200_iterations() {
        // Test that bench performs exactly 200 iterations as designed
        let mut out = Vec::new();
        handle_bench_command(&mut out).unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.contains("200 iters"),
            "Output should report 200 iterations"
        );
    }

    #[test]
    fn test_bench_command_deterministic() {
        // Test that bench produces consistent iteration count (deterministic seed)
        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        handle_bench_command(&mut out1).unwrap();
        handle_bench_command(&mut out2).unwrap();

        let output1 = String::from_utf8(out1).unwrap();
        let output2 = String::from_utf8(out2).unwrap();

        // Both should report same iteration count (timing may differ)
        assert!(
            output1.contains("200 iters"),
            "First run should have 200 iters"
        );
        assert!(
            output2.contains("200 iters"),
            "Second run should have 200 iters"
        );
    }

    #[test]
    fn test_bench_command_includes_timing() {
        // Test that output includes timing information
        let mut out = Vec::new();
        handle_bench_command(&mut out).unwrap();

        let output = String::from_utf8(out).unwrap();
        // Timing format should include some time measurement
        assert!(
            output.contains("ms") || output.contains("µs") || output.contains("ns"),
            "Output should include timing units"
        );
    }
}
