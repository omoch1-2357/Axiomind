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

# 詳細な出力でDoctestを実行(デバッグ用)
cargo test --workspace --doc --verbose
```

#### Doctest属性の使い分けガイドライン

Doctestには複数の属性があり、用途に応じて使い分けることで、適切なドキュメントとテストを提供できます。

**1. デフォルト(属性なし): 実行可能なコード例**

最も推奨される形式。コンパイル・実行の両方が成功する必要があります。

```rust
/// Creates a new deck with deterministic shuffling.
///
/// # Examples
///
/// ```
/// use axm_engine::Deck;
/// let deck = Deck::new(42); // シード42で初期化
/// assert_eq!(deck.remaining(), 52);
/// ```
pub fn new(seed: u64) -> Self { /* ... */ }
```

**いつ使うか**:
- APIの基本的な使い方を示す場合
- 単純な例で外部依存がない場合
- 実際に動作することを保証したい場合

**2. `no_run`: コンパイルのみ確認、実行はスキップ**

コードは有効だが、実行には時間がかかる、ファイルI/Oが必要、ネットワーク接続が必要などの場合に使用。

```rust
/// Runs a simulation for the specified number of hands.
///
/// # Examples
///
/// ```no_run
/// use axm_engine::Engine;
///
/// let mut engine = Engine::new(42);
/// // 100万ハンドのシミュレーション(実行には時間がかかるため no_run)
/// for _ in 0..1_000_000 {
///     let result = engine.play_hand();
///     // 結果の処理...
/// }
/// ```
pub fn play_hand(&mut self) -> HandResult { /* ... */ }
```

**いつ使うか**:
- 実行に時間がかかる処理(大量ループ、長時間待機)
- ファイルシステムへの読み書きが必要
- ネットワーク接続が必要
- 外部プロセスの起動が必要
- ユーザー入力を待つ処理

**メリット**:
- コンパイルエラーは検出できる(型チェック、構文チェック)
- CIの実行時間が短縮される
- コード例が実際の使用方法を示せる

**3. `ignore`: コンパイル・実行の両方をスキップ**

コードが完全でない、または意図的に古いバージョンの例を示す場合に使用。**通常は避けるべき**。

```rust
/// Legacy API example (deprecated, use new API instead).
///
/// ```ignore
/// // この例は古いAPIを使用しており、現在はコンパイルできません
/// let old_engine = OldEngine::create();
/// ```
pub fn new_api() -> Self { /* ... */ }
```

**いつ使うか**:
- 擬似コード・概念的な例を示す場合
- 外部ツールのセットアップが複雑で再現困難な場合
- プラットフォーム固有のコードで他環境では動作しない場合

**注意**: `ignore`は最後の手段。可能な限り`no_run`や実行可能な例を優先してください。

**4. `compile_fail`: コンパイルエラーになることを示す**

APIの誤用例を示す場合や、型安全性を説明する場合に使用。

```rust
/// Processes an action. Type-safe: only valid actions are accepted.
///
/// # Examples
///
/// ```compile_fail
/// use axm_engine::{Engine, Action};
///
/// let mut engine = Engine::new(42);
/// // コンパイルエラー: Actionは文字列ではない
/// engine.process_action("invalid");
/// ```
pub fn process_action(&mut self, action: Action) { /* ... */ }
```

**いつ使うか**:
- 型安全性を示す場合
- よくある間違いを説明する場合
- APIの制約を明示する場合

**5. `should_panic`: 実行時にパニックすることを期待**

特定の条件下でパニックすることを示す場合に使用。

```rust
/// Validates that the action is legal. Panics if invalid.
///
/// # Panics
///
/// Panics if the action amount exceeds the player's stack.
///
/// # Examples
///
/// ```should_panic
/// use axm_engine::{Engine, Action};
///
/// let mut engine = Engine::new(42);
/// // パニックする: 所持金を超える額
/// engine.validate_action(Action::Raise(1_000_000));
/// ```
pub fn validate_action(&self, action: Action) { /* ... */ }
```

**いつ使うか**:
- 前提条件違反でパニックする場合
- デバッグアサーションの動作を示す場合

#### Doctest属性の選択フローチャート

```
コード例を書く
  ↓
実行可能か?
  ├─ Yes → 属性なし(デフォルト) ← 最優先
  └─ No
      ↓
      コンパイル可能か?
        ├─ Yes
        │   ↓
        │   実行に時間/リソースが必要か?
        │     ├─ Yes → no_run
        │     └─ No → パニックを期待?
        │               ├─ Yes → should_panic
        │               └─ No → 属性なし(または見直す)
        └─ No
            ↓
            意図的にコンパイルエラーを示す?
              ├─ Yes → compile_fail
              └─ No → ignore (最後の手段)
```

#### Doctestのベストプラクティス

**1. 完全な例を書く**
```rust
// 悪い例: 不完全
/// ```
/// let result = process(data);
/// ```

// 良い例: 完全
/// ```
/// use axm_engine::Engine;
///
/// let engine = Engine::new(42);
/// let result = engine.play_hand();
/// assert!(result.is_ok());
/// ```
```

**2. 必要なインポートを明示**
```rust
// 悪い例: インポート不足
/// ```
/// let deck = Deck::new(42);
/// ```

// 良い例: 完全なインポート
/// ```
/// use axm_engine::Deck;
///
/// let deck = Deck::new(42);
/// ```
```

**3. 実際の使用例を示す**
```rust
// 悪い例: 抽象的すぎる
/// ```no_run
/// // なにかする...
/// do_something();
/// ```

// 良い例: 具体的
/// ```no_run
/// use axm_engine::Engine;
///
/// let mut engine = Engine::new(42);
/// for _ in 0..100 {
///     let result = engine.play_hand();
///     println!("Hand result: {:?}", result);
/// }
/// ```
```

**4. エラーケースも示す**
```rust
/// Loads hand history from a JSONL file.
///
/// # Examples
///
/// ```no_run
/// use axm_cli::load_hands;
///
/// // 成功ケース
/// let hands = load_hands("data/hands/20250101/12-00-00.jsonl")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ```should_panic
/// // エラーケース: ファイルが存在しない
/// let hands = load_hands("nonexistent.jsonl").unwrap();
/// ```
```

#### CI/CDでのDoctest検証

Doctestは既存のCIパイプラインで自動的に検証されます:

**検証内容**:
1. **コンパイル**: すべてのdoctest(no_run含む)が構文的に正しいことを確認
2. **実行**: 属性なし、should_panic属性のdoctestが正常に実行されることを確認
3. **リンク**: ドキュメント内のクロスリンクが壊れていないことを確認

**CIで実行されるコマンド**:
```bash
# .github/workflows/ci.ymlで自動実行
cargo test --workspace --doc --verbose

# リンクエラー検証
cargo rustdoc --workspace -- -D warnings
```

**Doctest失敗時の対応**:
1. CIが失敗した場合、エラーログで失敗したdoctestのファイルと行番号を確認
2. ローカルで再現: `cargo test --doc -p <crate> --verbose`
3. コード例を修正、または適切な属性(`no_run`, `ignore`等)を追加
4. 再度`cargo test --doc`で確認してからpush

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

### ドキュメント品質メトリクスの監視

#### CI警告ログの追跡

CIパイプラインは自動的にドキュメント不足の警告を追跡します:

**警告の確認方法**:
1. GitHub Actionsの「Validate Documentation」ジョブを開く
2. 「Check for missing documentation」ステップのログを確認
3. 「Documentation Coverage Report」セクションで警告数を確認

**警告例**:
```
⚠️  Found 12 items without documentation:
warning: missing documentation for a function
  --> rust/engine/src/cards.rs:42:1
```

**対応方法**:
1. 警告されたファイルと行番号を確認
2. 該当する公開API(`pub`)に`///`コメントを追加
3. `cargo doc --workspace`でローカル確認後、コミット

**メトリクスの追跡**:
- CIログに「Documentation Metrics」セクションが表示されます
- 「Missing documentation warnings」の数値を時系列で追跡
- PRごとに警告数が増加していないか確認

#### 破壊的変更の検出と警告

**自動検出**:
CIパイプラインは自動的に破壊的変更を検出します:

**検出内容**:
- 削除された公開API(`pub fn`, `pub struct`など)
- 変更された関数シグネチャ
- ドキュメントコメントが更新されていない公開API変更
- Cargo.tomlのバージョン変更

**警告の確認方法**:
1. GitHub Actionsの「Validate Documentation」ジョブを開く
2. 「Check for breaking changes」ステップのログを確認
3. 検出された変更と影響を確認

**警告例**:
```
⚠️  Potential breaking change in rust/engine/src/lib.rs:
- pub fn old_function(x: i32) -> String
+ pub fn old_function(x: i32, y: i32) -> String

⚠️  rust/cli/src/commands/play.rs: Public API modified but no documentation comments updated
```

**対応チェックリスト**:
1. [ ] 変更されたすべての公開APIのドキュメントを更新
2. [ ] 破壊的変更の理由と移行方法をドキュメントに記載
3. [ ] `# Examples`セクションを更新(該当する場合)
4. [ ] `cargo test --doc`でdoctestが通ることを確認
5. [ ] RUNBOOK.mdを更新(手順が変わった場合)

**手動実行**:
ローカルで破壊的変更を確認する場合:
```bash
# mainブランチとの差分を確認
bash scripts/check-breaking-changes.sh main

# 特定のブランチとの比較
bash scripts/check-breaking-changes.sh develop
```

#### PRレビュー時のドキュメント確認

**PRテンプレート活用**:
PRを作成すると、自動的に「Documentation Checklist」が表示されます。
レビュアーは以下を確認してください:

**必須チェック項目**:
- [ ] 新規/変更された公開APIに`///`コメントがある
- [ ] 新規モジュールに`//!`モジュールレベルドキュメントがある
- [ ] 複雑なAPIにはコード例(doctest)が含まれている
- [ ] クロスリファレンス(`[Type]`)が適切に使用されている
- [ ] 破壊的変更がある場合、影響を受けるドキュメントが更新されている
- [ ] `cargo test --doc`が成功する
- [ ] `cargo rustdoc -- -D warnings`でリンクエラーがない

**レビューポイント**:
1. **ドキュメントの明瞭性**: 説明が明確で理解しやすいか
2. **完全性**: `# Arguments`, `# Returns`, `# Errors`などの必要なセクションがあるか
3. **正確性**: コード例が実際に動作するか
4. **一貫性**: プロジェクト全体のドキュメントスタイルに沿っているか

#### ドキュメント品質の向上ガイドライン

**最低基準** (すべての公開APIに必須):
- 機能説明(1-2文)
- 目的と責任の明確化

**推奨基準** (複雑なAPIに推奨):
- 引数の意味(`# Arguments`)
- 戻り値の説明(`# Returns`)
- エラーケース(`# Errors`)
- パニック条件(`# Panics`)
- 基本的な使用例(`# Examples`)

**理想的な基準** (主要なAPIに推奨):
- 複数のコード例(基本、応用、エラーケース)
- 関連型へのクロスリファレンス
- パフォーマンス特性(`# Performance`)
- 安全性に関する注意(`# Safety`)

**例**:
```rust
/// Evaluates the strength of a poker hand.
///
/// This function uses a fast lookup table for 5-card hands,
/// providing O(1) evaluation time.
///
/// # Arguments
///
/// * `cards` - A slice of exactly 5 cards
///
/// # Returns
///
/// Returns the hand rank (High Card = 0, Royal Flush = 9)
///
/// # Panics
///
/// Panics if the slice length is not exactly 5.
///
/// # Examples
///
/// ```
/// use axm_engine::{evaluate_hand, Card};
///
/// let hand = vec![
///     Card::new("As"), Card::new("Ks"), Card::new("Qs"),
///     Card::new("Js"), Card::new("Ts"),
/// ];
/// let rank = evaluate_hand(&hand);
/// assert_eq!(rank, 9); // Royal Flush
/// ```
///
/// # Performance
///
/// Typical evaluation time: 10-50 nanoseconds
///
/// See also: [`Hand`], [`compare_hands`]
pub fn evaluate_hand(cards: &[Card]) -> u8 {
    // Implementation...
}
```

## トラブルシュート
- 乱数の再現 `--seed` を指定し同一バージョンで再実行
- JSONL の破損 末尾途中行を検出し以降を破棄
- SQLite のロック 単一プロセスで書き込み バッチ化を使用
