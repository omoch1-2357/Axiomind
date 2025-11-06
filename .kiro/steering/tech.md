# Technology Stack

## Architecture

**Monorepo with clean boundaries**: Rust workspace for core engine, CLI binary, and web server. Python AI planned as separate component with file-based or gRPC integration. Engine is pure logic (no I/O), CLI and web handle user interaction.

**Event-driven logging**: Engine emits structured events to JSONL files (append-only), web server subscribes to event streams for real-time UI updates via SSE.

## Core Technologies

- **Language**: Rust (stable edition 2021)
- **Runtime**: Tokio async runtime for web server
- **Data Storage**: JSONL files (hand histories) + SQLite (aggregation)
- **Web Framework**: Warp 0.3 with htmx-based UI
- **RNG**: ChaCha8 deterministic PRNG

## Key Libraries

- **clap 4.5**: CLI argument parsing with derive macros
- **serde/serde_json**: Serialization of HandRecord to JSONL
- **rand/rand_chacha**: Deterministic shuffling and seeding
- **rusqlite**: SQLite bindings for aggregation queries
- **tokio**: Async runtime, SSE streaming, file I/O
- **warp**: HTTP server with filter combinators
- **zstd**: JSONL compression for archival

## Development Standards

### Type Safety
- Rust's ownership system enforced at compile time
- No unsafe blocks in core game logic
- Strong typing for cards, actions, game states

### Code Quality
- **rustfmt**: Enforced via pre-commit hook (`.githooks/pre-commit`)
- **clippy**: Linting with `-D warnings` (treat warnings as errors)
- **Conventional Commits**: Required commit message format

### Testing
- **Unit tests**: Inline with modules, test determinism and conservation laws
- **Integration tests**: CLI command execution in isolated temp directories (`rust/cli/tests/integration/`)
- **E2E tests**: Browser automation for web UI (`npm run test:e2e`)
- **Frontend validation**: ESLint required for JavaScript changes (`npm run lint`)

### Reproducibility
- All tests use fixed seeds for deterministic outcomes
- Hand history format specified in ADR-0001
- CLI `--seed` flag ensures reproducible game runs

## Development Environment

### Required Tools
- Rust stable (via rustup)
- Python 3.12+ (for future AI component)
- Node.js (for frontend testing and linting)

### Common Commands
```bash
# Build entire workspace
cargo build --release

# Build specific package
cargo build -p axm-engine
cargo build -p axm_cli
cargo build -p axm_web

# Run all tests
cargo test --workspace

# Format and lint (pre-commit does this automatically)
cargo fmt --all
cargo clippy --workspace -- -D warnings

# Frontend validation
npm run lint
npm run test:e2e

# Run CLI
cargo run -p axm_cli -- play --vs ai --hands 10
cargo run -p axm_cli -- sim --hands 1000

# Start web server
cargo run -p axm_web --bin axm-web-server
```

### Git Hooks
Pre-commit hook at `.githooks/pre-commit` auto-formats code and stages changes. Enable with:
```bash
git config core.hooksPath .githooks
```

## Key Technical Decisions

### JSONL for Hand History (ADR-0001)
**Why**: Append-only, corruption-resilient, line-oriented (easy to split/merge), human-readable for debugging.

**Format**: One JSON object per line, UTF-8, LF line endings. Compressed archives use `.jsonl.zst`.

### SQLite for Aggregation (ADR-0002)
**Why**: Serverless, file-based, excellent query performance for stats. Single-writer model matches batch operation pattern.

**Usage**: Pre-computed stats, search indexes. JSONL remains source of truth.

### Monorepo Structure (ADR-0003)
**Why**: Shared engine logic across CLI and web server, atomic cross-component changes, unified versioning.

**Layout**: `rust/engine` (library), `rust/cli` (binary), `rust/web` (library + binary).

### Deterministic RNG
**Why**: Reproducible outcomes enable scientific comparison, debugging, and verification of edge cases.

**Implementation**: ChaCha8 PRNG with explicit seed management. Every hand includes seed in HandRecord for replay.

### Frontend: htmx + SSE
**Why**: Minimal JavaScript, server-driven UI updates, progressive enhancement. Game state lives on server, browser is thin client.

**Testing**: Browser E2E required (Rust tests don't validate JavaScript/htmx integration).

---
_Generated: 2025-11-02_
