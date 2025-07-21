# 📟 Iambic Keyer 技術仕様書

## 1. 概要
本仕様は、Rust + Embassy を用いた iambic キーヤーの設計および動作仕様を定義します。リアルタイム・省メモリ・モード拡張性・高応答性を重視した構造により、アマチュア無線運用や組み込み通信に適した CW 制御を実現します。

対応モード:
- **Mode A**: 単純な iambic スクイーズ送出
- **Mode B**: スクイーズ解除時に反対符号を 1 回送出（Accu-Keyer風）
- **SuperKeyer**: Dah優先＋押下履歴制御

補助機能:
- FSM（有限状態機械）による状態遷移管理
- GPIO + タイマー制御による符号出力
- CharSpace（文字間スペース）制御（トグル対応）

---

## 2. 入力管理と割り込み
- パドル押下（Dit/Dah）は GPIO エッジ割り込みで検知（ISR）
- ISR は送信キューを操作せず、押下状態と時刻のみ記録
- 状態は evaluator タスクに非同期で受け渡し

---

## 3. FSM 設計
```rust
enum FSMState {
  Idle,
  DitHold,
  DahHold,
  Squeeze(Element),
  MemoryPending(Element),
  CharSpacePending(Instant),
}
```

- 状態遷移は unit/4 周期で更新
- Squeeze: Dit+Dah 同時押下時に交互送出
- MemoryPending: ModeB/SK モードで解除後に反対符号送出
- CharSpacePending: 全解除 → unit×3 待機 → 次送信開始

## 4. SuperKeyerController 構造
```rust
struct SuperKeyerController {
  dit_time: Option<Instant>,
  dah_time: Option<Instant>,
}
```
- Dah優先判断：押下時刻で比較
- メモリ送出：スクイーズ解除後に反対符号挿入
- FSM から必要なタイミングで使用・クリア

## 5. 出力制御（Sender Task）
```rust
enum Element {
  Dit,
  Dah,
  CharSpace,
}
```
-Dit → unit 秒オン
- Dah → unit×3 秒オン
- CharSpace → unit×3 秒間の休止（出力なし）
- 各送出後に unit 秒の inter-element スペースを挿入


## 6. CharSpace の制御仕様
- KeyerConfig.char_space_enabled によって ON/OFF 切替可能
- 一度パドルが完全に解除された後、
- 3unit 経過後の入力 → 送出開始（間合いを表現）
- 早すぎる入力 → 3unit 経過まで待機
- CharSpacePending 中も入力を常時監視

## 7. モード別比較
| モード | スクイーズ解除時 | Dah優先 | メモリ送出 | 特徴 | 
| Mode A | Idleへ即復帰 | × | × | 最も基本的な挙動 | 
| Mode B | 反対符号を1回送出 | × | ○ | Accu-Keyer風挙動 | 
| SuperKeyer | 優先判断＋履歴付き送出 | ○ | ○ | Dah優先、履歴に基づく柔軟制御 | 




