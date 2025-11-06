# RUNBOOK

ローカルとオフラインを前提とする手順。

## セットアップ
- Rust stable を導入 rustup を使用
- Python 3.12 を導入
- 仮想環境を作成
  - `python -m venv .venv`
  - `.\\.venv\\Scripts\\Activate.ps1`
  - `pip install -U pip ruff black`

## データ
- ハンド履歴 data/hands/YYYYMMDD/*.jsonl
- 集計 DB data/db.sqlite
- ログ data/logs/*.log

## ドキュメント生成とホスティング

### ローカルでのドキュメント生成

#### 基本的な使い方

**ワークスペース全体のドキュメント生成**
```bash
# すべてのクレート(axm-engine, axm_cli, axm_web)のドキュメントを生成し、ブラウザで開く
cargo doc --workspace --open

# 外部クレートの依存関係を除外(推奨: ビルド時間短縮)
cargo doc --workspace --no-deps --open
```

生成されたドキュメントは `target/doc/` ディレクトリに保存されます。

**特定のクレートのみ生成**
```bash
# engineクレートのみ
cargo doc -p axm-engine --open

# CLIクレートのみ
cargo doc -p axm_cli --open

# webクレートのみ
cargo doc -p axm_web --open
```

特定のクレートの変更を確認する場合、ビルド時間を大幅に短縮できます。

#### プライベートAPIを含む内部開発用ドキュメント

内部実装の詳細を確認したい場合、プライベート項目を含むドキュメントを生成できます:

```bash
# すべての内部APIを含む(内部開発・デバッグ用)
cargo doc --workspace --document-private-items --open

# 特定クレートのみ、プライベート項目を含む
cargo doc -p axm-engine --document-private-items --open
```

**注意**: このオプションは内部開発用であり、GitHub Pagesには公開されません(公開APIのみ公開)。

#### ドキュメントの変更差分を確認

コードの変更後にドキュメントを再生成すると、変更が自動的に反映されます:

```bash
# 1. ドキュメントコメントを追加・修正
# 2. 再ビルド
cargo doc --workspace --no-deps

# 3. ブラウザをリロードして変更を確認(または--openで再度開く)
cargo doc --workspace --no-deps --open
```

### ドキュメント生成のトラブルシューティング

#### よくあるエラーと解決方法

**1. 壊れたドキュメントリンクエラー**

**エラー例**:
```
error: unresolved link to `NonExistentType`
```

**原因**: ドキュメントコメント内で参照している型やモジュールが存在しないか、パスが間違っています。

**解決方法**:
```rust
// 悪い例: パスが不完全
/// See [Card] for details.

// 良い例: 完全なパスを使用
/// See [`crate::cards::Card`] for details.
/// Or use: [`Card`](crate::cards::Card)

// クレート外への参照(他クレートのドキュメントへリンク)
/// See [`axm_engine::Engine`] for the game engine.
```

**検証コマンド**:
```bash
# リンクエラーを警告として表示
cargo rustdoc --workspace -- -D warnings
```

**2. 未解決のインポートエラー**

**エラー例**:
```
error: unresolved import `crate::internal_module`
```

**原因**: ドキュメント内のコード例で、プライベートモジュールやテスト環境でのみ有効なインポートを使用しています。

**解決方法**:
```rust
// 悪い例: プライベートモジュールへの参照
/// ```
/// use crate::internal_module::PrivateType;
/// ```

// 良い例: 公開APIのみ使用
/// ```
/// use axm_engine::{Engine, GameState};
/// let engine = Engine::new(/* ... */);
/// ```

// または、実行不可能な例には`no_run`属性を使用
/// ```no_run
/// // コンパイルはするが実行しない例
/// let result = some_function_requiring_setup();
/// ```
```

**3. Doctestの失敗**

**エラー例**:
```
---- src/lib.rs - Engine::new (line 42) stdout ----
error[E0425]: cannot find value `config` in this scope
```

**原因**: ドキュメント内のコード例が不完全、または実行時エラーが発生します。

**解決方法**:
```rust
// 悪い例: 不完全な例
/// ```
/// let engine = Engine::new(config);
/// ```

// 良い例: 実行可能な完全な例
/// ```
/// use axm_engine::Engine;
/// let engine = Engine::default();
/// ```

// または、コンパイルのみ確認する場合
/// ```compile_fail
/// // コンパイルエラーになることを示す例
/// let invalid = Engine::new("wrong type");
/// ```

// 実行をスキップする場合
/// ```ignore
/// // 依存関係が複雑で実行困難な例
/// let result = complex_setup_requiring_files();
/// ```
```

**Doctestの実行**:
```bash
# すべてのDoctestを実行
cargo test --workspace --doc

# 特定クレートのみ
cargo test -p axm-engine --doc
```

**4. ビルドが遅い場合の対処法**

**問題**: `cargo doc`の実行に時間がかかる

**解決方法**:
```bash
# 外部依存関係を除外(最も効果的)
cargo doc --workspace --no-deps --open

# 特定のクレートのみビルド
cargo doc -p axm-engine --open

# 増分ビルドを活用(2回目以降は高速)
# Cargoはデフォルトで増分コンパイルを使用
```

**5. 権限エラー(Windows)**

**エラー例**:
```
Access is denied. (os error 5)
```

**原因**: `target/doc/` ディレクトリが他のプロセスでロックされている可能性があります。

**解決方法**:
```bash
# ブラウザを閉じてから再実行
# または、targetディレクトリをクリーン
cargo clean
cargo doc --workspace --open
```

### GitHub Pagesの有効化(初回のみ)
リポジトリでドキュメントを公開するには、以下の手順でGitHub Pages機能を有効化します:

1. リポジトリの Settings > Pages に移動
2. Source を「Deploy from a branch」に設定
3. Branch を「gh-pages」、フォルダを「/ (root)」に設定
4. Save をクリック

設定後、mainブランチへのpushで自動的にドキュメントがビルド・デプロイされます。

### 公開URLへのアクセス
設定完了後、以下のURLでドキュメントにアクセス可能:
- `https://<owner>.github.io/<repo>/`

### GitHub Pagesのトラブルシューティング
- **デプロイが失敗する**: Settings > Actions > General で「Read and write permissions」が有効か確認
- **GitHub Pagesが更新されない**: Actions タブでワークフローが成功しているか確認
- **404エラーが出る**: gh-pagesブランチが作成されているか確認

## トラブルシュート
- 乱数の再現 `--seed` を指定し同一バージョンで再実行
- JSONL の破損 末尾途中行を検出し以降を破棄
- SQLite のロック 単一プロセスで書き込み バッチ化を使用
