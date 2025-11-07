# 実装計画

## 概要
本タスクリストは、Rustドキュメント自動化機能の実装に必要な作業を定義します。すべてのタスクは、requirements.mdおよびdesign.mdで承認された仕様に基づいています。

## タスク一覧

- [x] 1. ドキュメント生成スクリプトの作成
  - Cargo.tomlを解析してワークスペースメンバーを自動検出する仕組みを実装
  - GitHub Pagesのルートに配置するカスタムindex.htmlを生成
  - 各クレート(axm_engine, axm_cli, axm_web)へのナビゲーションリンクを含むHTMLを出力
  - プロジェクト概要とクレート説明を含むレスポンシブなスタイルを適用
  - スクリプトに実行権限を付与し、bashで実行可能にする
  - _Requirements: 3.4, 3.5_

- [x] 2. CI/CDパイプラインへのrustdocビルドジョブ統合
- [x] 2.1 既存CIワークフローへのrustdocジョブ追加
  - .github/workflows/ci.ymlにrustdocビルドジョブを追加
  - cargo doc --workspace --no-depsを実行してワークスペース全体のドキュメントを生成
  - ドキュメント生成スクリプト(generate-doc-index.sh)を実行してindex.htmlを作成
  - 生成されたtarget/doc/ディレクトリをGitHub Actionsアーティファクトとして保存
  - 既存のCargoキャッシュ戦略(registry/git/target)を再利用
  - 既存ジョブ(test, fmt, clippy等)と並列実行可能な独立ジョブとして定義
  - _Requirements: 2.1, 2.2, 2.4, 2.5, 2.6_

- [x] 2.2 ドキュメント品質検証ジョブの追加
  - .github/workflows/ci.ymlに検証ジョブを追加
  - cargo rustdoc --workspace -- -D warningsを実行して壊れたリンクをエラーとして検出
  - ドキュメントコメント不足の警告をgrepパターンマッチで抽出
  - 検証失敗時にはCI全体を失敗させ、エラー詳細をログに出力
  - 既存のdoctestジョブ(cargo test --workspace --doc)との重複を避ける
  - _Requirements: 2.3, 2.7, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_

- [x] 3. GitHub Pagesデプロイの自動化
- [x] 3.1 デプロイ専用ワークフローの作成
  - .github/workflows/deploy-docs.ymlを新規作成
  - mainブランチへのpushイベントとworkflow_dispatchをトリガーとして設定
  - rustdocビルドとindex.html生成を実行
  - peaceiris/actions-gh-pages@v3を使用してgh-pagesブランチへデプロイ
  - デプロイ失敗時には明確なエラーメッセージを出力してワークフローを失敗させる
  - GitHub Pages機能の有効化手順をドキュメントに追記
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 3.2 検索機能とクロスクレートリンクの検証
  - rustdoc標準の検索インデックス(search-index.js)が正しく生成されることを確認
  - クレート間のドキュメントリンク([crate::module::Type]形式)が正しく動作することを検証
  - GitHub Pagesデプロイ後にブラウザで検索機能をテスト
  - トップページから各クレートへのナビゲーションが正常に動作することを確認
  - _Requirements: 3.6, 6.5_

- [x] 4. engineクレートのドキュメント充実化(主要API)
- [x] 4.1 コアデータ構造のドキュメント作成
  - Card, Suit, Rank, Deck, Engine, GameState, Player, HandRecordの8つの主要型に機能説明を追加
  - 各構造体/列挙型の目的と役割を1-2文で説明
  - 主要なフィールドに対して説明コメントを追加
  - Engine型とDeck型には使用例(doctest)を追加
  - cargo doc -p axm-engineでビルド成功を確認
  - _Requirements: 1.1, 1.5, 1.6_

- [x] 4.2 公開APIのドキュメント作成
  - 公開関数(evaluate_hand, validate_action等)に機能説明を追加
  - Arguments、Returns、Errorsセクションを含む包括的なドキュメントを記述
  - 主要な関数にはdoctestを追加してコード例を提供
  - lib.rsにモジュール概要とクレート全体の使い方ガイドを追加
  - cargo rustdoc -p axm-engine -- -D warningsでリンクエラーがないことを確認
  - _Requirements: 1.1, 1.5, 1.6_

- [x] 5. cliクレートとwebクレートのドキュメント充実化
- [x] 5.1 CLIサブコマンドのドキュメント作成
  - play, sim, stats, verify等の主要サブコマンド実装にドキュメントを追加
  - 各サブコマンドの目的、使い方、主要なオプションの説明を記述
  - 主要コマンドには使用例(doctest)を追加
  - cargo doc -p axm_cliでビルド成功を確認
  - _Requirements: 1.2, 1.5_

- [x] 5.2 webクレートのハンドラーとAPIドキュメント作成
  - ハンドラー関数とセッション管理APIにドキュメントを追加
  - HTTPメソッド、パス、エンドポイントの目的を明記
  - リクエスト/レスポンス形式とエラーケースを説明
  - cargo doc -p axm_webでビルド成功を確認
  - _Requirements: 1.3, 1.5_

- [x] 6. ローカル開発環境のドキュメント生成手順整備
  - RUNBOOK.mdにローカルでのrustdoc生成手順を追記
  - cargo doc --workspace --openコマンドの使い方を説明
  - 特定クレートのみ生成する方法(cargo doc -p <crate> --open)を記載
  - プライベートAPIを含む内部開発用ドキュメント生成方法を追記
  - よくあるエラー(壊れたリンク、未解決のインポート)のトラブルシューティング手順を追加
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [x] 7. ドキュメント品質の継続的検証機能の実装
- [x] 7.1 Doctestの実行と検証
  - 既存のtestジョブ内のcargo test --workspace --docが正常に動作することを確認
  - doctest失敗時の詳細なエラーメッセージが出力されることを検証
  - 実行不可能なコード例にno_run属性を追加するガイドラインを作成
  - _Requirements: 4.4, 4.5, 4.6, 6.6_

- [x] 7.2 ドキュメント品質メトリクスの監視
  - CI警告ログからドキュメント不足の警告数を追跡できることを確認
  - PRレビュー時のドキュメント追加確認チェックリストを作成
  - 破壊的変更時のドキュメント更新を促す警告メッセージを実装
  - _Requirements: 1.4, 6.1, 6.2, 6.3, 6.4_

- [x] 8. 統合テストとエンドツーエンドテスト
- [x] 8.1 CI/CDパイプライン統合テスト
  - フィーチャーブランチでワークフローをトリガーし、rustdocジョブとvalidate-docsジョブが並列実行されることを確認
  - 両ジョブが成功し、アーティファクトが正しく生成されることを検証
  - 意図的に壊れたリンクを含むコードを追加してvalidate-docsジョブが失敗することを確認
  - 意図的に失敗するdoctestを追加してtestジョブが失敗することを確認
  - _Requirements: 全要件の統合検証_

- [x] 8.2 GitHub Pagesデプロイテスト
  - mainブランチへのマージ後にdeploy-docsワークフローが自動実行されることを確認
  - gh-pagesブランチが正しく更新されることを検証
  - GitHub Pagesの公開URL(https://&lt;owner&gt;.github.io/&lt;repo&gt;/)でドキュメントが閲覧可能であることを確認
  - トップページに各クレートへのリンクが表示され、正常にナビゲートできることをテスト
  - 検索バーでAPIを検索し、正しい検索結果が表示されることを確認
  - _Requirements: 全要件の統合検証_

- [ ] 9. パフォーマンス検証とキャッシュ効率化
  - cargo doc --workspace --no-depsのビルド時間を測定(目標: CI環境で5分以内)
  - Cargoキャッシュが正しく動作し、2回目のビルドが高速化されることを確認
  - GitHub Pagesのページロード時間を測定(目標: トップページ1秒以内)
  - Lighthouse監査でドキュメントサイトの性能スコアを確認
  - _Requirements: 全要件の性能基準達成_

## タスク進行上の注意事項

### 依存関係
- タスク1は、タスク2の前に完了する必要がある(スクリプトがCIワークフローで使用される)
- タスク2完了後、タスク3のデプロイワークフローを作成可能
- タスク4-5のドキュメント充実化は、タスク2-3の自動化基盤完成後に並行して進行可能
- タスク8の統合テストは、すべての機能実装完了後に実行

### 品質基準
- すべてのドキュメントコメントは英語で記述(プロジェクト標準)
- 機能説明(1-2文)は最低基準、引数・戻り値説明は推奨
- 主要なAPIには必ずdoctestを含める
- cargo rustdoc -- -D warningsでエラーが出ないことを確認

### 検証方法
- 各タスク完了後にローカル環境でcargo doc --openを実行してプレビュー
- CIパイプラインでの自動検証を必ず実施
- GitHub Pagesデプロイ後にブラウザで実際のドキュメントを確認

## 要件カバレッジ

### Requirement 1: ドキュメントコメントの充実化
- タスク4.1, 4.2 (engine), タスク5.1 (cli), タスク5.2 (web)

### Requirement 2: Rustdocビルドの自動化
- タスク2.1, 2.2

### Requirement 3: GitHub Pagesへのホスティング
- タスク1, 3.1, 3.2

### Requirement 4: ドキュメント品質の継続的検証
- タスク2.2, 7.1, 7.2

### Requirement 5: ローカル開発環境でのドキュメント生成
- タスク6

### Requirement 6: ドキュメントのメンテナンス性確保
- タスク3.2, 7.2
