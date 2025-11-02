# Project Structure

## Organization Philosophy

**Domain-driven with clear boundaries**: Core game logic isolated in `engine` (pure functions, no I/O), CLI and web server depend on engine but never the reverse. Each Rust package has single responsibility: `engine` = rules, `cli` = batch operations, `web` = streaming UI.

**Data as first-class citizen**: Hand histories in `data/hands/` are source of truth, SQLite in `data/db.sqlite` for fast queries. Docs in `docs/`, specs in `.kiro/specs/`, steering in `.kiro/steering/`.

## Directory Patterns

### Rust Workspace (`rust/`)
**Location**: `rust/engine/`, `rust/cli/`, `rust/web/`
**Purpose**: Monorepo structure with shared engine library
**Pattern**:
```
rust/
  engine/          # Core library (axm-engine)
    src/
      lib.rs       # Public API exports
      cards.rs     # Card representation and deck
      engine.rs    # Game orchestration
      game.rs      # State machine and action processing
      hand.rs      # Hand evaluation (rankings)
      player.rs    # Player state and stack management
      pot.rs       # Pot calculation and side pots
      rules.rs     # Betting rules and blind structure
      logger.rs    # Event emission and HandRecord serialization
      errors.rs    # Error types

  cli/             # CLI binary (axm)
    src/
      main.rs      # Entry point, clap argument parsing
      lib.rs       # Shared logic for commands
      commands/    # Subcommand implementations
      helpers/     # Utilities (config, paths, formatting)
    tests/
      integration/ # Integration tests for CLI commands
      helpers/     # Test utilities (cli_runner, assertions)

  web/             # Web server (axm_web)
    src/
      lib.rs       # Public API
      server.rs    # Warp HTTP server setup
      session.rs   # Session management
      events.rs    # SSE event streaming
      handlers/    # Request handlers
    static/        # HTML, CSS, JavaScript for UI
```

**Key principle**: Engine exports structs and functions, never imports from CLI or web. CLI and web can import engine.

### Data Storage (`data/`)
**Location**: `data/hands/`, `data/db.sqlite`, `data/logs/`
**Purpose**: All persistent game data
**Pattern**:
```
data/
  hands/
    YYYYMMDD/       # Daily directories
      HH-MM-SS.jsonl
      *.jsonl.zst   # Compressed archives
  db.sqlite         # Aggregation database
  logs/             # Application logs
```

**JSONL format**: One hand per line, UTF-8, LF line endings. See ADR-0001 for schema.

### Documentation (`docs/`)
**Location**: `docs/ARCHITECTURE.md`, `docs/CLI.md`, `docs/decisions/`
**Purpose**: Technical architecture, runbooks, ADRs
**Example**: `docs/decisions/0001-hand-history-jsonl.md`

**Language**: Japanese for user-facing docs (RUNBOOK, CLI usage), English for technical architecture and inline code comments.

### Kiro System (`.kiro/`)
**Location**: `.kiro/steering/`, `.kiro/specs/`, `.kiro/settings/`
**Purpose**: AI-DLC spec-driven development metadata
**Pattern**:
- `steering/` - Project memory (product, tech, structure)
- `specs/` - Feature specifications (requirements → design → tasks)
- `settings/` - Templates and rules (not documented in steering)

## Naming Conventions

- **Rust files**: `snake_case.rs` (matches module naming)
- **Rust modules**: `snake_case` (e.g., `axm_engine`, `axm_cli`, `axm_web`)
- **Binary names**: `axm` (CLI), `axm-web-server` (web server)
- **Structs/Enums**: `PascalCase` (e.g., `HandRecord`, `GameState`, `Action`)
- **Functions**: `snake_case` (e.g., `process_action`, `evaluate_hand`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_PLAYERS`, `DEFAULT_STACK`)

## Import Organization

```rust
// Standard library (alphabetical)
use std::fs::File;
use std::io::Write;

// External crates (alphabetical)
use rand::RngCore;
use serde::{Deserialize, Serialize};

// Internal crate modules (alphabetical)
use crate::cards::Card;
use crate::game::GameState;

// Workspace dependencies
use axm_engine::{Engine, HandRecord};
```

**Workspace dependencies**: CLI and web depend on `axm-engine` via `path = "../engine"` in Cargo.toml.

## Code Organization Principles

### Engine Isolation
Engine has **no I/O dependencies**. All logging happens via return values (e.g., `HandRecord`). Callers (CLI, web) decide where to write.

**Example**: `engine.play_hand()` returns `Result<HandRecord, EngineError>`. CLI writes to JSONL file, web streams via SSE.

### Testing Structure
- **Unit tests**: Inline with `#[cfg(test)] mod tests`
- **Integration tests**: Separate `tests/` directory at crate root
- **Test helpers**: `tests/helpers/` for reusable test utilities

**Temp files**: Integration tests use `temp_files.rs` helper to create isolated directories.

### Error Handling
Use `thiserror` for custom error types. Engine defines `EngineError`, CLI adds `CliError` wrapping engine errors.

```rust
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Invalid action: {0}")]
    InvalidAction(String),

    #[error("Game already finished")]
    GameFinished,
}
```

### Configuration
CLI uses TOML for config files (`.axm.toml`). Defaults embedded in binary, override via `--config` flag or `cfg` command.

**Location**: `~/.axm.toml` or per-project `.axm.toml`

---
_Generated: 2025-11-02_
