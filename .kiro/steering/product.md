# Product Overview

Axiomind is a Heads-Up No-Limit Texas Hold'em poker system designed for offline self-learning AI research. It provides a complete game engine, simulation tools, web interface, and data infrastructure for poker AI experimentation.

## Core Capabilities

- **Game Engine**: Complete poker rule implementation with deterministic state transitions, hand evaluation, and event logging
- **Interactive Play**: CLI-based gameplay against AI opponents or human players, with replay and analysis tools
- **Web Interface**: Real-time poker UI with htmx-powered interactions and Server-Sent Events for live game updates
- **Data Collection**: Structured hand history logging (JSONL) and aggregation (SQLite) for training and analysis
- **AI Training Platform**: Infrastructure for training and evaluating poker agents (Python integration planned)

## Target Use Cases

### Research & Development
- Self-play training: Generate large datasets through automated simulations
- Policy evaluation: Compare different AI strategies across thousands of hands
- Rule verification: Validate poker rules, conservation laws, and game state integrity

### Interactive Testing
- Manual gameplay: Test AI behavior against human opponents via CLI or web UI
- Hand replay: Analyze specific scenarios and decision points
- Real-time monitoring: Observe AI decision-making through event streams

### Data Analysis
- Hand history analysis: Extract patterns from JSONL logs
- Performance metrics: Aggregate statistics and win rates via SQLite
- Export capabilities: Convert data for external analysis tools

## Value Proposition

**Offline-First**: No external dependencies, complete local execution for reproducible research

**Data-Driven**: Every hand is logged with full state history, enabling comprehensive analysis and training

**Modular Architecture**: Clean separation between game logic (engine), user interfaces (CLI/web), and AI components (Python) allows independent development and testing

**Production-Ready Testing**: Comprehensive E2E testing strategy ensures UI changes are validated in real browsers, not just backend tests

---

**Focus**: This product enables poker AI research through reproducible simulations, structured data collection, and flexible interfaces. It prioritizes correctness, data integrity, and offline reproducibility over real-time multiplayer or monetization features.
