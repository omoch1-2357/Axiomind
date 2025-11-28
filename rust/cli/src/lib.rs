//! # Axiomind CLI Library
//!
//! Command-line interface for the Axiomind poker engine.
//!
//! ## Module Organization
//!
//! - **`cli`**: CLI structures (AxiomindCli, Commands enum)
//! - **`commands`**: Command handler implementations
//! - **`formatters`**: Card/board/action formatting
//! - **`io_utils`**: File I/O helpers (JSONL, compression)
//! - **`validation`**: Input parsing and validation
//! - **`config`**, **`error`**, **`ui`**: Support modules
//!
//! ## Usage
//!
//! ```no_run
//! use std::io;
//! let args = vec!["axiomind", "play", "--vs", "ai", "--hands", "10"];
//! let code = axiomind_cli::run(args, &mut io::stdout(), &mut io::stderr());
//! assert_eq!(code, 0);
//! ```
//!
//! ## Commands
//!
//! `play`, `sim`, `replay`, `stats`, `verify`, `eval`, `export`, `dataset`,
//! `deal`, `bench`, `rng`, `cfg`, `doctor`

use clap::Parser;
use std::io::Write;
pub mod cli;
mod commands;
mod config;
mod error;
pub mod formatters;
pub mod io_utils;
pub mod ui;
pub mod validation;

// Import CLI types from cli module
use cli::{AxiomindCli, Commands};

// Import utility functions from extracted modules
use commands::{
    handle_bench_command, handle_cfg_command, handle_dataset_command, handle_deal_command,
    handle_doctor_command, handle_eval_command, handle_export_command, handle_play_command,
    handle_replay_command, handle_rng_command, handle_sim_command, handle_stats_command,
    handle_verify_command,
};

// Re-exports
pub use cli::Vs;
pub use error::{BatchValidationError, CliError};

/// Main entry point for the CLI application.
///
/// Parses command-line arguments and dispatches to the appropriate subcommand handler.
///
/// # Arguments
///
/// * `args` - Iterator over command-line arguments (typically `std::env::args()`)
/// * `out` - Output stream for normal output (typically `stdout`)
/// * `err` - Output stream for error messages (typically `stderr`)
///
/// # Returns
///
/// Exit code: `0` for success, `2` for errors, `130` for interruptions (Ctrl+C)
///
/// # Example
///
/// ```
/// use std::io;
/// let args = vec!["axiomind", "deal", "--seed", "42"];
/// let code = axiomind_cli::run(args, &mut io::stdout(), &mut io::stderr());
/// assert_eq!(code, 0);
/// ```
pub fn run<I, S>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let argv: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();

    let parsed = AxiomindCli::try_parse_from(&argv);
    match parsed {
        Err(e) => handle_parse_error(e, out, err),
        Ok(cli) => execute_command(cli.cmd, out, err),
    }
}

/// Handle clap parsing errors with appropriate output and exit codes.
fn handle_parse_error(e: clap::Error, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    use clap::error::ErrorKind;

    match e.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            let _ = write!(out, "{}", e);
            0
        }
        _ => {
            // For parse errors, show clap's error message plus a helpful commands list
            const COMMANDS: &[&str] = &[
                "play", "replay", "stats", "verify", "deal", "bench", "sim", "eval", "export",
                "dataset", "cfg", "doctor", "rng",
            ];

            let _ = writeln!(err, "{}", e);
            let _ = writeln!(err);
            let _ = writeln!(err, "Axiomind Poker CLI");
            let _ = writeln!(err, "Usage: axiomind <command> [options]\n");
            let _ = writeln!(err, "Commands:");
            for c in COMMANDS {
                let _ = writeln!(err, "  {}", c);
            }
            let _ = writeln!(err, "\nFor full help, run: axiomind --help");
            2
        }
    }
}

/// Execute the parsed command and convert the result to an exit code.
fn execute_command(cmd: Commands, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let result = match cmd {
        Commands::Cfg => handle_cfg_command(out, err),
        Commands::Play {
            vs,
            hands,
            seed,
            level,
        } => {
            let stdin = std::io::stdin();
            let mut stdin_lock = stdin.lock();
            handle_play_command(vs, hands, seed, level, out, err, &mut stdin_lock)
        }
        Commands::Replay { input, speed } => handle_replay_command(input, speed, out, err),
        Commands::Stats { input } => handle_stats_command(input, out, err),
        Commands::Verify { input } => {
            if let Some(path) = input {
                handle_verify_command(path, out, err)
            } else {
                let _ = ui::write_error(err, "input required");
                return 2;
            }
        }
        Commands::Doctor => handle_doctor_command(out, err),
        Commands::Eval {
            ai_a,
            ai_b,
            hands,
            seed,
        } => handle_eval_command(&ai_a, &ai_b, hands, seed, out),
        Commands::Bench => handle_bench_command(out),
        Commands::Deal { seed } => handle_deal_command(seed, out),
        Commands::Rng { seed } => handle_rng_command(seed, out),
        Commands::Sim {
            hands,
            output,
            seed,
            level,
            resume,
        } => handle_sim_command(hands, output, seed, level, resume, out, err),
        Commands::Export {
            input,
            format,
            output,
        } => handle_export_command(input, output, format, out, err),
        Commands::Dataset {
            input,
            outdir,
            train,
            val,
            test,
            seed,
        } => handle_dataset_command(input, outdir, train, val, test, seed, out, err),
    };

    match result {
        Ok(()) => 0,
        Err(CliError::Interrupted(_)) => 130,
        Err(e) => {
            let _ = writeln!(err, "Error: {}", e);
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test command dispatch integration (Task 8.1)
    #[test]
    fn test_cfg_command_dispatch() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_cfg_command(&mut out, &mut err);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Configuration") || !output.is_empty());
    }

    #[test]
    fn test_doctor_command_dispatch() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let result = handle_doctor_command(&mut out, &mut err);
        // Doctor command should succeed
        assert!(result.is_ok()); // Either outcome is valid for test
    }

    #[test]
    fn test_rng_command_dispatch_with_seed() {
        let mut out = Vec::new();

        let result = handle_rng_command(Some(42), &mut out);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("RNG") || output.contains("seed") || !output.is_empty());
    }

    #[test]
    fn test_rng_command_dispatch_without_seed() {
        let mut out = Vec::new();

        let result = handle_rng_command(None, &mut out);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deal_command_dispatch_with_seed() {
        let mut out = Vec::new();

        let result = handle_deal_command(Some(42), &mut out);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_deal_command_dispatch_without_seed() {
        let mut out = Vec::new();

        let result = handle_deal_command(None, &mut out);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bench_command_dispatch() {
        let mut out = Vec::new();

        let result = handle_bench_command(&mut out);
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("hands/sec") || output.contains("Benchmark") || !output.is_empty());
    }

    // Integration tests for play command moved to commands/play.rs module

    #[test]
    fn test_play_level_validation_rejects_out_of_range() {
        // Test that clap rejects level=0
        let result =
            AxiomindCli::try_parse_from(["axiomind", "play", "--vs", "ai", "--level", "0"]);
        assert!(result.is_err());

        // Test that clap rejects level=21
        let result =
            AxiomindCli::try_parse_from(["axiomind", "play", "--vs", "ai", "--level", "21"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_play_level_validation_accepts_valid_range() {
        // Test that clap accepts level=1
        let result =
            AxiomindCli::try_parse_from(["axiomind", "play", "--vs", "ai", "--level", "1"]);
        assert!(result.is_ok());

        // Test that clap accepts level=20
        let result =
            AxiomindCli::try_parse_from(["axiomind", "play", "--vs", "ai", "--level", "20"]);
        assert!(result.is_ok());
    }

    // Phase 3 command dispatch tests (Task 12.1 - TDD)

    #[test]
    fn test_stats_command_dispatch_integration() {
        // Test that stats command module is properly integrated
        // This test verifies the dispatch works, not the full functionality
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Use a non-existent file to test error handling path
        let result = handle_stats_command("nonexistent.jsonl".to_string(), &mut out, &mut err);

        // Should return an error (file doesn't exist)
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_command_dispatch_integration() {
        // Test that eval command module is properly integrated
        let mut out = Vec::new();

        // Run eval with minimal hands count
        let result = handle_eval_command("baseline", "baseline", 1, Some(42), &mut out);

        // Should succeed with baseline AI
        assert!(result.is_ok());

        let output = String::from_utf8(out).unwrap();
        // Output should contain some eval results
        assert!(!output.is_empty());
    }

    #[test]
    fn test_export_command_dispatch_integration() {
        // Test that export command module is properly integrated
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Use a non-existent input file to test error handling
        let result = handle_export_command(
            "nonexistent.jsonl".to_string(),
            "output.csv".to_string(),
            "csv".to_string(),
            &mut out,
            &mut err,
        );

        // Should return an error (file doesn't exist)
        assert!(result.is_err());
    }

    #[test]
    fn test_play_command_dispatch_via_handler() {
        // Test that play command handler is properly exposed
        use std::io::Cursor;

        let mut out = Vec::new();
        let mut err = Vec::new();
        let input = "quit\n";
        let mut stdin = Cursor::new(input.as_bytes());

        // Run play command with AI opponent, 1 hand
        let result = handle_play_command(
            Vs::Ai,
            Some(1),
            Some(42),
            Some(1),
            &mut out,
            &mut err,
            &mut stdin,
        );

        // Should complete successfully via handler
        assert!(result.is_ok());
    }

    // Phase 5 CLI module extraction tests (Task 21.1 - TDD)

    #[test]
    fn test_cli_module_exists_and_exports_axiomind_cli() {
        // Test that cli module is accessible and AxiomindCli can be constructed
        use crate::cli::AxiomindCli;

        // Should be able to parse valid arguments
        let result = AxiomindCli::try_parse_from(["axiomind", "cfg"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_module_exports_commands_enum() {
        // Test that Commands enum is accessible from cli module
        use crate::cli::Commands;

        // Verify Commands enum has expected variants by parsing
        let cli = crate::cli::AxiomindCli::try_parse_from(["axiomind", "bench"]).unwrap();

        // Match on Commands to verify it's the expected type
        match cli.cmd {
            Commands::Bench => {} // Expected variant
            _ => panic!("Expected Commands::Bench variant"),
        }
    }

    #[test]
    fn test_cli_types_preserve_all_13_subcommands() {
        // Verify all 13 subcommands are preserved in Commands enum
        let commands = vec![
            vec!["axiomind", "cfg"],
            vec!["axiomind", "play", "--vs", "ai"],
            vec!["axiomind", "replay", "--input", "test.jsonl"],
            vec!["axiomind", "stats", "--input", "test.jsonl"],
            vec!["axiomind", "verify", "--input", "test.jsonl"],
            vec!["axiomind", "doctor"],
            vec![
                "axiomind", "eval", "--ai-a", "a", "--ai-b", "b", "--hands", "1",
            ],
            vec!["axiomind", "bench"],
            vec!["axiomind", "deal"],
            vec!["axiomind", "rng"],
            vec!["axiomind", "sim", "--hands", "1"],
            vec![
                "axiomind", "export", "--input", "a", "--format", "csv", "--output", "b",
            ],
            vec!["axiomind", "dataset", "--input", "a", "--outdir", "b"],
        ];

        // All should parse successfully
        for cmd_args in commands {
            let result = crate::cli::AxiomindCli::try_parse_from(&cmd_args);
            assert!(result.is_ok(), "Failed to parse: {:?}", cmd_args);
        }
    }
}
