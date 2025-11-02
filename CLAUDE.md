# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# AI-DLC and Spec-Driven Development

Kiro-style Spec Driven Development implementation on AI-DLC (AI Development Life Cycle)

## Project Context

### Paths
- Steering: `.kiro/steering/`
- Specs: `.kiro/specs/`

### Steering vs Specification

**Steering** (`.kiro/steering/`) - Guide AI with project-wide rules and context
**Specs** (`.kiro/specs/`) - Formalize development process for individual features

### Active Specifications
- Check `.kiro/specs/` for active specifications
- Use `/kiro:spec-status [feature-name]` to check progress

## Development Guidelines
- Think in English, but generate responses in Japanese (思考は英語、回答の生成は日本語で行うように)
- Avoid using Python for command-line file operations; use native shell commands or Rust tools instead
- Clean up temporary directories and files created during operations when work is complete

### Frontend Development (CRITICAL)
**IMPORTANT**: Web applications with JavaScript require comprehensive testing beyond Rust unit tests:

1. **Static Analysis (MANDATORY)**
   - Run `npm run lint` before committing any JavaScript changes
   - ESLint must pass with zero errors
   - Fix all syntax errors and type warnings

2. **Browser E2E Testing (MANDATORY)**
   - Run `npm run test:e2e` for all UI-related changes
   - Test must simulate real user interactions in an actual browser
   - Verify Content-Type headers, form submissions, and API integrations

3. **Integration Point Validation**
   - Test the complete flow: Browser → htmx → Server → Response
   - Verify API payload formats match server expectations
   - Check JavaScript runtime errors in browser console

**Why This Matters**: Passing Rust tests does NOT guarantee the frontend works. You must test in a real browser.

See `docs/TESTING.md` and `docs/FRONTEND_GUIDELINES.md` for details.

## Minimal Workflow
- Phase 0 (optional): `/kiro:steering`, `/kiro:steering-custom`
- Phase 1 (Specification):
  - `/kiro:spec-init "description"`
  - `/kiro:spec-requirements {feature}`
  - `/kiro:validate-gap {feature}` (optional: for existing codebase)
  - `/kiro:spec-design {feature} [-y]`
  - `/kiro:validate-design {feature}` (optional: design review)
  - `/kiro:spec-tasks {feature} [-y]`
- Phase 2 (Implementation): `/kiro:spec-impl {feature} [tasks]`
  - `/kiro:validate-impl {feature}` (optional: after implementation)
- Progress check: `/kiro:spec-status {feature}` (use anytime)

## Development Rules
- 3-phase approval workflow: Requirements → Design → Tasks → Implementation
- Human review required each phase; use `-y` only for intentional fast-track
- Keep steering current and verify alignment with `/kiro:spec-status`

## Steering Configuration
- Load entire `.kiro/steering/` as project memory
- Default files: `product.md`, `tech.md`, `structure.md`
- Custom files are supported (managed via `/kiro:steering-custom`)

---

## Project Overview

Axiomind is a poker game engine and AI training platform built in Rust with a focus on head-to-head (HU) Texas Hold'em. The codebase consists of:

- **Rust engine** (`rust/engine`): Core game rules, state transitions, RNG, hand evaluation, and event system
- **Rust CLI** (`rust/cli`): Binary `axm` for playing, simulation, verification, and dataset generation
- **Rust web server** (`rust/web`): Local HTTP server with htmx-based UI and SSE streaming
- **Python AI** (planned): Training and inference, initially file-based, eventually gRPC

## Architecture Highlights

### Workspace Structure
```
rust/
  engine/  - Core game engine library (axm-engine)
  cli/     - CLI tool binary (axm)
  web/     - Web server library (axm_web)
docs/      - Architecture, runbook, ADRs
data/      - Hand histories (JSONL), SQLite DB, logs
```

### Data Flow
1. Engine emits `HandRecord` to JSONL files in `data/hands/YYYYMMDD/*.jsonl`
2. Aggregation stored in SQLite (`data/db.sqlite`)
3. CLI reads JSONL for stats, verification, replay
4. Web server subscribes to engine events and streams to UI via SSE

### Hand History Format (JSONL)
- One hand per line, UTF-8, LF line endings
- Format specified in ADR-0001 (`docs/decisions/0001-hand-history-jsonl.md`)
- Compressed files use `.jsonl.zst` extension
- Fields include: `hand_id`, `seed`, `level`, `sb`, `bb`, `button`, `players`, `actions`, `board`, `showdown`, `net_result`, `end_reason`, `ts`

## Common Commands

### Building and Testing
```bash
# Build entire workspace
cargo build --release

# Build specific package
cargo build -p axm-engine
cargo build -p axm_cli
cargo build -p axm_web

# Run all tests
cargo test --workspace

# Run tests for specific package
cargo test -p axm-engine
cargo test -p axm_cli

# Run specific integration test
cargo test --test integration --package axm_cli

# Run single test by name
cargo test test_cli_commands::test_cfg_subcommand -- --exact
```

### Linting and Formatting
```bash
# Format all code (pre-commit hook enforces this)
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check

# Run clippy for linting
cargo clippy --workspace -- -D warnings
```

### Running the CLI
```bash
# Build and run CLI
cargo run -p axm_cli -- <command>

# Examples:
cargo run -p axm_cli -- play --vs ai --hands 10 --level 3
cargo run -p axm_cli -- sim --hands 1000 --ai baseline
cargo run -p axm_cli -- stats --input data/hands/
cargo run -p axm_cli -- verify
cargo run -p axm_cli -- deal
```

### Git Hooks
Pre-commit hook at `.githooks/pre-commit` automatically runs `cargo fmt --all` and stages changes if formatting is needed. Enable with:
```bash
git config core.hooksPath .githooks
```

## Key Components

### Engine Module (`rust/engine/src/`)
- `cards.rs` - Card representation and utilities
- `deck.rs` - Deck shuffling with reproducible RNG (ChaCha)
- `engine.rs` - Main game engine orchestration
- `game.rs` - Game state and action processing
- `hand.rs` - Hand evaluation and ranking
- `player.rs` - Player state and stack management
- `pot.rs` - Pot and side-pot calculations
- `rules.rs` - Betting rules, blinds, and level structure
- `logger.rs` - Event logging and HandRecord serialization
- `errors.rs` - Error types

### CLI Commands (`rust/cli/`)
Available via `axm` binary (see `docs/CLI.md` for full details):
- `play` - Play against AI or human
- `replay` - Replay hand history
- `sim` - Run large-scale simulations
- `eval` - Evaluate AI policies head-to-head
- `stats` - Aggregate JSONL statistics
- `verify` - Verify game rules and conservation laws
- `serve` - Start local web UI server
- `deal` - Deal single hand for inspection
- `bench` - Benchmark hand evaluation and state transitions
- `rng` - Verify RNG properties
- `cfg` - Display/override default config
- `doctor` - Environment diagnostics
- `export` - Format conversion and extraction
- `dataset` - Dataset creation and splitting
- `train` - Launch training (planned)

Common CLI options:
- `--seed <u64>` - Reproducible RNG seed
- `--ai-version <id>` - AI model version (default: latest)
- `--adaptive <on|off>` - Real-time AI adaptation (default: on)

### Web Server (`rust/web/`)
- `server.rs` - Warp-based HTTP server
- `session.rs` - Session management
- `events.rs` - SSE event stream for live game updates
- `static_handler.rs` - Serve static HTML/CSS/JS

### Testing Structure
- Unit tests inline with modules
- Integration tests in `rust/cli/tests/integration/`
- Test helpers in `rust/cli/tests/helpers/` including:
  - `cli_runner.rs` - Execute CLI with temp directories
  - `assertions.rs` - Custom assertions for JSONL, config
  - `temp_files.rs` - Temporary file management

## Reproducibility and Debugging

### RNG Seeding
All game outcomes are deterministic given a seed:
```bash
# Reproduce exact hand with same version
cargo run -p axm_cli -- play --seed 42 --hands 1
```

### JSONL Corruption Recovery
Engine detects incomplete lines at EOF and discards them. Verify with:
```bash
cargo run -p axm_cli -- verify --input data/hands/20250829/
```

### SQLite Locking
Single-process writes only. Use batching for bulk operations.

## Code Style and Conventions

- **Rust**: Follow `rustfmt` (enforced by pre-commit) and `clippy` guidelines
- **Commit messages**: Use Conventional Commits format
- **Branching**: Work on feature branches, keep `main` clean
- **Documentation**: Japanese for user-facing docs, English for technical architecture (inline code comments in English preferred)

## Important Architectural Decisions (ADRs)

See `docs/decisions/`:
- ADR-0001: JSONL format for hand history
- ADR-0002: SQLite for aggregation
- ADR-0003: Monorepo structure
- ADR-0004: Git and branching strategy

## Dependencies

### Rust
- `clap` - CLI argument parsing
- `serde`, `serde_json` - Serialization
- `rand`, `rand_chacha` - Deterministic RNG
- `rusqlite` - SQLite bindings
- `warp` - Web server framework
- `tokio` - Async runtime
- `zstd` - Compression

### Python (Future)
- Python 3.12+
- Virtual environment managed with `venv`
- Code formatting: `ruff`, `black`

## References

- `docs/ARCHITECTURE.md` - High-level system design
- `docs/RUNBOOK.md` - Setup and troubleshooting
- `docs/STACK.md` - Technology stack details
- `docs/CLI.md` - Complete CLI command reference
- `docs/GAME_RULES.md` - Game rules specification
