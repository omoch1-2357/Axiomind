# Axiomind Heads‑Up No‑Limit Hold'em

自己学習研究向けの Heads‑Up No‑Limit Texas Hold'em システム。オフラインで動作。

## リポ構成
- `rust/engine`: ルールと状態遷移と乱数と役判定とイベント
- `rust/cli`: コマンドライン
- `rust/web`: ローカル HTTP サーバ
- `rust/ai`: 学習と推論
- `docs`: 設計資料と決定記録
- `data`: ログとハンド履歴
- `tmp`: 作業用

## データ
- ハンド履歴 JSONL
- 集計 SQLite

## 運用
- main は常時クリーン
- 作業は feature ブランチで行う
- コミットは Conventional Commits に従う
 - バイナリ名は `axiomind`
