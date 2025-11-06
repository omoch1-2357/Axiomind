# Requirements Document

## Project Description (Input)
新規にe2eテストなどを追加したことで、今まで見えていなかった潜在的かつクリティカルな問題が明らかになった。これらを修正し、rust-web-server specで実装した各種実装に関して、改めて完全に動くようにしたい。

## Introduction

本仕様書は、E2Eテストの導入により発覚したWebサーバー実装の重大な問題を修正し、全ての機能を実際のブラウザ環境で完全に動作させることを目的とします。TESTING.mdおよびFRONTEND_GUIDELINES.mdに記載された2025-11-02のインシデントレポートによると、178個のRustテストが全て合格していたにもかかわらず、実際のブラウザでは以下の問題により完全に動作していませんでした：

1. **JavaScript構文エラー** - `game.js:66-69`のテンプレートリテラル誤用
2. **htmx Content-Type不一致** - `application/x-www-form-urlencoded` vs `application/json`のミスマッチ
3. **静的解析の欠如** - ESLintが未実装
4. **ブラウザE2Eテストの欠如** - Playwrightテストが存在しなかった

これらの問題は、バックエンドテストだけではフロントエンド統合の健全性を保証できないことを示しています。本仕様では、これらの問題を根本的に解決し、継続的な品質を保証するための包括的な要件を定義します。

**スコープ**: 本仕様は緊急のバグ修正、テスト基盤の強化、ドキュメント整備に集中します。game.jsの大規模なリファクタリング（モジュール分割、ES modules導入）は、別specification（`web-server-refactor`）として分離し、将来的に対応します。

## Requirements

### Requirement 1: JavaScript構文エラーの修正

**Objective:** As a developer, I want all JavaScript syntax errors to be identified and fixed, so that the UI renders correctly in browsers.

#### Acceptance Criteria

1. WHEN ESLintが`game.js`を検査する THEN システムはテンプレートリテラル内の不正な引用符混在を検出しなければならない
2. WHEN `game.js:66-69`のテンプレートリテラルを修正する THEN システムはバッククォートを一貫して使用しなければならない
3. WHEN JavaScriptファイルがブラウザで読み込まれる THEN システムは構文エラーなしで実行されなければならない
4. WHEN コンソールログを確認する THEN システムはJavaScript実行時エラーを表示しないこと
5. IF テンプレートリテラル内で条件演算子を使用する THEN システムは全ての文字列をバッククォートで囲まなければならない

### Requirement 2: htmx Content-Type問題の解決

**Objective:** As a web client, I want form submissions to use the correct Content-Type header, so that the server can parse requests properly.

#### Acceptance Criteria

1. WHEN `START GAME`ボタンがクリックされる THEN システムは`application/json`のContent-Typeでリクエストを送信しなければならない
2. WHEN APIエンドポイント`/api/sessions`にPOSTする THEN システムはJSONペイロード形式でデータを送信しなければならない
3. WHEN `opponent_type`フィールドを送信する THEN システムは`"ai:baseline"`形式（`"AI"`や`{"AI": "baseline"}`ではない）を使用しなければならない
4. WHEN フォームデータをJSON変換する THEN システムは`FormData`から`Object.fromEntries()`を使用して変換しなければならない
5. IF htmxの`hx-ext="json-enc"`を使用する THEN システムは適切なhtmx拡張ライブラリをロードしなければならない

### Requirement 3: ESLint静的解析の完全実装

**Objective:** As a developer, I want automated JavaScript linting to catch syntax errors before runtime, so that code quality is maintained.

#### Acceptance Criteria

1. WHEN `npm run lint`を実行する THEN システムは全てのJavaScriptファイルに対してESLintを実行しなければならない
2. WHEN ESLintがエラーを検出する THEN システムはゼロエラーになるまでコミットをブロックしなければならない
3. WHEN `.eslintrc.json`設定を確認する THEN システムは`no-template-curly-in-string`ルールを有効化しなければならない
4. WHEN pre-commitフックが実行される THEN システムは`npm run lint`を自動実行しなければならない
5. IF lintエラーが存在する THEN システムはエラー位置と修正方法を明確に表示しなければならない

### Requirement 4: ブラウザE2Eテストの包括的実装

**Objective:** As a QA engineer, I want comprehensive E2E tests that validate the complete user flow in a real browser, so that integration issues are caught before deployment.

#### Acceptance Criteria

1. WHEN Playwrightテストを実行する THEN システムは実ブラウザでゲーム開始フローを検証しなければならない
2. WHEN APIリクエストをインターセプトする THEN システムはContent-Typeヘッダーが`application/json`であることを検証しなければならない
3. WHEN ペイロード構造を検証する THEN システムは`level`と`opponent_type`フィールドの存在を確認しなければならない
4. WHEN JavaScriptコンソールエラーを監視する THEN システムはエラー発生時にテストを失敗させなければならない
5. WHEN 静的アセット読み込みを検証する THEN システムはCSS、JavaScript、画像の正常な読み込みを確認しなければならない

### Requirement 5: フロントエンド・バックエンド統合の検証

**Objective:** As a system integrator, I want to verify that the browser UI correctly communicates with the Rust backend, so that all integration points work seamlessly.

#### Acceptance Criteria

1. WHEN ブラウザがフォームを送信する THEN システムはRustサーバーが期待する正確なJSON形式でデータを送信しなければならない
2. WHEN サーバーがレスポンスを返す THEN システムはhtmxがDOMを正しく更新しなければならない
3. WHEN SSEイベントストリームを接続する THEN システムはゲームイベントをリアルタイムで受信しなければならない
4. WHEN ベット入力フィールドを検証する THEN システムはクライアント側とサーバー側の両方でバリデーションを実行しなければならない
5. IF APIエラーが発生する THEN システムはユーザーフレンドリーなエラーメッセージを表示しなければならない

### Requirement 6: テスト駆動開発プロセスの確立

**Objective:** As a development team, I want a test-first development process, so that new features are validated in browsers before being considered complete.

#### Acceptance Criteria

1. WHEN 新しいフロントエンド機能を追加する THEN 開発者はPlaywright E2Eテストを先に作成しなければならない
2. WHEN Rustテストが合格する THEN 開発者はさらに`npm run test:e2e`を実行して統合を確認しなければならない
3. WHEN CIパイプラインが実行される THEN システムはRustテスト、ESLint、Playwright E2Eの全てを実行しなければならない
4. WHEN コードレビューを実施する THEN レビュアーはE2Eテストカバレッジを確認しなければならない
5. IF E2Eテストが失敗する THEN システムはスクリーンショットとトレースログを提供しなければならない

### Requirement 7: 静的アセット配信の検証

**Objective:** As a web server operator, I want to ensure all static assets are served with correct MIME types and headers, so that browsers can render the UI properly.

#### Acceptance Criteria

1. WHEN `app.css`をリクエストする THEN システムは`Content-Type: text/css`で応答しなければならない
2. WHEN `game.js`をリクエストする THEN システムは`Content-Type: application/javascript`で応答しなければならない
3. WHEN 画像ファイルをリクエストする THEN システムは適切な画像MIMEタイプで応答しなければならない
4. WHEN 存在しないファイルをリクエストする THEN システムは404レスポンスを返さなければならない
5. IF キャッシュヘッダーを設定する THEN システムは静的アセットに適切なCache-Controlを付与しなければならない

### Requirement 8: ドキュメント更新と知識共有

**Objective:** As a team member, I want comprehensive documentation of the incident and fixes, so that similar issues are prevented in the future.

#### Acceptance Criteria

1. WHEN TESTING.mdを更新する THEN システムはインシデントの根本原因と修正内容を記録しなければならない
2. WHEN FRONTEND_GUIDELINES.mdを参照する THEN 開発者は必須チェックリストを明確に理解できなければならない
3. WHEN 新規開発者がオンボーディングする THEN システムはフロントエンドテストの重要性を強調する資料を提供しなければならない
4. WHEN コードサンプルを提供する THEN システムは正しい実装例と誤った実装例の両方を示さなければならない
5. IF 新しいフロントエンド技術を導入する THEN システムはベストプラクティスとテスト戦略をドキュメント化しなければならない

### Requirement 9: 継続的品質保証プロセスの構築

**Objective:** As a project manager, I want automated quality gates that prevent broken code from reaching production, so that user experience is consistently high.

#### Acceptance Criteria

1. WHEN pre-commitフックが実行される THEN システムは`cargo fmt`、`npm run lint`、および基本的な構文チェックを実行しなければならない
2. WHEN CIパイプラインが実行される THEN システムは全てのテストレイヤー（Unit、Integration、E2E）を実行しなければならない
3. WHEN いずれかのテストが失敗する THEN システムはマージをブロックしなければならない
4. WHEN コードカバレッジを測定する THEN システムはフロントエンド統合テストのカバレッジ率を報告しなければならない
5. IF テストギャップが発見される THEN システムは開発者に新しいテストケース追加を促すアラートを発行しなければならない

### Requirement 10: エラーハンドリングとデバッグ支援の強化

**Objective:** As a developer, I want clear error messages and debugging tools, so that I can quickly identify and fix issues.

#### Acceptance Criteria

1. WHEN ブラウザコンソールでエラーが発生する THEN システムはスタックトレースと関連コンテキストを表示しなければならない
2. WHEN APIリクエストが失敗する THEN システムはHTTPステータスコード、エラーメッセージ、およびリクエストペイロードをログに記録しなければならない
3. WHEN htmxリクエストがエラーを返す THEN システムはユーザーに通知しリトライオプションを提供しなければならない
4. WHEN E2Eテストが失敗する THEN システムは失敗時のスクリーンショットとブラウザログを保存しなければならない
5. IF 開発モードで実行中 THEN システムは詳細なデバッグ情報をコンソールに出力しなければならない

### Requirement 11: htmx統合の正規化

**Objective:** As a frontend architect, I want standardized htmx usage patterns, so that all AJAX interactions are predictable and maintainable.

#### Acceptance Criteria

1. WHEN htmx属性を使用する THEN システムはJSON送信に`hx-vals='js:{...}'`パターンを使用しなければならない
2. WHEN サーバー応答を処理する THEN システムは適切な`hx-swap`戦略（innerHTML、outerHTML、none）を指定しなければならない
3. WHEN htmxイベントをリッスンする THEN システムは`htmx:afterSwap`、`htmx:responseError`などの標準イベントを使用しなければならない
4. WHEN 複雑なフォームロジックが必要 THEN システムはhtmxの代わりに`fetch` APIを使用しても良い
5. IF htmx拡張を使用する THEN システムは必要なスクリプトを`<head>`セクションで明示的にロードしなければならない

## Future Work

### game.jsのリファクタリング（別specification推奨）

大規模なコードリファクタリング（モジュール分割、ES modules導入、名前空間管理）は、本specのスコープ外とし、将来的に別specification（例: `web-server-refactor`）として対応することを推奨します。

**分離理由**:
- 本specは緊急のバグ修正に集中（1-2日で本番投入可能）
- リファクタリングは5-7日の工数が必要で、影響範囲が大きい
- 段階的な改善により、リグレッションリスクを最小化

**リファクタリングspec候補要件**:
- ES modulesへの移行（card-renderer.js, bet-validator.js, error-handler.js, sse-client.js）
- グローバル関数の名前空間管理
- XSS対策の体系的実装（DOMPurify導入検討）
- テストカバレッジ向上（モジュール単位のテスト）
