//! Random number generator verification command.
//!
//! The `rng` command verifies the properties of the ChaCha20 random number generator
//! used by the poker engine. It generates sample random values and displays them
//! for inspection.
//!
//! ## Purpose
//!
//! This command is primarily used for:
//! - Verifying RNG determinism (same seed produces same sequence)
//! - Debugging random number generation issues
//! - Validating RNG distribution properties

use crate::error::CliError;
use rand::{RngCore, SeedableRng};
use std::io::Write;

/// Handle the rng command - verify random number generator properties.
///
/// Generates and displays a sample of random numbers using the ChaCha20 RNG
/// with the specified seed (or a random seed if not provided).
///
/// # Arguments
///
/// * `seed` - Optional seed value for the RNG (uses random seed if None)
/// * `out` - Output stream for RNG sample values
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(CliError)` on write failure
///
/// # Example
///
/// ```ignore
/// # use axiomind_cli::commands::handle_rng_command;
/// # use std::io;
/// let mut out = io::stdout();
/// handle_rng_command(Some(12345), &mut out).expect("RNG command failed");
/// ```
pub fn handle_rng_command(seed: Option<u64>, out: &mut dyn Write) -> Result<(), CliError> {
    let s = seed.unwrap_or_else(rand::random);
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(s);
    let mut vals = vec![];
    for _ in 0..5 {
        vals.push(rng.next_u64());
    }
    writeln!(out, "RNG sample: {:?}", vals)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rng_command_with_explicit_seed() {
        let mut out = Vec::new();
        let seed = Some(12345u64);

        let result = handle_rng_command(seed, &mut out);

        assert!(result.is_ok());
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("RNG sample"));
    }

    #[test]
    fn test_rng_command_without_seed() {
        let mut out = Vec::new();

        let result = handle_rng_command(None, &mut out);

        assert!(result.is_ok());
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("RNG sample"));
    }

    #[test]
    fn test_rng_command_produces_deterministic_output() {
        let seed = Some(42u64);

        // Run twice with same seed
        let mut out1 = Vec::new();
        let _ = handle_rng_command(seed, &mut out1);

        let mut out2 = Vec::new();
        let _ = handle_rng_command(seed, &mut out2);

        // Output should be identical
        assert_eq!(out1, out2, "Same seed should produce same output");
    }

    #[test]
    fn test_rng_command_outputs_multiple_values() {
        let mut out = Vec::new();
        let seed = Some(123u64);

        let _ = handle_rng_command(seed, &mut out);

        let output = String::from_utf8(out).unwrap();

        // Output should contain multiple comma-separated values
        assert!(output.contains(","), "Should output multiple values");
    }
}
