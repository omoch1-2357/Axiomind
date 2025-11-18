# Technology Stack

## Architecture

**Rust-First with Loose Coupling**: Core game logic in Rust for correctness and performance, with modular interfaces for CLI, web, and AI components. Data flows through files (JSONL, SQLite) rather than direct coupling, enabling independent development.

## Core Technologies

- **Language**: Rust (stable, edition 2021)
- **Runtime**: Node.js 18+ (for frontend tooling only, not production)
- **Frontend**: Static HTML + htmx (no build step), vanilla JavaScript (ES2021+)
- **Future**: Python 3.12 for AI training (planned, file-based or gRPC integration)

## Rust Workspace Structure

### Three-Crate Workspace (Cargo workspace resolver 2)

**axiomind-engine** (`rust/engine/`): Core game logic
- Rules, state transitions, random number generation (ChaCha8)
- Hand evaluation, pot management, player actions
- Event logging and hand history generation
- Dependencies: `rand`, `rand_chacha`, `serde`, `thiserror`, `chrono`

**axiomind_cli** (`rust/cli/`): Command-line interface (binary: `axiomind`)
- Gameplay, simulation, replay, statistics, verification
- File operations for JSONL and SQLite
- Dependencies: `clap` (derive), `rusqlite`, `zstd`, engine crate

**axiomind_web** (`rust/web/`): HTTP server (binary: `axiomind-web-server`)
- Local web server with static file serving
- REST API for game sessions
- SSE (Server-Sent Events) for real-time game updates
- Dependencies: `warp`, `tokio`, `tracing`, engine crate

## Key Libraries

- **Serialization**: `serde` + `serde_json` (all crates)
- **Error Handling**: `thiserror` for domain-specific errors
- **CLI Parsing**: `clap` with derive macros
- **HTTP Server**: `warp` 0.3 with async/await
- **Async Runtime**: `tokio` (multi-threaded)
- **Frontend**: `htmx` 1.9.12 (vendored), no bundler required

## Development Standards

### Type Safety
- Rust strict mode (standard compiler settings)
- No unsafe code unless explicitly justified
- Explicit error types with `thiserror`

### Code Quality
- **Rust**: `rustfmt` and `clippy` enforced (zero warnings in CI)
- **JavaScript**: ESLint (recommended rules) with syntax validation
- **Commits**: Conventional Commits format (feat, fix, docs, refactor, test)

### Testing
- **Unit Tests**: Inline with modules (`#[cfg(test)]`), target 80%+ coverage
- **Integration Tests**: HTTP API validation in `rust/*/tests/`
- **E2E Tests**: Playwright for browser testing (critical: validates frontend actually works)
- **Philosophy**: Backend tests validate logic, E2E tests validate user experience

## Frontend Technologies

### Static Architecture (No Build Step)
- **HTML**: Semantic, htmx attributes for interactivity
- **CSS**: Plain CSS in `rust/web/static/css/`
- **JavaScript**: ES2021+ modules in `rust/web/static/js/`

### htmx Integration
- JSON encoding via `htmx.org/dist/ext/json-enc.js`
- Forms send `application/json` (not form-encoded)
- Server responds with HTML fragments for DOM updates
- SSE for real-time event streams

### Browser Support
- Modern browsers (Chrome/Firefox/Safari last 2 versions)
- No IE support, no polyfills required

## Development Environment

### Required Tools
- Rust stable (latest)
- Node.js 18+ and npm 9+ (for ESLint and Playwright)
- Playwright browsers (installed via `npx playwright install`)

### Common Commands
```bash
# Rust Development
cargo build --release              # Build all crates
cargo run -p axiomind_cli -- play       # Run CLI
cargo run -p axiomind_web               # Start web server

# Testing
cargo test --workspace             # Rust tests
npm run lint                       # ESLint
npm run test:e2e                   # Playwright E2E

# Code Quality
cargo fmt                          # Format Rust
cargo clippy -- -D warnings        # Lint Rust
npm run lint:fix                   # Auto-fix JS
```

## Data Technologies

### Hand History (JSONL)
- One hand per line, UTF-8, LF line endings
- Full state capture: actions, board, showdown, results
- Deterministic replay via seed storage
- Location: `data/hands/`

### Aggregation (SQLite)
- Game statistics, win rates, hand counts
- Location: `data/db.sqlite`
- Bundled via `rusqlite` (no external SQLite required)

## Key Technical Decisions

### Why Rust for Core Engine?
- **Correctness**: Strong type system prevents state inconsistencies
- **Performance**: Fast simulations for large-scale training
- **Determinism**: ChaCha8 RNG ensures reproducible results

### Why Static HTML + htmx (No React/SPA)?
- **Simplicity**: No build step, no dependency management
- **Server Control**: Business logic stays in Rust, not duplicated in JS
- **Reliability**: Backend tests validate actual response HTML
- **Learning**: Documented incident (2025-11-02) showed passing Rust tests don't mean frontend works; E2E tests are mandatory

### Why File-Based AI Integration?
- **Decoupling**: Python AI development independent of Rust engine
- **Flexibility**: Easy to swap training algorithms or frameworks
- **Evolution Path**: Start with files, add gRPC later if needed

### Why JSONL (Not Database)?
- **Append-Only**: Efficient for high-throughput simulations
- **Portable**: Easy to process with any language
- **Auditable**: Human-readable, version-control friendly
- **Streaming**: Can process hands without loading entire file

---

**Stack Philosophy**: Choose boring technology. Rust for correctness, static HTML for simplicity, files for flexibility. Optimize for maintainability and reproducibility over cutting-edge frameworks.
