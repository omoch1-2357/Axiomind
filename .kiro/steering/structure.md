# Project Structure

## Organization Philosophy

**Domain-Driven Separation**: Three independent crates (engine, cli, web) with clear boundaries. Engine owns game logic, CLI and web are thin interfaces. Communication via engine APIs and data files (JSONL, SQLite), not tight coupling.

**Documentation-First**: Every architectural decision documented in `docs/`. Decision records capture rationale for future reference.

**Test Adjacency**: Tests live close to code (inline for units, `tests/` for integration/E2E).

## Directory Patterns

### Rust Workspace (`rust/`)

**Location**: `/rust/engine/`, `/rust/cli/`, `/rust/web/`
**Purpose**: Cargo workspace members, each a separate crate with lib + optional binary
**Example**:
```
rust/engine/
├── Cargo.toml            # Dependencies: rand, serde
├── src/
│   ├── lib.rs            # Public API
│   ├── game.rs           # State machine
│   ├── rules.rs          # Poker rules
│   ├── cards.rs, deck.rs # Domain types
│   └── errors.rs         # Error types
└── tests/                # Integration tests (if needed)
```

**Pattern**: Each crate has `lib.rs` for the library and optional `src/bin/*.rs` or `src/main.rs` for executables. Engine is library-only; CLI and web have binaries.

### Documentation (`docs/`)

**Location**: `/docs/`
**Purpose**: Architecture decisions, runbooks, testing strategy, guidelines
**Example**: `ARCHITECTURE.md`, `STACK.md`, `TESTING.md`, `FRONTEND_GUIDELINES.md`, `CLI.md`, `decisions/`

**Pattern**: Each major decision or subsystem gets a markdown file. Incident reports in `incidents/`. Decision records in `decisions/`.

### Frontend Static Assets (`rust/web/static/`)

**Location**: `/rust/web/static/`
**Purpose**: Static HTML, CSS, JavaScript served by web server
**Example**:
```
rust/web/static/
├── index.html            # Entry point
├── css/
│   └── app.css          # Styles
└── js/
    └── game.js          # UI logic (htmx integration)
```

**Pattern**: No build step. Files served as-is. htmx loaded from CDN or vendored. JavaScript modules use ES2021+ syntax.

### E2E Tests (`tests/e2e/`)

**Location**: `/tests/e2e/`
**Purpose**: Playwright browser tests for complete user flows
**Example**:
```
tests/e2e/
├── game-flow.spec.js         # Start game, make moves
├── sse-events.spec.js        # Real-time updates
├── error-handling.spec.js    # API errors
└── betting-controls.spec.js  # User input validation
```

**Pattern**: One spec file per user flow or feature. Each test validates Content-Type headers, console errors, and DOM updates.

### Data Directories (`data/`)

**Location**: `/data/` (gitignored)
**Purpose**: Runtime-generated hand histories and statistics
**Example**:
```
data/
├── hands/                # JSONL hand histories
│   └── 20250829-*.jsonl
└── db.sqlite             # Aggregated statistics
```

**Pattern**: Append-only JSONL files, one hand per line. SQLite for queries. Not checked into version control.

### Configuration

**Location**: Root directory (`.eslintrc.json`, `playwright.config.js`, `Cargo.toml`)
**Purpose**: Tool configuration at workspace level
**Example**:
- `Cargo.toml`: Workspace definition, member crates
- `.eslintrc.json`: JavaScript linting rules
- `playwright.config.js`: E2E test configuration

## Naming Conventions

### Files
- **Rust**: `snake_case.rs` (e.g., `game.rs`, `hand_evaluator.rs`)
- **Markdown**: `SCREAMING_CASE.md` for docs (e.g., `ARCHITECTURE.md`, `TESTING.md`)
- **JavaScript**: `kebab-case.js` (e.g., `game-logic.js`) or `camelCase.js` (e.g., `game.js`)
- **HTML/CSS**: `kebab-case` (e.g., `index.html`, `app.css`)

### Crates & Binaries
- **Crate names**: `axiomind-engine`, `axiomind_cli`, `axiomind_web` (hyphen in Cargo.toml, underscore in code)
- **Binary name**: `axiomind` (CLI), `axiomind-web-server` (web server)
- **Library names**: `axiomind_engine`, `axiomind_cli`, `axiomind_web` (underscore for imports)

### Rust Code
- **Modules**: `snake_case` (e.g., `mod hand_evaluator;`)
- **Types**: `PascalCase` (e.g., `struct GameState`, `enum Action`)
- **Functions**: `snake_case` (e.g., `fn deal_cards()`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `const MAX_PLAYERS: usize = 2;`)

### JavaScript
- **Functions**: `camelCase` (e.g., `function startGame()`)
- **Classes**: `PascalCase` (e.g., `class GameUI`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `const API_BASE_URL`)

## Import Organization

### Rust
```rust
// Standard library (alphabetical)
use std::collections::HashMap;
use std::io::Write;

// External crates (alphabetical)
use rand::Rng;
use serde::{Deserialize, Serialize};

// Internal crates (alphabetical)
use axiomind_engine::{GameState, Action};

// Current crate modules (alphabetical)
use crate::config::Config;
use crate::ui::render;
```

**No path aliases**: Rust uses `crate::` for current crate, explicit crate names for dependencies.

### JavaScript
```javascript
// External libraries (CDN-loaded, accessed as globals)
// htmx loaded in HTML, available as window.htmx

// No module imports (no bundler)
// Code in <script> tags or standalone .js files
```

**Pattern**: Vanilla JavaScript, no imports. Libraries loaded via `<script>` tags.

## Code Organization Principles

### Rust Crate Boundaries

**Engine**: Pure game logic, no I/O
- Exports: `GameState`, `Action`, `HandResult`, `evaluate_hand()`
- Does NOT: Read files, make HTTP requests, render UI
- Tests: Inline unit tests (`#[cfg(test)]`)

**CLI**: I/O and user interaction
- Depends on: `axiomind-engine`
- Responsibilities: Parse args, read/write files, print output
- Tests: Integration tests in `tests/` for subcommands

**Web**: HTTP server and static assets
- Depends on: `axiomind-engine`
- Responsibilities: Route HTTP requests, manage sessions, serve static files, stream SSE
- Tests: Integration tests for API endpoints, E2E tests for UI

### Dependency Rules

1. **Engine is dependency-free** (except utility crates: rand, serde)
2. **CLI and web depend on engine** (never the reverse)
3. **No circular dependencies** between crates
4. **Data flows through files** (engine → JSONL → CLI/web)

### Testing Layers

**Unit Tests** (inline): Test functions and methods in isolation
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hand_evaluation() {
        // ...
    }
}
```

**Integration Tests** (`rust/*/tests/`): Test crate APIs and HTTP endpoints
```rust
// tests/game_flow.rs
use axiomind_engine::GameState;

#[test]
fn test_complete_hand() {
    // ...
}
```

**E2E Tests** (`tests/e2e/`): Test user flows in real browser
```javascript
// tests/e2e/game-flow.spec.js
test('start game and make bet', async ({ page }) => {
    await page.goto('http://localhost:8080');
    // ...
});
```

## File Layout Examples

### Adding a New Engine Feature
```
rust/engine/src/
├── lib.rs                    # Add pub mod new_feature;
├── new_feature.rs            # Implementation with inline tests
└── errors.rs                 # Add NewFeatureError if needed
```

### Adding a New CLI Subcommand
```
rust/cli/src/
├── main.rs                   # Add subcommand to clap enum
└── subcommands/
    └── new_command.rs        # Command implementation
```

### Adding a Web API Endpoint
```
rust/web/src/
├── handlers/
│   └── new_endpoint.rs       # Handler function
├── server.rs                 # Add route
└── tests/
    └── test_new_endpoint.rs  # Integration test
```

### Adding a Frontend Feature
```
rust/web/static/
├── index.html                # Add UI elements with htmx
└── js/
    └── game.js               # Add event handlers

tests/e2e/
└── new-feature.spec.js       # Playwright E2E test (MANDATORY)
```

## Configuration Files Location

- **Workspace**: `/Cargo.toml` (workspace members)
- **Linting**: `/.eslintrc.json` (JavaScript)
- **Testing**: `/playwright.config.js` (E2E), inline for Rust
- **Git**: `/.gitignore`, `/.gitattributes`
- **CI**: `/.github/workflows/*.yml`

## Development Workflow Directories

### Not in Version Control
- `/target/` - Rust build artifacts
- `/node_modules/` - npm dependencies (dev only)
- `/data/` - Runtime data (JSONL, SQLite)
- `/tmp/` - Temporary working files

### Version Controlled
- `/docs/` - Documentation and decision records
- `/rust/` - Source code
- `/tests/` - E2E and validation tests
- `/scripts/` - Automation scripts

---

**Structural Principle**: Separation of concerns via crates, documentation via markdown, data via files. New features follow existing crate boundaries; new crates require strong justification.
