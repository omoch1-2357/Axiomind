# Project Structure

## Organization Philosophy

**Layered workspace with strict boundaries**: Core engine contains pure game logic with zero I/O dependencies. Consumer applications (CLI, web) handle I/O, UI, and orchestration. Clear separation enables testing engine in isolation and reusing it across multiple interfaces.

**Data-first architecture**: Engine emits immutable event records (HandRecord), consumers are responsible for persistence (JSONL files), aggregation (SQLite), and presentation (terminal, web UI). Engine never touches filesystem or network.

## Directory Patterns

### Engine Library (`rust/engine/`)
**Location**: `rust/engine/src/`
**Purpose**: Pure game logic - card representation, deck shuffling, hand evaluation, betting rules, state transitions. No I/O, no async, no UI concerns.
**Example**: `cards.rs` (Card/Rank/Suit types), `deck.rs` (ChaCha RNG shuffling), `hand.rs` (5-card evaluator), `game.rs` (action processing), `logger.rs` (HandRecord serialization structures)

**Key principle**: Engine is deterministic and testable in isolation. Given a seed, outcomes are reproducible. All game state is observable through public methods or HandRecord events.

### CLI Application (`rust/cli/`)
**Location**: `rust/cli/src/`
**Purpose**: Command-line interface for batch operations - simulation, verification, stats aggregation, hand replay, dataset generation. Direct filesystem and SQLite access.
**Example**: `main.rs` (clap subcommands), `config.rs` (TOML config loading), `ui.rs` (terminal output formatting)

**Key commands**: `play` (interactive), `sim` (bulk simulation), `stats` (JSONL aggregation), `verify` (rule checking), `serve` (launch web server), `dataset` (training data preparation)

**Testing pattern**: Integration tests in `tests/integration/`, helpers in `tests/helpers/` (cli_runner.rs for temp directory execution, assertions.rs for JSONL validation)

### Web Server Library (`rust/web/`)
**Location**: `rust/web/src/`
**Purpose**: Local HTTP server with HTML UI - session management, SSE event streaming, static file serving. Bridges engine events to browser via server-sent events.
**Example**: `server.rs` (warp routes), `session.rs` (UUID-based sessions), `events.rs` (SSE broadcast), `static_handler.rs` (HTML/CSS/JS serving)

**Architecture**: Engine actions → tokio broadcast channel → SSE streams per session → htmx DOM updates. Async runtime (tokio) isolates async I/O from synchronous engine.

### Documentation (`docs/`)
**Location**: `docs/`
**Purpose**: Architecture design, runbooks, ADRs (Architectural Decision Records), CLI reference, game rules specification
**Example**: `ARCHITECTURE.md` (system overview), `CLI.md` (command reference), `decisions/` (ADRs in markdown)

**Language**: Technical docs in Japanese (architecture, runbook), command reference and inline code comments in English

### Data Storage (`data/`)
**Location**: `data/`
**Purpose**: Hand history JSONL files, SQLite database, logs. Organized by date for JSONL (`data/hands/YYYYMMDD/*.jsonl`).
**Example**: `data/hands/20250829/session_001.jsonl` (hand records), `data/db.sqlite` (aggregated stats)

**Gitignore pattern**: `data/` is excluded from version control (generated content)

## Naming Conventions

### Files
- **Rust source**: `snake_case.rs` (cards.rs, hand.rs, player.rs)
- **Packages**: `kebab-case` in Cargo.toml name field (axm-engine, axm_cli, axm_web)
- **Library names**: `snake_case` in Cargo.toml lib name field (axm_engine, axm_cli, axm_web)
- **Binaries**: Short names (axm) for CLI executable

### Types and Functions
- **Structs/Enums**: PascalCase (Card, HandRecord, PlayerAction, Street)
- **Functions**: snake_case (deal_hand, evaluate_hand, process_action)
- **Constants**: SCREAMING_SNAKE_CASE (STARTING_STACK, DEFAULT_LEVEL)
- **Modules**: snake_case matching filename (cards, deck, engine)

### Data Files
- **JSONL**: `{identifier}.jsonl` or `{identifier}.jsonl.zst` (compressed)
- **Database**: `db.sqlite` (single aggregation database)
- **Config**: `config.toml` (CLI default configuration)

## Module Organization

### Engine Library Structure
```rust
// lib.rs: Public module declarations only
pub mod cards;
pub mod deck;
pub mod engine;
// ...

// cards.rs: Domain types and logic
pub struct Card { ... }
pub enum Rank { ... }
pub enum Suit { ... }

// errors.rs: Error types with thiserror
#[derive(Error, Debug)]
pub enum EngineError { ... }
```

**Principle**: Each module is self-contained domain logic. Public API through `pub` declarations in lib.rs. Errors in dedicated errors.rs module.

### Import Style
```rust
// Standard library first
use std::fs::File;
use std::io::{BufWriter, Write};

// External crates
use serde::{Deserialize, Serialize};
use rand::SeedableRng;

// Internal crate modules
use crate::cards::{Card, Rank};
use crate::deck::Deck;
```

**Convention**: Three-section imports (std, external, internal), grouped by related types, absolute paths via `crate::` prefix.

## Code Organization Principles

### Dependency Direction
```
CLI/Web → Engine
     ↓
  Data files (JSONL, SQLite)
```

Engine has no dependencies on CLI or Web. CLI and Web depend on engine via workspace path dependencies. Data files are output artifacts, never inputs to engine (engine state is in-memory only).

### Separation of Concerns
- **Engine**: Game rules, state transitions, hand evaluation → Pure functions, deterministic RNG
- **CLI**: Orchestration, I/O, user interaction → Calls engine methods, writes JSONL, queries SQLite
- **Web**: HTTP serving, session management, event streaming → Wraps engine in async runtime, broadcasts events

### Testing Boundaries
- **Unit tests**: Inline with modules (`#[cfg(test)]`), test pure logic in isolation
- **Integration tests**: CLI `tests/integration/`, test end-to-end CLI commands with temp directories
- **Test helpers**: Reusable utilities in `tests/helpers/` (CLI runner, JSONL assertions, temp file management)

### Configuration Management
- **Engine**: Configuration via constructor parameters (seed, level), no global state
- **CLI**: TOML config file for defaults, command-line flags override config
- **Web**: Server config (port, host) via startup parameters, session state in memory (HashMap)

## Workspace Structure

```
Axiomind/
├── rust/
│   ├── engine/     # axm-engine library
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── cards.rs
│   │   │   ├── deck.rs
│   │   │   ├── engine.rs
│   │   │   ├── game.rs
│   │   │   ├── hand.rs
│   │   │   ├── player.rs
│   │   │   ├── pot.rs
│   │   │   ├── rules.rs
│   │   │   ├── logger.rs
│   │   │   └── errors.rs
│   │   └── Cargo.toml
│   ├── cli/        # axm binary
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── lib.rs
│   │   │   ├── config.rs
│   │   │   └── ui.rs
│   │   ├── tests/
│   │   │   ├── integration/
│   │   │   └── helpers/
│   │   └── Cargo.toml
│   └── web/        # axm_web library
│       ├── src/
│       │   ├── lib.rs
│       │   ├── server.rs
│       │   ├── session.rs
│       │   ├── events.rs
│       │   └── static_handler.rs
│       └── Cargo.toml
├── docs/           # Documentation and ADRs
│   ├── ARCHITECTURE.md
│   ├── CLI.md
│   ├── STACK.md
│   └── decisions/
├── data/           # Generated data (gitignored)
│   ├── hands/
│   │   └── YYYYMMDD/
│   └── db.sqlite
├── .githooks/      # Pre-commit formatting
├── Cargo.toml      # Workspace root
└── CLAUDE.md       # AI development instructions
```

**Organization rationale**: Workspace enables atomic cross-package changes during active development. Engine can be extracted as standalone crate later. Data directory is separate from code for clear distinction between source and output artifacts.

---
_Document patterns, not file trees. New files following patterns shouldn't require updates_
