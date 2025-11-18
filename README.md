# Axiomind - Heads-Up No-Limit Hold'em

A comprehensive Heads-Up No-Limit Texas Hold'em poker system built in Rust for AI research and self-learning. Designed for offline operation with complete game engine, CLI tools, web interface, and AI framework.

## Features

- **Complete Game Engine**: Full Texas Hold'em rules, state transitions, hand evaluation, and event system
- **Command-Line Interface**: Play, simulate, analyze, and benchmark poker games
- **Web Interface**: Interactive browser-based UI with real-time game updates via Server-Sent Events (SSE)
- **AI Framework**: Built-in AI opponents with configurable difficulty levels (1-20)
- **Data Persistence**: JSONL hand history and SQLite aggregated statistics
- **Comprehensive Testing**: Full test coverage with verification tools

## Quick Start

### Prerequisites

- Rust 1.70+ (stable toolchain)
- Cargo (comes with Rust)

### Installation

```bash
# Clone the repository
git clone https://github.com/omoch1-2357/Axiomind.git
cd Axiomind

# Build all components
cargo build --release

# Run tests to verify installation
cargo test
```

### Usage

#### Play a Game (CLI)

```bash
# Play 10 hands against AI at level 2
cargo run -p axiomind_cli --bin axiomind -- play --vs ai --hands 10 --level 2
```

#### Start Web Server

```bash
# Start the web server on default port 8080
cargo run -p axiomind_web --bin axiomind-web-server

# Or with custom port
cargo run -p axiomind_web --bin axiomind-web-server --port 3000

# Then open your browser to http://localhost:8080
```

#### Simulate Games

```bash
# Run 1000 hands simulation
cargo run -p axiomind_cli --bin axiomind -- sim --hands 1000 --level 5 --output data/sim.jsonl
```

#### Analyze Statistics

```bash
# Analyze hand history from JSONL files
cargo run -p axiomind_cli --bin axiomind -- stats --input data/hands/
```

## Project Structure

```
Axiomind/
├── rust/
│   ├── engine/     # Core game engine (rules, state transitions, hand evaluation)
│   ├── cli/        # Command-line interface and tools
│   ├── web/        # Web server and HTTP API
│   └── ai/         # AI opponents and learning framework
├── docs/           # Design documentation and ADRs
├── data/           # Hand history logs and statistics
└── tests/          # Integration tests
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `play` | Play poker against AI or human opponent |
| `sim` | Run large-scale game simulations |
| `replay` | Replay hand history from JSONL files |
| `stats` | Analyze and aggregate statistics |
| `eval` | Evaluate and compare AI policies |
| `verify` | Verify game rules and invariants |
| `deal` | Deal and display a single hand |
| `bench` | Benchmark hand evaluation and state transitions |
| `export` | Convert hand history to various formats (CSV, SQLite, etc.) |
| `dataset` | Create and split datasets for training |
| `cfg` | Display and override configuration |
| `doctor` | Run environment diagnostics |
| `rng` | Verify random number generator |

See [`docs/CLI.md`](docs/CLI.md) for detailed command documentation.

## Web API

The web server provides a RESTful API with real-time SSE streaming:

- `POST /api/sessions` - Create new game session
- `GET /api/sessions/{id}/state` - Get current game state
- `POST /api/sessions/{id}/actions` - Submit player action
- `GET /api/sessions/{id}/events` - Stream game events (SSE)
- `GET /api/history` - Retrieve hand history
- `GET /api/settings` - Get/update server settings
- `GET /health` - Health check endpoint

See [`rust/web/API.md`](rust/web/API.md) for complete API documentation.

## Data Format

### Hand History (JSONL)

All hands are logged in JSON Lines format with complete game information:

```json
{
  "hand_id": "20250829-000001",
  "seed": 42,
  "level": 3,
  "sb": 100,
  "bb": 200,
  "button": "P2",
  "players": [
    {"id": "P1", "stack_start": 20000},
    {"id": "P2", "stack_start": 20000}
  ],
  "actions": [...],
  "board": ["Ah", "Kd", "7c", "2s", "2d"],
  "showdown": [...],
  "net_result": {"P1": -500, "P2": 500},
  "end_reason": "showdown",
  "ts": "2025-08-29T00:00:00Z"
}
```

### Blind Structure

20 levels of blinds (level 21+ treated as level 20):

| Level | Small Blind | Big Blind |
|-------|-------------|-----------|
| 1 | 50 | 100 |
| 2 | 75 | 150 |
| 3 | 100 | 200 |
| ... | ... | ... |
| 20 | 4000 | 8000 |

See [`docs/GAME_RULES.md`](docs/GAME_RULES.md) for complete game rules.

## Development

### Building

```bash
# Build all packages
cargo build --workspace

# Build specific package
cargo build -p axiomind_engine
cargo build -p axiomind_cli
cargo build -p axiomind_web

# Release build
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for specific package
cargo test -p axiomind_engine
cargo test -p axiomind_cli
cargo test -p axiomind_web

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets -- -D warnings

# Check for security issues
cargo deny check
```

## Documentation

- [`docs/README.md`](docs/README.md) - Japanese repository overview
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) - System architecture
- [`docs/STACK.md`](docs/STACK.md) - Technology stack
- [`docs/GAME_RULES.md`](docs/GAME_RULES.md) - Poker rules and blind structure
- [`docs/CLI.md`](docs/CLI.md) - CLI command reference
- [`rust/web/README.md`](rust/web/README.md) - Web server documentation
- [`rust/web/API.md`](rust/web/API.md) - API reference
- [`rust/web/DEPLOYMENT.md`](rust/web/DEPLOYMENT.md) - Deployment guide
- [`rust/web/TROUBLESHOOTING.md`](rust/web/TROUBLESHOOTING.md) - Troubleshooting guide

## Contributing

This project follows standard Rust development practices:

1. **Branching**: Work on feature branches, keep `main` clean
2. **Commits**: Follow [Conventional Commits](https://www.conventionalcommits.org/)
3. **Testing**: All new code must include tests
4. **Quality**: Code must pass `cargo fmt`, `cargo clippy`, and `cargo test`

## License

See repository for license information.

## Support

For issues, questions, or contributions, please use the project's GitHub repository.
