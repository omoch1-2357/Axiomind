# CLI

バイナリ名 `axm`

## 共通オプション
- `--seed <u64>` 乱数シード 既定なし
- `--ai-version <id>` AI のモデルバージョン 既定 latest
- `--adaptive <on|off>` AI のリアルタイム適応 既定 on

## コマンド

| Command | Description | Options | Implementation Status |
|---------|-------------|---------|----------------------|
| `play` | 対戦を実行 | `--vs ai\|human --hands <N> --level <L>` | PARTIAL - AI opponent is placeholder (always checks, demo mode only) |
| `replay` | ハンド履歴を再生 | `--input <path>` | PARTIAL - Count only, full visual replay not implemented |
| `sim` | 大量対戦シミュレーション | `--hands <N> --ai <name>` | IMPLEMENTED |
| `eval` | ポリシー評価 | `--ai-a <name> --ai-b <name> --hands <N>` | PARTIAL - Random placeholder results, AI parameters not used |
| `stats` | JSONL から集計 | `--input <file\|dir>` | IMPLEMENTED |
| `verify` | ルールと保存則の検証 | | IMPLEMENTED |
| `serve` | ローカル UI サーバを起動 | `--open --port <n>` | PLANNED - Not available in CLI |
| `deal` | 1 ハンドだけ配って表示 | | IMPLEMENTED |
| `bench` | 役判定や状態遷移のベンチマーク | | IMPLEMENTED |
| `rng` | 乱数の検証 | | IMPLEMENTED |
| `cfg` | 既定設定の表示と上書き | | IMPLEMENTED |
| `doctor` | 環境診断 | | IMPLEMENTED |
| `export` | 形式変換や抽出 | | IMPLEMENTED |
| `dataset` | データセット作成と分割 | | IMPLEMENTED |
| `train` | 学習を起動 | | PLANNED - Not yet implemented |

## Known Limitations and Workarounds

### `serve` command
The web server is not integrated into the CLI. To run the web UI:
```bash
cargo run -p axm_web --bin axm-web-server
```

### `play --vs ai`
The AI opponent is currently a placeholder that always checks. This is for demonstration purposes only.
WARNING: Results from AI play mode should not be used for serious evaluation.

### `replay`
Only counts hands in the file. Full visual replay functionality is planned but not yet implemented.

### `eval`
Returns random results. AI parameters (--ai-a, --ai-b) are currently not used.
For real AI simulations, use the `sim` command instead.

## New Command Implementation Checklist

When adding a new CLI command, complete this checklist before merging:

### 1. Command Definition
- [ ] Command enum variant exists in `Commands` enum (rust/cli/src/lib.rs)
- [ ] Command parameters are defined with clap derive macros
- [ ] Command is added to COMMANDS array ONLY if fully implemented
- [ ] PLANNED commands are NOT added to COMMANDS array

### 2. Implementation
- [ ] Implementation is complete, not a stub or placeholder
- [ ] All command parameters are actually used in the implementation
- [ ] Input validation is implemented for all parameters
- [ ] Error handling covers common failure cases
- [ ] Warning system is used if implementation has limitations

### 3. Testing
- [ ] Behavioral tests verify actual command behavior (not just output format)
- [ ] Tests validate blocking behavior for interactive commands
- [ ] Tests verify parameter usage (parameters affect behavior)
- [ ] Tests cover error paths and edge cases
- [ ] All tests pass: `cargo test --test test_<command_name>`

### 4. Documentation
- [ ] Command added to CLI.md command table with accurate status
- [ ] Implementation status clearly indicated (IMPLEMENTED/PARTIAL/PLANNED)
- [ ] Known limitations documented in "Known Limitations and Workarounds" section
- [ ] Usage examples provided if command has complex parameters

### 5. Quality Assurance
- [ ] Manual testing completed with various inputs
- [ ] Exit codes are correct (0 = success, 2 = error)
- [ ] Error messages are helpful and actionable
- [ ] Warnings go to stderr, data output goes to stdout
- [ ] CI pipeline passes all checks

### Command Status Definitions

- **IMPLEMENTED**: Fully functional, all parameters work as documented
- **PARTIAL**: Command exists but has limitations or placeholder behavior (must show warnings)
- **PLANNED**: Command is designed but not yet implemented (must NOT appear in COMMANDS array)

### Examples

#### Good: Fully Implemented Command
```rust
// Commands enum has variant
#[derive(Subcommand)]
enum Commands {
    Stats { input: String },
    // ...
}

// COMMANDS array includes it
const COMMANDS: &[&str] = &["stats", /* ... */];

// Implementation uses all parameters
fn execute_stats_command(input: &str) -> i32 {
    // Real implementation here
    process_file(input)
}
```

#### Bad: Placeholder Without Warning
```rust
// DON'T DO THIS - placeholder without warning
fn execute_eval_command(ai_a: String, ai_b: String) -> i32 {
    // Parameters ignored, returns random results
    println!("AI A won: {}", rand::random::<u32>());
    0
}
```

#### Good: Placeholder With Warning
```rust
// DO THIS - clear warning about limitations
fn execute_eval_command(ai_a: String, ai_b: String, err: &mut dyn Write) -> i32 {
    display_warning(err, "This is a placeholder returning random results. AI parameters are not used.");
    warn_parameter_unused(err, "ai-a");
    warn_parameter_unused(err, "ai-b");
    // ... placeholder implementation
    0
}
```

#### Good: Planned Command Not in Array
```rust
// Commands enum can have planned variants
#[derive(Subcommand)]
enum Commands {
    Train { /* ... */ },  // Planned, not implemented
    // ...
}

// But COMMANDS array excludes it
const COMMANDS: &[&str] = &[
    "play", "sim", /* ... */
    // "train" is NOT here - won't appear in help text
];
```

