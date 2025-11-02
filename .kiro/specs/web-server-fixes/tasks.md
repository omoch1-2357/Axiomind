# Implementation Plan

## Phase 0: Pre-Fix (事前修正)

- [x] 0. JavaScript構文エラーの緊急修正とESLint環境の正規化
  - game.js:66-69の構文エラー修正（テンプレートリテラル内の引用符をバッククォートに統一）
  - .eslintignoreファイルを作成し、game.test.jsとhtmx.min.jsを除外対象として追加
  - npm run lintを実行し、エラーゼロになることを確認
  - ESLintが正常にgame.jsファイル全体をパース可能であることを検証
  - _Requirements: 1.1, 1.2, 1.3, 3.1, 3.5_

## Phase 1: Emergency Fixes (緊急修正 - 1-2日)

- [x] 1. htmx JSON送信拡張の統合とContent-Type問題の解決
  - index.htmlの<head>セクションにjson-enc.js拡張スクリプトを追加（htmx 1.9.12 CDNから）
  - 既存のhx-ext="json-enc"属性が正常に動作することを確認
  - フォーム送信時にapplication/jsonのContent-Typeヘッダーが設定されることを検証
  - opponent_type形式が"ai:baseline"文字列として正しく送信されることを確認（既存実装の維持）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 11.1, 11.5_

- [ ] 1.1 既存E2Eテストでの統合検証
  - tests/e2e/game-flow.spec.jsを実行し、ゲーム開始フローが成功することを確認
  - APIリクエストインターセプトでContent-Type: application/jsonを検証
  - RustサーバーがJSONペイロードを正常にデシリアライズできることを確認
  - JavaScriptコンソールにエラーが出力されないことを検証
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.1, 5.2_

- [ ] 1.2 Pre-commitフックとCI/CDパイプラインの検証
  - Pre-commitフックが正常にESLintを実行することを確認（既存の.githooks/pre-commitを使用）
  - GitHub Actions CIのfrontend-lintジョブが成功することを検証
  - 全てのRustテスト（178個）が引き続き合格することを確認
  - CI/CDパイプライン全体が正常に完了することを検証
  - _Requirements: 3.2, 3.4, 6.3, 9.1, 9.2, 9.3_

## Phase 2: E2E Test Expansion (E2Eテスト拡充 - 2-3日)

- [ ] 2. SSEイベントストリームの包括的E2Eテスト
  - tests/e2e/sse-events.spec.jsファイルを作成
  - EventSource接続の確立とハンドシェイクを検証するテストを実装
  - game_startedイベント受信時のplayers配列構造を検証するテストを追加
  - hand_startedイベント受信時のhand_id、button_position、blinds情報の検証
  - action_takenイベント受信時のUI更新（ポット額、プレイヤースタック、current_player変更）を検証
  - SSE接続切断後の自動再接続メカニズム（5秒間隔）を検証するテストを実装
  - _Requirements: 4.1, 5.3, 6.1, 6.2_

- [ ] 2.1 エラーハンドリングの包括的E2Eテスト
  - tests/e2e/error-handling.spec.jsファイルを作成
  - 無効なセッションIDでのAPIリクエスト時の404/400レスポンス処理を検証
  - 無効なopponent_type形式（例: "AI"のみ、JSONオブジェクト形式）でのエラーレスポンスを検証
  - JavaScriptランタイムエラーの検出（page.on('console')でerrorレベル監視）を実装
  - APIタイムアウトシミュレーション時の待機状態表示とユーザーへの通知を検証
  - htmx:responseErrorイベント発生時のエラーモーダル表示を検証
  - _Requirements: 4.4, 5.5, 10.1, 10.2, 10.3, 10.4, 11.3_

- [ ] 2.2 ベット入力コントロールの包括的E2Eテスト
  - tests/e2e/betting-controls.spec.jsファイルを作成
  - ベット額バリデーション（最小額未満）時の赤枠エラー表示とボタン無効化を検証
  - ベット額バリデーション（最大額超過）時のエラーメッセージ表示を検証
  - 有効範囲内のベット額入力時のエラー非表示とボタン有効化を確認
  - Fold/Check/Callボタンクリック時の正しいAPIペイロード送信（hx-vals形式）を検証
  - 非アクティブターン時（current_player !== 0）のUI無効化とボタングレーアウトを確認
  - betInput動的バリデーション（oninputイベント）とリアルタイムエラー表示を検証
  - _Requirements: 5.4, 6.1, 6.2, 11.1, 11.2_

- [ ] 2.3 セッション管理の包括的E2Eテスト
  - tests/e2e/session-management.spec.jsファイルを作成
  - セッション作成APIレスポンスからsession_id（UUID v4形式）を取得し検証
  - /api/sessions/{id}/stateエンドポイントからGameStateResponseのJSON構造を検証
  - 複数セッションの並行実行と相互独立性（異なるsession_idで異なる状態）を検証
  - セッションの永続性（ページリロード後のsession_id復元とstate取得）を確認
  - 存在しないセッションIDでのアクセス時の404レスポンスとlobbyへのリダイレクトを検証
  - _Requirements: 4.3, 5.1, 5.2, 6.4_

- [ ] 2.4 エッジケースとショーダウンの包括的E2Eテスト
  - tests/e2e/edge-cases.spec.jsファイルを作成
  - 空のcommunity cards配列（プリフロップ時）でのプレースホルダー表示（"[?] [?] [?] [?] [?]"）を検証
  - showdown時のカード表示（players[1].hole_cards）とrenderHandResult()の結果オーバーレイを確認
  - split pot（引き分け）時の"Split Pot"メッセージ表示とpot金額の均等分配を検証
  - 手札結果オーバーレイの表示/解除（showHandResult/dismissHandResultボタン）を確認
  - ゲーム終了時（end_reason: "showdown" | "fold"）のUI状態とNext Handボタン表示を検証
  - _Requirements: 4.1, 6.1, 6.4_

- [ ] 2.5 静的アセット配信の包括的E2Eテスト
  - tests/e2e/static-assets.spec.jsファイルを作成（既存のgame-flow.spec.jsから分離）
  - /static/css/app.cssリクエスト時のContent-Type: text/cssヘッダーを検証
  - /static/js/game.jsリクエスト時のContent-Type: application/javascriptヘッダーを検証
  - /static/js/htmx.min.jsおよび/static/js/json-enc.jsの正常な読み込みを確認
  - 存在しないファイルパス（例: /static/nonexistent.js）での404レスポンスを検証
  - Cache-Controlヘッダーの存在と適切な値（静的アセット用）を確認
  - _Requirements: 4.5, 7.1, 7.2, 7.3, 7.4, 7.5_

## Phase 3: Documentation & Process Formalization (ドキュメント整備 - 1-2日)

- [ ] 3. TESTING.mdの更新とインシデントレポートの統合
  - 2025-11-02インシデントの詳細（178個のRustテスト全合格だがブラウザで不動作）を追加
  - 根本原因分析セクションを追加（構文エラー、Content-Type不一致、静的解析欠如、E2E未実装）
  - テストピラミッド図を更新（Unit → Integration → E2E階層の明示）
  - フロントエンド変更時のE2E必須チェックリストを追加（npm run lint、npm run test:e2e、ブラウザ手動確認）
  - E2Eテスト失敗時のデバッグ手順（スクリーンショット、トレースログ、ブラウザコンソール確認）を文書化
  - _Requirements: 6.4, 8.1, 8.3, 10.5_

- [ ] 3.1 FRONTEND_GUIDELINES.mdの更新とベストプラクティス標準化
  - htmx json-enc拡張の使用例と統合パターンを追加
  - テンプレートリテラルのベストプラクティス（バッククォート一貫使用、no-template-curly-in-stringルール）を文書化
  - E2Eテスト作成ガイド（Playwright API、page.waitForSelector、API interception）を追加
  - 正しい実装例と誤った実装例の比較セクション（XSS対策、htmx属性、Content-Type）を追加
  - 新規JavaScript機能追加時のワークフロー（E2Eテスト作成 → 実装 → ESLint → 手動確認）を文書化
  - _Requirements: 6.1, 8.2, 8.4, 11.2, 11.3_

- [ ] 3.2 CI/CDパイプラインの文書化と開発者オンボーディング資料の整備
  - .github/workflows/ci.ymlの各ジョブにコメントを追加（目的、依存関係、成功基準）
  - READMEに全テストコマンド一覧を追加（cargo test、npm run lint、npm run test:e2e）
  - 新規開発者向けオンボーディングドキュメントを作成（環境セットアップ、Git hooks有効化、初回テスト実行）
  - Pre-commitフックの動作説明と有効化手順（git config core.hooksPath .githooks）を追加
  - CIパイプライン失敗時のトラブルシューティングガイド（ログ確認、ローカル再現、修正手順）を文書化
  - _Requirements: 6.3, 8.3, 9.1, 9.2, 9.4_

- [ ] 3.3 継続的品質保証プロセスの標準化
  - フロントエンド品質ゲートチェックリストを作成（ESLintゼロエラー、E2E全合格、手動ブラウザ確認）
  - コードレビューガイドラインにE2Eテストカバレッジ確認項目を追加
  - テストギャップ検出プロセスを定義（新規API追加時の対応E2Eテスト要求）
  - フロントエンド統合テストカバレッジ測定方法を文書化（Playwrightレポート活用）
  - テスト駆動開発プロセスのフローチャート図を追加（要件 → E2Eテスト → 実装 → 検証）
  - _Requirements: 6.1, 6.2, 6.4, 9.4, 9.5_

## 完了基準

全てのタスク完了後、以下の基準を満たすこと：

- ✅ npm run lintがゼロエラーで成功
- ✅ npm run test:e2eが全テストケース合格（5+ test files、20+ test cases）
- ✅ cargo test --workspaceが全178テスト合格（既存テスト互換性維持）
- ✅ GitHub Actions CI全ジョブ成功（frontend-lint、frontend-e2e含む）
- ✅ 手動ブラウザテストで全機能動作確認（ゲーム開始、ベット、SSEイベント受信、エラーハンドリング）
- ✅ TESTING.mdおよびFRONTEND_GUIDELINES.mdが最新の実装を反映
- ✅ 11個の要件すべてに対応するテストケースが実装済み

## 要件カバレッジマップ

| 要件ID | 要件概要 | 対応タスク |
|-------|---------|----------|
| Req 1 | JavaScript構文エラー修正 | 0, 1.1 |
| Req 2 | htmx Content-Type解決 | 1, 1.1 |
| Req 3 | ESLint完全実装 | 0, 1.2 |
| Req 4 | ブラウザE2E実装 | 1.1, 2, 2.1, 2.2, 2.3, 2.4, 2.5 |
| Req 5 | 統合検証 | 1.1, 2.3 |
| Req 6 | テスト駆動プロセス | 2, 2.1, 2.2, 3.3 |
| Req 7 | 静的アセット配信検証 | 2.5 |
| Req 8 | ドキュメント更新 | 3, 3.1, 3.2 |
| Req 9 | 品質保証プロセス | 1.2, 3.2, 3.3 |
| Req 10 | エラーハンドリング強化 | 2.1, 3 |
| Req 11 | htmx統合正規化 | 1, 2.2, 3.1 |
