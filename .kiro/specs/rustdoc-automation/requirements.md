# Requirements Document

## Project Description (Input)
Rustdocの充実化とビルド/ホスティングの自動化

## Introduction
本仕様は、Axiomindプロジェクトにおけるドキュメント生成の自動化と充実化を目的としています。現在、Rustワークスペース(engine、cli、web)には適切なドキュメントコメントが不足しており、開発者がAPIを理解するのに時間がかかります。本仕様では、rustdocによる包括的なドキュメント生成、自動ビルドプロセスの整備、ホスティング環境の構築を行います。

## Requirements

### Requirement 1: ドキュメントコメントの充実化
**Objective:** As a コードベースのメンテナー, I want すべての公開API・モジュール・主要な関数に適切なドキュメントコメントを付与する, so that 開発者がAPIの使い方や設計意図を理解しやすくなる

#### Acceptance Criteria
1. WHEN rustdoc-automationシステムがengineクレートのソースコードをスキャンする THEN rustdoc-automationシステム SHALL すべての`pub`宣言された構造体・列挙型・トレイトに対してドキュメントコメント(`///`)の存在を確認する
2. WHEN rustdoc-automationシステムがcliクレートのソースコードをスキャンする THEN rustdoc-automationシステム SHALL すべてのサブコマンド実装とヘルパー関数に対してドキュメントコメントの存在を確認する
3. WHEN rustdoc-automationシステムがwebクレートのソースコードをスキャンする THEN rustdoc-automationシステム SHALL すべてのハンドラー関数とセッション管理APIに対してドキュメントコメントの存在を確認する
4. IF ドキュメントコメントが存在しない公開API要素が検出された場合 THEN rustdoc-automationシステム SHALL 警告を出力し、該当箇所のリストをレポートする
5. WHERE ドキュメントコメント内 THE rustdoc-automationシステム SHALL 以下の要素を含むことを推奨する: 機能説明、引数の意味、戻り値の説明、使用例(該当する場合)、エラーケース(該当する場合)
6. WHEN engineクレートの`lib.rs`が更新される THEN rustdoc-automationシステム SHALL モジュール概要とクレート全体の使い方ガイドがコメントに含まれていることを確認する

### Requirement 2: Rustdocビルドの自動化
**Objective:** As a CI/CDパイプライン管理者, I want rustdocビルドをGitHubアクションで自動化する, so that コードの変更時に常に最新のドキュメントが生成される

#### Acceptance Criteria
1. WHEN `.github/workflows/`ディレクトリにrustdocビルド用のワークフローファイルが作成される THEN rustdoc-automationシステム SHALL ワークスペース全体(`cargo doc --workspace`)のドキュメントを生成するジョブを定義する
2. WHEN mainブランチへのpushまたはpull requestが作成される THEN GitHub Actions SHALL rustdocビルドジョブを自動実行する
3. IF rustdocビルドが失敗した場合 THEN GitHub Actions SHALL ビルドを失敗させ、CI結果にエラー詳細を表示する
4. WHEN rustdocビルドが成功する THEN GitHub Actions SHALL 生成されたHTMLドキュメントをアーティファクトとして保存する
5. WHERE rustdocビルドジョブ内 THE GitHub Actions SHALL `--no-deps`フラグを使用し、外部クレートのドキュメント生成を除外する
6. WHEN rustdocビルドジョブが実行される THEN GitHub Actions SHALL `--document-private-items`フラグを使用せず、公開APIのみをドキュメント化する
7. IF ドキュメントコメントに無効なマークダウン構文が含まれる場合 THEN rustdocビルド SHALL 警告を出力する

### Requirement 3: GitHub Pagesへのホスティング
**Objective:** As a 開発者またはユーザー, I want rustdocで生成されたドキュメントをGitHub Pagesで閲覧する, so that オンラインでいつでもAPIリファレンスにアクセスできる

#### Acceptance Criteria
1. WHEN mainブランチへのpushが成功し、rustdocビルドが完了する THEN GitHub Actions SHALL 生成されたドキュメントを`gh-pages`ブランチまたはGitHub Pages対応ディレクトリにデプロイする
2. WHEN GitHub Pagesのデプロイが完了する THEN ドキュメント SHALL `https://<owner>.github.io/<repo>/`または指定されたカスタムドメインでアクセス可能になる
3. IF デプロイ処理が失敗した場合 THEN GitHub Actions SHALL エラーメッセージを出力し、ワークフローを失敗させる
4. WHEN ドキュメントがGitHub Pagesで公開される THEN トップページ SHALL 各クレート(`axm-engine`, `axm_cli`, `axm_web`)へのリンクを含む
5. WHERE GitHub Pagesのルートディレクトリ THE ドキュメント SHALL プロジェクト概要とナビゲーションメニューを含むカスタムインデックスページを提供する
6. WHEN ユーザーがGitHub Pagesのドキュメントにアクセスする THEN ドキュメント SHALL 検索機能(rustdoc標準の検索バー)を提供する

### Requirement 4: ドキュメント品質の継続的検証
**Objective:** As a プロジェクトメンテナー, I want ドキュメントの品質と一貫性を継続的に検証する, so that ドキュメントが常に高品質で保守性が高い状態を維持できる

#### Acceptance Criteria
1. WHEN pre-commitフックまたはCIパイプラインが実行される THEN rustdoc-automationシステム SHALL `cargo doc --workspace`を実行し、ドキュメントコメントの構文エラーを検出する
2. IF ドキュメントコメントに壊れたリンクまたは未解決のパスが含まれる場合 THEN rustdoc-automationシステム SHALL 警告または失敗を出力する
3. WHEN 新しい公開API要素が追加される AND ドキュメントコメントが存在しない場合 THEN rustdoc-automationシステム SHALL CI/CDパイプラインで警告を出力する
4. WHERE Rustファイルのドキュメントコメント内 THE rustdoc-automationシステム SHALL コード例の構文チェックを実行する(`cargo test --doc`)
5. WHEN ドキュメント内のコード例が実行される THEN rustdoc-automationシステム SHALL 例が正しくコンパイルおよび実行されることを確認する
6. IF ドキュメント内のコード例が失敗する場合 THEN rustdoc-automationシステム SHALL 該当箇所とエラー詳細をレポートする

### Requirement 5: ローカル開発環境でのドキュメント生成
**Objective:** As a 開発者, I want ローカル環境で簡単にドキュメントを生成・確認する, so that 変更前後のドキュメントをすぐにプレビューできる

#### Acceptance Criteria
1. WHEN 開発者が`cargo doc --workspace --open`を実行する THEN rustdoc-automationシステム SHALL すべてのクレートのドキュメントを生成し、ブラウザで自動的に開く
2. WHEN 開発者が特定のクレート(例: `cargo doc -p axm-engine --open`)のドキュメントを生成する THEN rustdoc-automationシステム SHALL 該当クレートのみのドキュメントを生成し、ブラウザで開く
3. IF ローカルでドキュメント生成が失敗した場合 THEN rustdoc-automationシステム SHALL エラー詳細をターミナルに出力する
4. WHEN 開発者が`README.md`または`RUNBOOK.md`を確認する THEN ドキュメント SHALL ローカルでのrustdoc生成手順を明記している
5. WHERE ローカル開発環境 THE rustdoc-automationシステム SHALL 生成されたドキュメントを`target/doc/`ディレクトリに配置する
6. WHEN 開発者がドキュメントの変更を確認する THEN rustdoc-automationシステム SHALL 変更差分が反映された最新のHTMLドキュメントをブラウザに表示する

### Requirement 6: ドキュメントのメンテナンス性確保
**Objective:** As a 長期的なプロジェクトメンテナー, I want ドキュメントが常に最新の状態を保ち、保守しやすい構造を持つ, so that プロジェクトの成長に伴いドキュメントが陳腐化しない

#### Acceptance Criteria
1. WHEN 公開APIに破壊的変更が加えられる THEN rustdoc-automationシステム SHALL 該当するドキュメントコメントの更新を促す警告を出力する
2. IF ドキュメントコメント内でバージョン番号や古い仕様への参照が検出される場合 THEN rustdoc-automationシステム SHALL 定期的な見直しを促す警告を出力する
3. WHEN 新しいモジュールまたはクレートが追加される THEN rustdoc-automationシステム SHALL モジュール概要ドキュメント(`//!`コメント)の存在を確認する
4. WHERE ドキュメントコメント内 THE rustdoc-automationシステム SHALL 適切なマークダウンフォーマット(見出し、リスト、コードブロック)の使用を推奨する
5. WHEN ドキュメントがGitHub Pagesで公開される THEN ドキュメント SHALL 各クレート間のリンク(クロスクレート参照)が正しく動作することを保証する
6. IF ドキュメントコメント内の例が古くなり、現在のAPIと一致しなくなった場合 THEN doctest実行時 SHALL エラーを検出する

