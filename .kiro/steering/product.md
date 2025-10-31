# Product Overview

Axiomind is a poker game engine and AI training platform specializing in head-to-head (HU) Texas Hold'em. It serves poker researchers, AI developers, and serious players who need reproducible game outcomes, comprehensive hand histories, and a platform for training and evaluating poker AI agents.

## Core Capabilities

1. **Deterministic Game Engine**: Reproducible poker hands using seed-based RNG (ChaCha), ensuring exact game state reconstruction for debugging, testing, and research
2. **Comprehensive Data Capture**: JSONL-based hand history logging with complete action sequences, board states, and outcomes for AI training and analysis
3. **Multi-Interface Access**: CLI tool for batch simulations and scripting, web UI for interactive play, designed for both human analysis and programmatic control
4. **AI Integration Platform**: File-based AI policy evaluation with planned gRPC support, enabling training pipelines and head-to-head policy comparisons
5. **Verification and Diagnostics**: Built-in rule verification, conservation law checking, and RNG quality testing to ensure game integrity

## Target Use Cases

- **AI Research**: Generate large-scale training datasets with reproducible outcomes, evaluate policy improvements through head-to-head matches
- **Poker Analysis**: Replay specific hands with different decisions, analyze statistical patterns across thousands of hands
- **Development and Testing**: Verify game rule implementations, benchmark performance, ensure deterministic behavior for unit testing
- **Interactive Learning**: Play against AI opponents with real-time feedback, export hand histories for post-game review

## Value Proposition

Axiomind bridges the gap between poker game rules and AI development by providing a **reproducible, verifiable, and observable** poker environment. Unlike typical poker platforms:

- Every hand can be exactly reproduced from a seed value
- Complete action history is captured in machine-readable JSONL format
- Game state integrity is continuously verified through conservation laws
- Multiple interfaces (CLI, web, future gRPC) support diverse workflows from rapid prototyping to production training

The focus on **determinism and observability** makes Axiomind ideal for scientific poker AI research where reproducibility and data quality are paramount.

---
_Focus on patterns and purpose, not exhaustive feature lists_
