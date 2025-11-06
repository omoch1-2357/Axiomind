# axm_web

Local HTTP server for Axiomind poker game with interactive web UI. Provides real-time poker gameplay through Server-Sent Events (SSE) and htmx-based interface.

## Overview

`axm_web` is a Rust web server library that integrates with the `axm-engine` to provide:

- **Interactive Web UI**: Play poker through a modern browser interface with real-time updates
- **Session Management**: Create and manage multiple concurrent game sessions
- **Real-time Events**: Server-Sent Events (SSE) for live game state updates
- **RESTful API**: HTTP endpoints for game control, hand history, and statistics
- **AI Opponents**: Built-in AI opponents with configurable difficulty levels

## Features

- **Real-time Game Updates**: SSE streaming keeps UI synchronized with game state
- **Hand History**: View and filter past hands with detailed action sequences
- **Statistics Dashboard**: Win rates, pot sizes, and performance metrics
- **Configuration Management**: Adjust blind levels, AI difficulty, and game parameters
- **Comprehensive Logging**: Structured logging with tracing for debugging and monitoring
- **Error Handling**: Graceful error handling with detailed error messages

## Architecture

The server follows a layered architecture:

1. **HTTP Layer**: Warp-based routing and request handling
2. **Session Layer**: Game session lifecycle and state management
3. **Event Layer**: Real-time event broadcasting via SSE
4. **Engine Integration**: Seamless integration with `axm-engine`
5. **Storage Layer**: Hand history and statistics persistence

## Setup

### Prerequisites

- Rust 1.70+ (stable toolchain)
- Cargo workspace configured with `rust/web` member

### Building

```bash
# Build the web library
cargo build -p axm_web

# Build in release mode for production
cargo build -p axm_web --release
```

### Running Tests

```bash
# Run all tests
cargo test -p axm_web

# Run specific test category
cargo test -p axm_web --lib           # Unit tests
cargo test -p axm_web --test '*'      # Integration tests

# Run with logging output
cargo test -p axm_web -- --nocapture
```

### Starting the Server

The web server is typically started through the CLI:

```bash
# Start server on default port (8080)
cargo run -p axm_cli -- serve

# Specify custom port
cargo run -p axm_cli -- serve --port 3000

# Specify host and static directory
cargo run -p axm_cli -- serve --host 0.0.0.0 --port 8080 --static-dir ./static
```

## Usage

### Creating a Game Session

```rust
use axm_web::{ServerConfig, WebServer, AppContext};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure server
    let config = ServerConfig::new("127.0.0.1", 8080, PathBuf::from("./static"));

    // Create application context
    let ctx = AppContext::new(config)?;

    // Start server
    let server = WebServer::new(ctx);
    let handle = server.spawn()?;

    println!("Server running on http://127.0.0.1:8080");

    // Wait for server to complete
    handle.await_shutdown().await;

    Ok(())
}
```

### API Usage Examples

See [API.md](API.md) for complete endpoint documentation.

**Create a new game session:**

```bash
curl -X POST http://localhost:8080/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"level": 1, "opponent_type": {"AI": "Baseline"}}'
```

**Submit player action:**

```bash
curl -X POST http://localhost:8080/api/sessions/{session_id}/actions \
  -H "Content-Type: application/json" \
  -d '{"action": {"Call": null}}'
```

**Stream game events:**

```bash
curl -N http://localhost:8080/api/sessions/{session_id}/events
```

## Configuration

The server accepts configuration through `ServerConfig`:

```rust
pub struct ServerConfig {
    host: String,        // Bind address (default: "127.0.0.1")
    port: u16,          // Port number (default: 8080)
    static_dir: PathBuf, // Static files directory
}
```

### Environment Variables

- `RUST_LOG`: Logging level (e.g., `info`, `debug`, `axm_web=debug`)
- `AXM_WEB_HOST`: Override default host
- `AXM_WEB_PORT`: Override default port

## Project Structure

```
rust/web/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── server.rs           # HTTP server and routing
│   ├── session.rs          # Session management
│   ├── events.rs           # SSE event bus
│   ├── handlers/           # API endpoint handlers
│   │   ├── game.rs         # Game session endpoints
│   │   ├── sse.rs          # SSE streaming
│   │   ├── history.rs      # Hand history endpoints
│   │   └── settings.rs     # Configuration endpoints
│   ├── static_handler.rs   # Static file serving
│   ├── ai.rs               # AI opponent implementation
│   ├── history.rs          # Hand history storage
│   ├── settings.rs         # Settings management
│   ├── errors.rs           # Error types
│   ├── logging.rs          # Logging configuration
│   ├── metrics.rs          # Performance metrics
│   └── middleware.rs       # HTTP middleware
├── tests/                  # Integration tests
├── README.md              # This file
├── API.md                 # API documentation
├── DEPLOYMENT.md          # Deployment guide
├── TROUBLESHOOTING.md     # Common issues and solutions
└── Cargo.toml            # Package configuration
```

## Key Components

### Session Manager

Manages game session lifecycle:

- Session creation with configurable parameters
- Action processing and state updates
- Session cleanup and timeout handling
- Thread-safe concurrent access

### Event Bus

Broadcasts game events to connected clients:

- SSE connection management
- Event serialization and streaming
- Subscription lifecycle handling
- Automatic cleanup on disconnect

### History Store

Persists and retrieves hand history:

- In-memory storage with filtering support
- Statistics calculation (win rates, pot sizes)
- Efficient lookup by hand ID or time range

### Settings Store

Manages application settings:

- Default blind levels
- AI difficulty configuration
- Session timeout parameters
- Persistent storage (in-memory for now)

## Testing

The project includes comprehensive tests:

- **Unit Tests**: Module-level logic testing
- **Integration Tests**: End-to-end API testing
- **Performance Tests**: Concurrent session handling
- **Documentation Tests**: Validation of required documentation

Run all tests with coverage:

```bash
cargo test -p axm_web --workspace --all-features
```

## Performance

Typical performance characteristics:

- **Concurrent Sessions**: Handles 100+ simultaneous games
- **Event Latency**: <10ms from engine event to SSE delivery
- **Memory Usage**: ~50MB baseline + ~1MB per active session
- **Static Assets**: Efficient serving with proper caching headers

## Security Considerations

- **Input Validation**: All API inputs are validated
- **Error Handling**: No sensitive information leaked in error messages
- **CORS**: Configured for local development (can be restricted for production)
- **Rate Limiting**: Not implemented yet (recommended for production)

## Dependencies

Key dependencies:

- `warp 0.3`: Web framework with filter-based routing
- `tokio 1.x`: Async runtime
- `serde/serde_json`: Serialization
- `axm-engine`: Game logic engine
- `uuid`: Session ID generation
- `tracing`: Structured logging

See [Cargo.toml](Cargo.toml) for complete dependency list.

## Contributing

When contributing to `axm_web`:

1. Follow TDD methodology: write tests first
2. Ensure all tests pass: `cargo test -p axm_web`
3. Run clippy: `cargo clippy -p axm_web -- -D warnings`
4. Format code: `cargo fmt --all`
5. Update documentation as needed

## Troubleshooting

For common issues and solutions, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## License

Part of the Axiomind project. See main repository for license information.

## Related Documentation

- [API.md](API.md) - Complete API endpoint reference
- [DEPLOYMENT.md](DEPLOYMENT.md) - Production deployment guide
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues and solutions
- [../cli/README.md](../cli/README.md) - CLI tool documentation
- [../../docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md) - System architecture
