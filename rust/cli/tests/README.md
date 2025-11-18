# CLI Test Suite Documentation

このディレクトリには、Axiomind CLI (`axiomind`) の包括的なテストスイートが含まれています。

## テスト構成

### 統合テスト (`integration.rs`)
統合テストは、CLIの完全なエンドツーエンドの動作を検証します。各テストカテゴリは仕様書のシリーズ記号（A/B/C...）に対応しています。

#### カテゴリ
- **cli_basic** (Aシリーズ): 基本コマンドと引数エラー処理
- **config_precedence**: 設定の優先順位（CLI > 環境変数 > 設定ファイル > デフォルト）
- **simulation_basic** (Dシリーズ): シミュレーション実行とレジューム機能
- **evaluation_basic** (Eシリーズ): AI評価コマンド
- **file_io_basic, file_corruption_recovery, file_dir_processing** (C/Fシリーズ): ファイル入出力と統計
- **game_logic** (B/J/L/Mシリーズ): ゲームルール検証、診断、状態管理
- **performance_stress** (14.2): パフォーマンステスト
- **data_format** (Kシリーズ): JSONL/SQLite のバリデーション

### ユニットテスト
- **test_ui.rs**: UIヘルパー関数のテスト
- **test_game_state.rs**: ゲーム状態管理のテスト
- **test_sim_resume.rs**: シミュレーションのレジューム機能のテスト

### ヘルパーモジュール (`helpers/`)
テストで使用される共通ユーティリティ：
- **cli_runner.rs**: CLIコマンドの実行と結果のキャプチャ
- **temp_files.rs**: テスト用一時ファイル管理
- **assertions.rs**: ポーカー固有のアサーション関数

## テストの実行

### すべてのテストを実行
```powershell
cargo test -p axiomind_cli
```

### 統合テストのみ実行
```powershell
cargo test -p axiomind_cli --test integration
```

### 特定のテストを実行
```powershell
# 単一テストの実行例
cargo test -p axiomind_cli --test integration cli_basic::a1_version_exits_with_zero

# カテゴリ全体の実行例
cargo test -p axiomind_cli --test integration cli_basic::
```

### 警告なしで静かに実行
```powershell
cargo test -p axiomind_cli -q
```

### 無視されたテストも含めて実行
```powershell
# パフォーマンステストを含む全テスト
cargo test -p axiomind_cli -- --include-ignored
```

## 環境変数

テストの動作を制御する環境変数：

### シミュレーション制御
- `axiomind_SIM_FAST=1`: 高速シミュレーションモードを有効化
- `axiomind_SIM_SLEEP_MICROS=<num>`: ハンドごとの遅延（マイクロ秒）
- `axiomind_SIM_BREAK_AFTER=<num>`: 指定ハンド数後に中断（レジュームテスト用）

### データセット処理
- `axiomind_DATASET_STREAM_THRESHOLD=<num>`: ストリーミングモードの閾値
- `axiomind_DATASET_STREAM_TRACE=1`: ストリーミングのトレースを有効化

### 設定テスト
- `axiomind_CONFIG=<path>`: 設定ファイルのパス
- `axiomind_SEED=<num>`: シード値
- `axiomind_LEVEL=<num>`: レベル値
- `axiomind_AI_VERSION=<version>`: AIバージョン
- `axiomind_ADAPTIVE=<bool>`: アダプティブモード

### 診断テスト
- `axiomind_DOCTOR_SQLITE_DIR=<path>`: SQLite書き込み権限チェック用ディレクトリ
- `axiomind_DOCTOR_DATA_DIR=<path>`: データディレクトリアクセステスト用
- `axiomind_DOCTOR_LOCALE_OVERRIDE=<locale>`: ロケール設定の上書き

### 入力テスト
- `axiomind_TEST_INPUT=<input>`: テスト用の標準入力
- `axiomind_NON_TTY=1`: TTYなし状況のシミュレーション

## テストの追加

新しいテストを追加する際のガイドライン：

1. **適切なカテゴリに配置**: 既存のカテゴリ構造に従う
2. **明確なテスト名**: テストの目的が名前から分かるように
3. **コメントで仕様参照**: 対応する仕様書のセクションを記載
4. **環境変数のクリーンアップ**: テスト後は環境変数を適切にクリーンアップ
5. **一時ファイルの管理**: `TempFileManager`を使用して自動クリーンアップ

### テスト例
```rust
#[test]
fn test_example() {
    let cli = CliRunner::new().expect("cli runner");
    let result = cli.run_with_env(
        &["command", "--flag", "value"],
        &[("ENV_VAR", "value")]
    );
    
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("expected output"));
}
```

## トラブルシューティング

### パフォーマンステストの失敗
パフォーマンステスト（`p1_sim_large_run_under_budget`など）は環境依存のため、デフォルトで無視されています。これらのテストは実行時間の予算を持っており、遅いマシンでは失敗する可能性があります。

### 並行実行の問題
一部のテスト（特に`doctor`コマンド関連）は、環境変数やファイルシステムの状態を変更するため、並行実行で競合する可能性があります。`DOCTOR_LOCK` mutexを使用して同期化しています。

### 一時ファイルのクリーンアップ
`TempFileManager`は`Drop`トレイトでクリーンアップしますが、パニック時には残る可能性があります。`target/ds_*`ディレクトリを手動で削除できます。

## テストカバレッジ

このテストスイートは以下の要件をカバーしています：

- ✅ 全CLIコマンドの基本動作
- ✅ エラーハンドリングと入力バリデーション
- ✅ ファイルI/Oと破損データの回復
- ✅ ゲームルールの検証
- ✅ 設定管理と優先順位
- ✅ シミュレーションとレジューム機能
- ✅ データフォーマットのバリデーション
- ✅ パフォーマンスとストレステスト

詳細は `.kiro/specs/comprehensive-cli-testing/tasks.md` を参照してください。
