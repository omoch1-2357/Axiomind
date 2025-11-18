# STACK

## 言語とランタイム
- Rust stable
- Python 3.12
- HTML UI は静的 HTML と htmx

## ツール
- cargo rustfmt clippy
- venv pip ruff black
- cargo-deny pip-audit

## リポ構成
- rust/engine rust/cli rust/web rust/ai
- docs data tmp

## 規約
- Rust は rustfmt と clippy を適用
- Python は ruff と black を適用
- コミットは Conventional Commits
- main は常時クリーン 作業は feature ブランチ

## データ
- ハンド履歴は JSONL 詳細は ADR-0001
- 集計は SQLite 詳細は ADR-0002
