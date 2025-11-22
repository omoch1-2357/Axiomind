//! # Axiomind CLI Library
//!
//! This library provides the command-line interface for the Axiomind poker engine.
//! It exposes subcommands for playing, simulating, analyzing, and verifying poker hands.
//!
//! ## Main Entry Point
//!
//! The primary entry point is the [`run`] function, which parses command-line arguments
//! and executes the appropriate subcommand.
//!
//! ## Example Usage
//!
//! ```no_run
//! use std::io;
//! let args = vec!["axiomind", "play", "--vs", "ai", "--hands", "10"];
//! let code = axiomind_cli::run(args, &mut io::stdout(), &mut io::stderr());
//! assert_eq!(code, 0);
//! ```
//!
//! ## Available Subcommands
//!
//! - `play`: Play poker hands against AI or human opponents
//! - `sim`: Run large-scale simulations and generate hand histories
//! - `stats`: Aggregate statistics from JSONL hand history files
//! - `verify`: Validate game rules and hand history integrity
//! - `replay`: Replay previously recorded hands
//! - `deal`: Deal a single hand for inspection
//! - `bench`: Benchmark hand evaluation performance
//! - `eval`: Evaluate AI policies head-to-head
//! - `export`: Convert hand histories to various formats (CSV, JSON, SQLite)
//! - `dataset`: Create training/validation/test splits for ML
//! - `cfg`: Display current configuration settings
//! - `doctor`: Run environment diagnostics
//! - `rng`: Verify RNG properties

use clap::{Parser, Subcommand, ValueEnum};
use std::io::Write;
mod commands;
mod config;
mod error;
pub mod formatters;
pub mod io_utils;
pub mod ui;
pub mod validation;

// Import utility functions from extracted modules
use commands::{
    handle_bench_command, handle_cfg_command, handle_dataset_command, handle_deal_command,
    handle_doctor_command, handle_eval_command, handle_export_command, handle_play_command,
    handle_replay_command, handle_rng_command, handle_sim_command, handle_stats_command,
    handle_verify_command,
};

use axiomind_engine::engine::blinds_for_level;
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
/// Exit code: `0` for success, `2` for errors, `130` for interruptions
///
/// # Example
///
/// ```
/// use std::io;
/// let args = vec!["axiomind", "deal", "--seed", "42"];
/// let code = axiomind_cli::run(args, &mut io::stdout(), &mut io::stderr());
/// assert_eq!(code, 0);
/// ```
///
/// # Available Commands
///
/// - `play --vs {ai|human} --hands N`: Play N hands against AI or human
/// - `sim --hands N --output FILE`: Simulate N hands and save to FILE
/// - `stats --input PATH`: Display statistics from hand history files
/// - `verify --input PATH`: Validate hand history integrity
/// - `replay --input FILE`: Replay recorded hands
/// - `deal --seed N`: Deal a single hand with optional seed
/// - `bench`: Benchmark hand evaluation performance
/// - `eval --ai-a A --ai-b B --hands N`: Compare two AI policies
/// - `export --input IN --format FMT --output OUT`: Convert hand histories
/// - `dataset --input IN --outdir DIR`: Split data for training
/// - `cfg`: Display configuration settings
/// - `doctor`: Run environment diagnostics
/// - `rng --seed N`: Test RNG output
pub fn run<I, S>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    /// Aggregates statistics from JSONL hand history files.
    ///
    /// Reads hand history files (JSONL or .jsonl.zst) and computes summary statistics
    /// including total hands played and win distribution by player.
    ///
    /// # Arguments
    ///
    /// * `input` - Path to JSONL file or directory containing hand histories
    /// * `out` - Output stream for statistics report
    /// * `err` - Output stream for error messages and warnings
    ///
    /// # Returns
    ///
    /// `Result<(), CliError>`: `Ok(())` when statistics are valid, otherwise an `Err` that maps
    /// to exit code `2`.
    ///
    /// # Validation
    ///
    /// - Detects corrupted or incomplete records
    /// - Verifies chip conservation laws (sum of net_result must be zero)
    /// - Reports warnings for skipped records
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::io;
    /// let input = "data/hands/sample.jsonl";
    /// let code = axiomind_cli::run(
    ///     vec!["axiomind", "stats", "--input", input],
    ///     &mut io::stdout(),
    ///     &mut io::stderr()
    /// );
    /// assert_eq!(code, 0);
    /// ```
    const COMMANDS: &[&str] = &[
        "play", "replay", "stats", "verify", "deal", "bench", "sim", "eval", "export", "dataset",
        "cfg", "doctor", "rng",
    ];
    let argv: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();

    let parsed = AxiomindCli::try_parse_from(&argv);
    match parsed {
        Err(e) => {
            use clap::error::ErrorKind;

            // Help and version should print to stdout and exit 0
            match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    if write!(out, "{}", e).is_err() {
                        return 2;
                    }
                    0
                }
                _ => {
                    // Print clap error first
                    if writeln!(err, "{}", e).is_err()
                        || writeln!(err).is_err()
                        || writeln!(err, "Axiomind Poker CLI").is_err()
                        || writeln!(err, "Usage: axiomind <command> [options]\n").is_err()
                        || writeln!(err, "Commands:").is_err()
                    {
                        return 2;
                    }
                    for c in COMMANDS {
                        if writeln!(err, "  {}", c).is_err() {
                            return 2;
                        }
                    }
                    if writeln!(err, "\nFor full help, run: axiomind --help").is_err() {
                        return 2;
                    }
                    2
                }
            }
        }
        Ok(cli) => match cli.cmd {
            Commands::Cfg => match handle_cfg_command(out, err) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Play {
                vs,
                hands,
                seed,
                level,
            } => {
                // Use stdin for real input (supports both TTY and piped stdin)
                let stdin = std::io::stdin();
                let mut stdin_lock = stdin.lock();
                match handle_play_command(vs, hands, seed, level, out, err, &mut stdin_lock) {
                    Ok(()) => 0,
                    Err(e) => {
                        if writeln!(err, "Error: {}", e).is_err() {
                            return 2;
                        }
                        2
                    }
                }
            }
            Commands::Replay { input, speed } => {
                match handle_replay_command(input, speed, out, err) {
                    Ok(()) => 0,
                    Err(_) => 2,
                }
            }
            Commands::Stats { input } => match handle_stats_command(input, out, err) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Verify { input } => {
                let Some(path) = input else {
                    let _ = ui::write_error(err, "input required");
                    return 2;
                };
                match handle_verify_command(path, out, err) {
                    Ok(()) => 0,
                    Err(e) => {
                        if writeln!(err, "Error: {}", e).is_err() {
                            return 2;
                        }
                        2
                    }
                }
            }
            Commands::Doctor => match handle_doctor_command(out, err) {
                Ok(()) => 0,
                Err(_) => 2,
            },
            Commands::Eval {
                ai_a,
                ai_b,
                hands,
                seed,
            } => match handle_eval_command(&ai_a, &ai_b, hands, seed, out) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Bench => match handle_bench_command(out) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Deal { seed } => match handle_deal_command(seed, out) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Rng { seed } => match handle_rng_command(seed, out) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Sim {
                hands,
                output,
                seed,
                level,
                resume,
            } => match handle_sim_command(hands, output, seed, level, resume, out, err) {
                Ok(()) => 0,
                Err(CliError::Interrupted(_)) => 130,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Export {
                input,
                format,
                output,
            } => match handle_export_command(input, output, format, out, err) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
            Commands::Dataset {
                input,
                outdir,
                train,
                val,
                test,
                seed,
            } => match handle_dataset_command(input, outdir, train, val, test, seed, out, err) {
                Ok(()) => 0,
                Err(e) => {
                    if writeln!(err, "Error: {}", e).is_err() {
                        return 2;
                    }
                    2
                }
            },
        },
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "axiomind",
    author = "Axiomind",
    version,
    about = "Axiomind Poker CLI"
)]
struct AxiomindCli {
    #[command(subcommand)]
    cmd: Commands,
}

/// Available CLI subcommands.
///
/// Each variant represents a distinct operation mode of the Axiomind CLI.
/// Commands are designed to be composable: output from one command can often
/// be used as input to another (e.g., `sim` generates data, `stats` analyzes it).
#[derive(Subcommand, Debug)]
enum Commands {
    /// Play poker hands interactively or against AI.
    ///
    /// Run one or more hands of heads-up Texas Hold'em. In AI mode, the opponent
    /// makes automatic decisions. In human mode, the user is prompted for actions.
    ///
    /// # Options
    ///
    /// * `--vs` - Opponent type: `ai` or `human`
    /// * `--hands` - Number of hands to play (default: 1)
    /// * `--seed` - RNG seed for reproducibility (default: random)
    /// * `--level` - Blind level (1-20, higher means bigger blinds; levels 21+ treated as level 20)
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind play --vs ai --hands 10 --seed 42 --level 2
    /// ```
    Play {
        #[arg(long, value_enum)]
        vs: Vs,
        #[arg(long)]
        hands: Option<u32>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=20))]
        level: Option<u8>,
    },
    /// Replay previously recorded hands from a JSONL file.
    ///
    /// Load and replay hand histories, optionally controlling playback speed.
    ///
    /// # Options
    ///
    /// * `--input` - Path to JSONL file containing hand histories
    /// * `--speed` - Playback speed multiplier (default: 1.0, must be > 0)
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind replay --input data/hands/session.jsonl --speed 2.0
    /// ```
    Replay {
        #[arg(long)]
        input: String,
        #[arg(long)]
        speed: Option<f64>,
    },
    /// Aggregate statistics from hand history files.
    ///
    /// Compute summary statistics including total hands, win rates, and chip conservation
    /// validation from JSONL files or directories.
    ///
    /// # Options
    ///
    /// * `--input` - Path to JSONL file or directory
    ///
    /// # Output Format
    ///
    /// JSON object with:
    /// - `hands`: Total number of hands
    /// - `winners`: Win counts per player
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind stats --input data/hands/
    /// ```
    Stats {
        #[arg(long)]
        input: String,
    },
    /// Evaluate AI policies head-to-head.
    ///
    /// Compare two AI models by simulating N hands and reporting win rates.
    ///
    /// # Options
    ///
    /// * `--ai-a` - First AI policy identifier
    /// * `--ai-b` - Second AI policy identifier
    /// * `--hands` - Number of hands to simulate
    /// * `--seed` - RNG seed for reproducibility
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind eval --ai-a baseline --ai-b experimental --hands 1000 --seed 42
    /// ```
    Eval {
        #[arg(long, name = "ai-a")]
        ai_a: String,
        #[arg(long, name = "ai-b")]
        ai_b: String,
        #[arg(long)]
        hands: u32,
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Validate hand history integrity and game rules.
    ///
    /// Perform comprehensive validation checks on hand histories:
    /// - Board completeness (exactly 5 cards)
    /// - No duplicate cards
    /// - Chip conservation (net_result sums to zero)
    /// - Valid hand IDs
    /// - Betting rules compliance (no illegal reopening after short all-in)
    /// - Player roster consistency across hands
    ///
    /// # Options
    ///
    /// * `--input` - Path to JSONL file to verify
    ///
    /// # Returns
    ///
    /// Exit code 0 if all checks pass, 2 if any violations detected.
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind verify --input data/hands/session.jsonl
    /// ```
    Verify {
        #[arg(long)]
        input: Option<String>,
    },
    /// Deal a single hand for inspection.
    ///
    /// Generate and display hole cards for both players and the full board.
    /// Useful for debugging, testing, or manual game setup.
    ///
    /// # Options
    ///
    /// * `--seed` - RNG seed for reproducible deals (default: random)
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind deal --seed 12345
    /// ```
    Deal {
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Benchmark hand evaluation performance.
    ///
    /// Evaluates 200 random 7-card hands and reports execution time.
    /// Used for performance regression testing and optimization validation.
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind bench
    /// ```
    Bench,
    /// Run large-scale hand simulations.
    ///
    /// Generate and optionally record N hands of poker. Supports resuming from
    /// previous runs and breaking early for testing.
    ///
    /// # Options
    ///
    /// * `--hands` - Total number of hands to simulate
    /// * `--output` - Path to save hand histories (JSONL format)
    /// * `--seed` - Base RNG seed (each hand uses seed + hand_index)
    /// * `--level` - Blind level (1-20, higher means bigger blinds; levels 21+ treated as level 20)
    /// * `--resume` - Resume from existing JSONL file (skips completed hands)
    ///
    /// # Environment Variables
    ///
    /// * `axiomind_SIM_FAST` - Enable fast mode (batch writes, minimal output)
    /// * `axiomind_SIM_BREAK_AFTER` - Break after N hands (for testing)
    /// * `axiomind_SIM_SLEEP_MICROS` - Delay between hands in microseconds
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind sim --hands 10000 --output data/sim.jsonl --seed 42 --level 3
    /// ```
    Sim {
        #[arg(long)]
        hands: u64,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=20))]
        level: Option<u8>,
        #[arg(long)]
        resume: Option<String>,
    },
    /// Convert hand histories to various formats.
    ///
    /// Export JSONL hand histories to CSV, pretty-printed JSON, or SQLite database.
    ///
    /// # Options
    ///
    /// * `--input` - Path to input JSONL file
    /// * `--format` - Output format: `csv`, `json`, or `sqlite`
    /// * `--output` - Path to output file
    ///
    /// # Format Details
    ///
    /// - **csv**: Tabular format with columns: hand_id, seed, result, ts, actions, board
    /// - **json**: Pretty-printed JSON array of all hands
    /// - **sqlite**: Relational database with full-text search capability
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind export --input data/hands.jsonl --format sqlite --output data/hands.db
    /// ```
    Export {
        #[arg(long)]
        input: String,
        #[arg(long)]
        format: String,
        #[arg(long)]
        output: String,
    },
    /// Create training/validation/test dataset splits.
    ///
    /// Split a JSONL hand history file into train/val/test sets for machine learning.
    /// Supports both in-memory and streaming modes for large datasets.
    ///
    /// # Options
    ///
    /// * `--input` - Path to input JSONL file
    /// * `--outdir` - Output directory for split files
    /// * `--train` - Training set proportion (default: 0.8, can specify as 80 or 0.8)
    /// * `--val` - Validation set proportion (default: 0.1)
    /// * `--test` - Test set proportion (default: 0.1)
    /// * `--seed` - RNG seed for reproducible shuffling
    ///
    /// # Environment Variables
    ///
    /// * `axiomind_DATASET_STREAM_THRESHOLD` - Min records for streaming mode (default: 10000)
    /// * `axiomind_DATASET_STREAM_TRACE` - Enable streaming debug output
    ///
    /// # Output Files
    ///
    /// Creates `train.jsonl`, `val.jsonl`, and `test.jsonl` in the output directory.
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind dataset --input data/sim.jsonl --outdir data/splits --train 0.7 --val 0.2 --test 0.1
    /// ```
    Dataset {
        #[arg(long)]
        input: String,
        #[arg(long)]
        outdir: String,
        #[arg(long)]
        train: Option<f64>,
        #[arg(long)]
        val: Option<f64>,
        #[arg(long)]
        test: Option<f64>,
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Display current configuration settings.
    ///
    /// Shows all configuration values and their sources (default, file, or env var).
    /// Configuration hierarchy: environment variables > config file > defaults.
    ///
    /// # Configuration Sources
    ///
    /// 1. Environment variables (e.g., `axiomind_SEED`)
    /// 2. Config file (`~/.axiomind.toml` or project `.axiomind.toml`)
    /// 3. Built-in defaults
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind cfg
    /// ```
    Cfg,
    /// Run environment diagnostics.
    ///
    /// Verify that SQLite, file system, and locale are properly configured.
    /// Outputs a JSON report with pass/fail status for each check.
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind doctor
    /// ```
    Doctor,
    /// Test RNG output for debugging.
    ///
    /// Generate and display 5 random u64 values from the ChaCha20 RNG.
    /// Useful for verifying deterministic behavior and seed consistency.
    ///
    /// # Options
    ///
    /// * `--seed` - RNG seed (default: random)
    ///
    /// # Example
    ///
    /// ```bash
    /// axiomind rng --seed 12345
    /// ```
    Rng {
        #[arg(long)]
        seed: Option<u64>,
    },
}

/// Opponent type for the `play` command.
///
/// Determines whether the user plays against a human (interactive prompts)
/// or an AI opponent (automated decisions).
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Vs {
    /// Play against a human opponent (requires TTY for interactive input).
    Human,
    /// Play against an AI opponent (automated decision-making).
    Ai,
}

impl Vs {
    /// Returns the string representation of the opponent type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axiomind_cli::Vs;
    /// let opponent = Vs::Ai;
    /// assert_eq!(opponent.as_str(), "ai");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Vs::Human => "human",
            Vs::Ai => "ai",
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
}
