//! Command handler modules for Axiomind CLI.
//!
//! This module contains individual handler functions for each CLI subcommand.
//! Each command is implemented in its own module file with a consistent pattern:
//!
//! - Public handler function: `pub fn handle_COMMAND_command(...) -> Result<(), CliError>`
//! - Module-private helpers: Helper functions specific to that command
//! - Dependency injection: Output streams (`&mut dyn Write`) passed as parameters
//! - Error propagation: All errors propagated via `CliError` enum
//!
//! # Organization
//!
//! Commands are organized by complexity:
//! - **Simple commands** (Phase 2): cfg, doctor, rng, deal, bench
//! - **Moderate commands** (Phase 3): play, stats, eval, export
//! - **Complex commands** (Phase 4): replay, verify, sim, dataset
//!
//! # Example
//!
//! ```rust,ignore
//! // Command handlers will be added in Phase 2-4
//! use axiomind_cli::commands::handle_cfg_command;
//! use std::io;
//!
//! let mut out = io::stdout();
//! let mut err = io::stderr();
//! handle_cfg_command(&mut out, &mut err).expect("Command failed");
//! ```

// Phase 2: Simple command modules
mod bench;
mod cfg;
mod deal;
mod doctor;
mod rng;

pub use bench::handle_bench_command;
pub use cfg::handle_cfg_command;
pub use deal::handle_deal_command;
pub use doctor::handle_doctor_command;
pub use rng::handle_rng_command;

// Phase 3: Moderate command modules
mod eval;
mod export;
mod play;
mod stats;

pub use eval::handle_eval_command;
pub use export::handle_export_command;
pub use play::handle_play_command;
pub use stats::handle_stats_command;

// Phase 4: Complex command modules
mod dataset;
mod replay;
mod sim;
mod verify;

pub use dataset::handle_dataset_command;
pub use replay::handle_replay_command;
pub use sim::handle_sim_command;
pub use verify::handle_verify_command;
