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
// (modules and re-exports will be added as commands are extracted)

// Phase 3: Moderate command modules
// (to be added in Phase 3)

// Phase 4: Complex command modules
// (to be added in Phase 4)
