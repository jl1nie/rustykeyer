# Keyer-Main - Implementation Tasks

> ✅ **実装完了** - 21タスク全て完了しました！
> 
> **実装サマリー**:
> - ✅ **Phase 1-6**: 全6フェーズ完了 (21/21 タスク)
> - ✅ **keyer-core**: no_std共通ライブラリ完成
> - ✅ **firmware**: Embassy非同期タスク実装完成
> - ✅ **コンパイル**: 全プロジェクト成功
> 
> **更新履歴**:
> - 2025-01-21: 初版作成（設計書 v1.0 ベース）
> - 2025-01-21: **全タスク実装完了** - 21/21タスク達成

## 📋 実装概要

本タスクリストは、承認済みの設計仕様に基づいて、Iambic KeyerのRust/Embassy実装を段階的に進めるための詳細なタスク分解です。

**実装フェーズ**:
1. **Phase 1**: コア構造体とデータ型定義 (Tasks 1-4)
2. **Phase 2**: 入力管理と割り込み処理 (Tasks 5-7)
3. **Phase 3**: FSM（有限状態機械）実装 (Tasks 8-12)
4. **Phase 4**: 出力制御とメッセージキュー (Tasks 13-15)
5. **Phase 5**: モード別ロジック実装 (Tasks 16-18)
6. **Phase 6**: 統合とテスト (Tasks 19-21)

## 🔧 Phase 1: コア構造体とデータ型定義

### Task 1: 基本データ型の完成 ✅
- [x] `Element` enum に `CharSpace` バリアント追加
  ```rust
  #[derive(Copy, Clone, PartialEq, Debug)]
  pub enum Element {
      Dit,
      Dah,
      CharSpace,
  }
  ```
- [x] `FSMState` enum の全状態バリアント実装確認
- [x] `KeyerMode`, `PaddleSide` の完全性確認
- [x] 全構造体にDerive traitsを適切に追加 (`Debug`, `Clone`, `PartialEq` など)

### Task 2: KeyerConfig 構造体の拡張 ✅
- [x] 設定可能パラメータの完全実装
  ```rust
  pub struct KeyerConfig {
      pub mode: KeyerMode,
      pub char_space_enabled: bool,
      pub unit: Duration,
      pub debounce_ms: u64,        // 追加
      pub queue_size: usize,       // 追加
  }
  ```
- [x] デフォルト値実装 (`Default` trait)
- [x] 設定値バリデーション機能の追加

### Task 3: PaddleInput 構造体の機能完成 ✅
- [x] `both_pressed()` メソッドの実装
- [x] `both_released()` メソッドの実装
- [x] `get_press_times()` メソッドの実装（SuperKeyer モード用）
- [x] デバウンス定数を設定可能にする
- [x] 単体テスト関数の追加

### Task 4: SuperKeyerController の完全実装 ✅
- [x] `record_press()` メソッドの実装
- [x] `determine_priority()` メソッドの実装（Dah優先ロジック）
- [x] `clear_history()` メソッドの実装
- [x] `should_send_memory()` メソッドの実装
- [x] Instant の取得と比較ロジックの確立

## 🔄 Phase 2: 入力管理と割り込み処理

### Task 5: 割り込みハンドラの実装 ✅
- [x] EXTI0 handler (Dit入力) の実装
  ```rust
  #[interrupt]
  fn EXTI0() {
      // GPIO state read + PaddleInput.update()
  }
  ```
- [x] EXTI1 handler (Dah入力) の実装
- [x] 割り込み優先度の設定
- [x] GPIO設定（プルアップ、エッジ検出）の実装
- [x] ISR実行時間の最適化（<5μs目標）

### Task 6: GPIO HAL抽象化層の実装 ✅
- [x] `KeyerHAL` trait の定義
  ```rust
  pub trait KeyerHAL {
      fn read_dit_pin(&self) -> bool;
      fn read_dah_pin(&self) -> bool;
      fn set_key_output(&mut self, state: bool);
  }
  ```
- [x] STM32F4 用 HAL実装
- [x] エラーハンドリングの追加
- [x] テスト用モック HAL の実装

### Task 7: デバウンス処理の改良 ✅
- [x] ソフトウェアデバウンスアルゴリズムの改良
- [x] エッジ検出の正確性向上
- [x] ノイズ耐性の強化
- [x] デバウンス時間の動的調整機能

## 🎯 Phase 3: FSM（有限状態機械）実装

### Task 8: FSM状態遷移エンジンの実装 ✅
- [x] `KeyerFSM` 構造体の実装
  ```rust
  pub struct KeyerFSM {
      state: FSMState,
      config: KeyerConfig,
      super_keyer: SuperKeyerController,
  }
  ```
- [x] `update()` メソッドの実装（メインループ）
- [x] 状態遷移ログの追加（デバッグ用）
- [x] 状態不正遷移のエラーハンドリング

### Task 9: Idle状態の処理実装 ✅
- [x] Idle → DitHold 遷移ロジック
- [x] Idle → DahHold 遷移ロジック
- [x] Idle → Squeeze 遷移ロジック（同時押下検出）
- [x] CharSpacePending → Idle 遷移（タイムアウト処理）

### Task 10: Hold状態（DitHold/DahHold）の処理実装 ✅
- [x] Hold → Squeeze 遷移（追加パドル押下）
- [x] Hold → Idle 遷移（パドル解除）
- [x] 要素送出開始のタイミング制御
- [x] Hold状態中の追加入力処理

### Task 11: Squeeze状態の処理実装 ✅
- [x] 交互送出のロジック実装
- [x] 現在送出中要素の記憶
- [x] Squeeze → MemoryPending 遷移（Mode B/SuperKeyer）
- [x] Squeeze → Idle 遷移（Mode A）
- [x] 要素切り替えタイミングの制御

### Task 12: MemoryPending状態とCharSpacePending状態の実装 ✅
- [x] MemoryPending の反対要素送出ロジック
- [x] CharSpacePending の3unit待機処理
- [x] 待機中の新規入力処理
- [x] タイムアウト処理とタイマー管理

## 📤 Phase 4: 出力制御とメッセージキュー

### Task 13: メッセージキューの実装 ✅
- [x] SPSC キューの設定（heapless::spsc）
  ```rust
  static KEY_QUEUE: StaticCell<Queue<Element, 64>> = StaticCell::new();
  ```
- [x] Producer/Consumer の分離
- [x] キューオーバーフローのハンドリング
- [x] 統計情報の収集（送出済み要素数など）

### Task 14: Sender Task の実装 ✅
- [x] `sender_task` 非同期関数の実装
  ```rust
  #[embassy_executor::task]
  async fn sender_task(mut consumer: Consumer<Element, 64>, mut key_pin: impl OutputPin) {
      // Element受信 → GPIO制御 → タイミング待機
  }
  ```
- [x] Dit送出（unit時間ON + unit時間OFF）
- [x] Dah送出（3×unit時間ON + unit時間OFF）
- [x] CharSpace送出（3×unit時間 待機のみ）
- [x] inter-element space の正確な実装

### Task 15: タイミング制御の精密化 ✅
- [x] Embassy Timer を使用した高精度待機
- [x] unit時間の動的変更対応
- [x] ジッター最小化
- [x] リアルタイム性の確保

## 🎛️ Phase 5: モード別ロジック実装

### Task 16: Mode A の実装 ✅
- [x] 基本的なiambic動作
- [x] スクイーズ解除時の即座Idle復帰
- [x] メモリ機能なしの確認
- [x] シンプルな状態遷移の確保

### Task 17: Mode B の実装 ✅
- [x] Accu-Keyer風の動作実装
- [x] スクイーズ解除時の反対要素1回送出
- [x] MemoryPending状態の活用
- [x] Mode A との動作差異の確認

### Task 18: SuperKeyer モードの実装 ✅
- [x] Dah優先ロジックの実装
- [x] 押下履歴に基づく優先判断
- [x] 複雑なメモリ送出制御
- [x] SuperKeyerController との連携

## 🔬 Phase 6: 統合とテスト

### Task 19: evaluator_fsm タスクの統合実装 ✅
- [x] `evaluator_fsm` 非同期タスクの実装
  ```rust
  #[embassy_executor::task]
  async fn evaluator_fsm(
      paddle: &'static PaddleInput,
      producer: Producer<Element, 64>,
      config: KeyerConfig,
  ) {
      // unit/4 周期でのFSM更新
  }
  ```
- [x] PaddleInput からの状態読取り
- [x] FSM更新とキュー送信の連携
- [x] ポーリング周期の最適化（unit/4）

### Task 20: メイン関数の実装 ✅
- [x] Embassy executor の設定
- [x] 静的リソースの初期化
- [x] GPIO ピンの設定と割り当て
- [x] タスク起動とエラーハンドリング
- [x] `run_keyer()` 関数の実装

### Task 21: テストとデバッグ機能 ✅
- [x] ログ出力機能の追加（defmt使用）
- [x] 単体テスト関数の実装
- [x] 統合テストシナリオの作成
- [x] パフォーマンス測定機能
- [x] メモリ使用量の監視
- [x] 実機でのテスト実行

## 📊 実装指標

### パフォーマンス目標
- **割り込み応答時間**: < 10μs
- **ISR実行時間**: < 5μs
- **状態遷移遅延**: < unit/4
- **メモリ使用量**: < 2KB (RAM)
- **タスクスタック**: evaluator(256B), sender(128B)

### 品質指標
- **デバウンス精度**: 10ms ± 1ms
- **タイミング精度**: ±1% (unit時間)
- **状態遷移正確性**: 100%（テストケース）
- **キューオーバーフロー**: 0回（正常動作時）

## 🏁 完成条件

全タスクが完了し、以下の条件を満たした時点で実装完了とする：

1. **機能要件**: 3つのモード（A/B/SuperKeyer）が仕様通り動作
2. **非機能要件**: パフォーマンス指標をすべて満たす
3. **テスト**: 全単体テスト・統合テストが成功
4. **品質**: メモリリーク、デッドロック、データ競合がない
5. **ドキュメント**: コードコメントと使用方法が完備

---

> **実装開始準備**
> - すべてのタスクは順次実行推奨（依存関係に注意）
> - 各フェーズ完了時に動作確認を実施
> - 問題発見時は該当タスクに戻って修正