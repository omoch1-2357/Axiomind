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
```bash
# ワークスペース全体のドキュメントを生成してブラウザで開く
cargo doc --workspace --open

# 特定のクレートのみ生成
cargo doc -p axm-engine --open

# プライベートAPIを含む(内部開発用)
cargo doc --workspace --document-private-items --open
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

### トラブルシューティング
- **デプロイが失敗する**: Settings > Actions > General で「Read and write permissions」が有効か確認
- **GitHub Pagesが更新されない**: Actions タブでワークフローが成功しているか確認
- **404エラーが出る**: gh-pagesブランチが作成されているか確認

## トラブルシュート
- 乱数の再現 `--seed` を指定し同一バージョンで再実行
- JSONL の破損 末尾途中行を検出し以降を破棄
- SQLite のロック 単一プロセスで書き込み バッチ化を使用
