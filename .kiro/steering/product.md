# Product Overview

Axiomind is a poker game engine and AI training platform focused on head-to-head (HU) Texas Hold'em. It provides deterministic game simulation, comprehensive hand history storage, and infrastructure for training and evaluating poker AI agents.

## Core Capabilities

- **Deterministic Simulation**: Reproducible game outcomes via seeded RNG (ChaCha8), enabling scientific comparison and debugging
- **Comprehensive Logging**: Full hand histories in structured JSONL format with every action, decision point, and outcome preserved
- **Multi-Interface Access**: CLI for batch operations, web UI for interactive play, file-based AI integration (future: gRPC)
- **Verification & Validation**: Built-in tools to verify game rules, conservation laws, and RNG properties
- **AI Training Infrastructure**: Dataset generation, hand replay, and policy evaluation tools

## Target Use Cases

### Research & Development
- Poker AI algorithm development and comparative evaluation
- Game theory experimentation with reproducible results
- Large-scale simulation studies (millions of hands)

### Training & Analysis
- Generate training datasets from baseline/opponent policies
- Replay and analyze specific game scenarios
- Benchmark AI performance head-to-head

### Interactive Play
- Test AI agents against human players
- Real-time game streaming via web UI
- Inspect individual hands and decision points

## Value Proposition

**Reproducibility First**: Every game outcome is deterministic given a seed, enabling scientific comparison of AI strategies and debugging edge cases that would be impossible with non-deterministic systems.

**Offline & Local**: No external dependencies or cloud services required. All data stays on disk (JSONL + SQLite), enabling privacy-sensitive research and offline operation.

**Separation of Concerns**: Engine handles pure game logic, CLI provides batch operations, web server manages UI streaming, AI agents integrate via files or gRPC. Each component can evolve independently.

---
_Generated: 2025-11-02_
