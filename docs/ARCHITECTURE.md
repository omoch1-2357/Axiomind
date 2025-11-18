# ARCHITECTURE

コアは Rust のルール実装。CLI と HTML UI と Rust AI は疎結合で連携する。

## 構成
- Rust engine: ルール 状態遷移 乱数 役判定 イベント
- Rust cli: プレイとシミュレーションと検証
- Rust web: ローカル HTTP サーバ UI は HTML と htmx
- Rust ai: 学習と推論 将来は gRPC 連携を追加可能

## データフロー
1. engine は各ハンド終了時に HandRecord を JSONL に追記 data/hands
2. 集計は SQLite に保存 data/db.sqlite
3. cli は JSONL を読み集計や検証を実行
4. web は engine のイベントを購読し UI に配信 SSE を使用

## ハンド履歴 JSONL
- 単位 1 行 1 ハンド
- 文字コード UTF-8 改行 LF

### レコード例
```json
{
  "hand_id": "20250829-000001",
  "seed": 42,
  "level": 3,
  "sb": 100,
  "bb": 200,
  "button": "P2",
  "players": [
    {"id": "P1", "stack_start": 20000},
    {"id": "P2", "stack_start": 20000}
  ],
  "actions": [
    {"street": "preflop", "actor": "P1", "action": "call", "amount": 200},
    {"street": "preflop", "actor": "P2", "action": "check"}
  ],
  "board": ["Ah", "Kd", "7c", "2s", "2d"],
  "showdown": [
    {"player": "P1", "cards": ["As", "Ad"], "won": false, "amount": 0},
    {"player": "P2", "cards": ["Kh", "Kc"], "won": true,  "amount": 500}
  ],
  "net_result": {"P1": -500, "P2": 500},
  "end_reason": "showdown",
  "ts": "2025-08-29T00:00:00Z"
}
```

## 境界
- ルールと状態は engine に閉じる I O と UI は外側
- AI 連携は当初はファイル連携 将来は gRPC を追加可能
