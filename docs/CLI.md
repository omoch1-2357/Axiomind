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

