# Technology Stack

## Architecture

**Monorepo with layered separation**: Core game engine as library (axm-engine), consumer applications (CLI, web server) as separate packages. Strict boundary enforcement - game rules and state transitions isolated from I/O and UI concerns.

**Data flow pattern**: Engine emits immutable `HandRecord` events → JSONL files (append-only) → SQLite aggregation → Consumer tools read/analyze. Event-driven with SSE streaming for web UI.

## Core Technologies

- **Language**: Rust (stable, edition 2021)
- **Build System**: Cargo workspace with 3 members (engine, cli, web)
- **Runtime**: Tokio async runtime for web server, synchronous execution for engine/CLI

## Key Libraries

### Engine Core
- `rand` + `rand_chacha`: Deterministic RNG using ChaCha algorithm
- `serde` + `serde_json`: Serialization for JSONL hand records
- `thiserror`: Structured error handling
- `chrono`: Timestamp generation for hand records

### CLI
- `clap` (derive feature): Argument parsing with subcommands
- `rusqlite` (bundled): SQLite aggregation database
- `zstd`: JSONL compression for archival

### Web Server
- `warp`: HTTP server with filter-based routing
- `tokio`: Async runtime and SSE streaming
- `uuid`: Session ID generation
- `tokio-stream`: SSE event stream management

## Development Standards

### Type Safety
- Rust strict mode: no `unsafe`, explicit error handling with `Result<T, E>`
- Strong typing for domain concepts (Card, Street, PlayerAction, HandRecord)
- Exhaustive enum matching, no wildcard patterns in critical logic

### Code Quality
- **Formatting**: `rustfmt` enforced by pre-commit hook (auto-format and stage)
- **Linting**: `clippy` with `-D warnings` (deny all clippy warnings)
- **Conventions**: English for code/comments, Japanese for user-facing docs

### Testing
- Unit tests inline with modules (game logic, hand evaluation, pot calculation)
- Integration tests in `rust/cli/tests/integration/`
- Test helpers: `cli_runner.rs` (temp dir execution), `assertions.rs` (JSONL validation)
- Deterministic testing: All tests use explicit seeds for reproducibility

### CI/CD Quality Gates

**All code must pass these checks before being considered complete:**

#### Mandatory Checks (Blocking)
1. **Compilation**: `cargo check --workspace --all-features`
   - Zero compilation errors
   - All features must compile cleanly

2. **Testing**: `cargo test --workspace --all-features`
   - All tests must pass (unit, integration, doc tests)
   - No test failures or panics
   - New features require new tests

#### Required Checks (Must Address)
3. **Formatting**: `cargo fmt --all -- --check`
   - Must be compliant before commit
   - Pre-commit hook enforces this automatically
   - Run `cargo fmt --all` to fix

4. **Linting**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - **Zero warnings policy**: All clippy warnings must be addressed
   - Use `#[allow(clippy::xxx)]` with inline justification only when necessary
   - Common allowed patterns must be documented in this file

#### Test Coverage Requirements
- **New public APIs**: Must have unit tests
- **Complex logic**: Requires multiple test cases (happy path + edge cases)
- **Integration points**: CLI/Web features need integration tests
- **Deterministic testing**: All tests must use explicit seeds for reproducibility

#### Pre-commit Validation
```bash
# Automated by .githooks/pre-commit
cargo fmt --all              # Auto-format
cargo check --workspace      # Quick compile check
cargo clippy --workspace -- -D warnings  # Lint check
```

#### Validation Frequency
- **During TDD**: After each RED-GREEN-REFACTOR cycle
- **Before marking task complete**: Full CI suite via `/kiro:spec-impl`
- **Before PR/merge**: Full validation via `/kiro:validate-impl`
- **On push**: GitHub Actions CI pipeline (if configured)

#### CI/CD Philosophy
**Shift-left principle**: Catch issues early in development cycle, not in CI pipeline. Local validation should match CI pipeline exactly.

**Zero-warning policy**: Warnings become errors over time. Address immediately or document why they're acceptable.

**Test-first mandate**: Following TDD ensures tests exist before code. No code should be written without tests.

## Development Environment

### Required Tools
- Rust stable toolchain (rustup recommended)
- Cargo: build, test, fmt, clippy
- Git with configured hooks path: `git config core.hooksPath .githooks`

### Common Commands
```bash
# Build workspace
cargo build --release

# Run specific package tests
cargo test -p axm-engine
cargo test -p axm_cli --test integration

# Format and lint
cargo fmt --all
cargo clippy --workspace -- -D warnings

# Run CLI tool
cargo run -p axm_cli -- play --seed 42 --hands 10
cargo run -p axm_cli -- serve --port 8080
```

## Key Technical Decisions

### Deterministic RNG (ChaCha-based)
**Why**: Poker AI research requires exact reproducibility. Given a seed, every shuffle, card deal, and game outcome must be identical across runs and platforms. ChaCha provides cryptographic-quality randomness with perfect reproducibility.

**Pattern**: Engine initializes with `Deck::new_with_seed(u64)`, CLI accepts `--seed` flag, hand records store seed for replay.

### JSONL Hand History Format (ADR-0001)
**Why**: Line-delimited JSON allows append-only writes without parsing entire file, handles corruption gracefully (discard incomplete final line), and is universally readable by analysis tools.

**Pattern**: One hand per line, UTF-8, LF endings. Fields: `hand_id`, `seed`, `actions`, `board`, `showdown`, `net_result`, `end_reason`, `ts`. Compression via `.jsonl.zst` for archival.

### SQLite for Aggregation (ADR-0002)
**Why**: Local analysis needs indexed queries (win rates by position, action frequencies). SQLite provides SQL without server overhead, transactions for consistency, and portable single-file storage.

**Pattern**: CLI writes batched inserts, single-process writer, read-only queries from multiple processes. Schema tracks player stats, action distributions, showdown frequencies.

### Workspace Monorepo (ADR-0003)
**Why**: Tight coupling between engine and consumers during active development. Atomic cross-package changes, shared dependency versions, single test command.

**Pattern**: Root `Cargo.toml` defines workspace, engine as library dependency via `path = "../engine"`, consumers in `rust/{cli,web}`.

### Event-Driven Web Architecture
**Why**: Web UI needs real-time updates during hand progression. SSE allows server-push without WebSocket complexity, compatible with htmx-based UI.

**Pattern**: Engine actions emit events → `tokio::sync::broadcast` channel → SSE stream per session → htmx triggers DOM updates.

## Conventions

### Module Organization
- `lib.rs`: Public module declarations only, no logic
- Domain logic in separate modules: `cards`, `deck`, `game`, `hand`, etc.
- Errors in dedicated `errors.rs` using `thiserror`

### Naming
- **Packages**: `axm-engine` (kebab in Cargo.toml), `axm_engine` (snake_case lib name)
- **Types**: PascalCase (Card, HandRecord, PlayerAction)
- **Functions**: snake_case (deal_hand, evaluate_hand)
- **Constants**: SCREAMING_SNAKE_CASE (STARTING_STACK)

### Import Style
- Standard library first, external crates, then internal crate modules
- Use `crate::` for internal absolute paths
- Group related imports: `use crate::cards::{Card, Rank, Suit};`

### Error Handling
- Engine returns `Result<T, String>` for simplicity (MVP stage)
- CLI and web use `thiserror` for structured errors
- No panics in production code paths, use `expect()` only for invariants

---
_Document standards and patterns, not every dependency_
