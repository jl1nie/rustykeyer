# keyer-core API Reference

**Rust Iambic Keyer Core Library** - `no_std` 対応組み込み向けキーヤーライブラリ

## 📋 概要

`keyer-core`は、iambicキーヤーの核心機能を提供する`no_std`対応ライブラリです。Mode A、Mode B (Curtis A)、SuperKeyerの3つのキーヤーモードをサポートし、高精度タイミング制御とHAL抽象化を提供します。

### 🎯 主要機能
- **3つのキーヤーモード**: A（基本）、B（Curtis A）、SuperKeyer（Dah優先）
- **HAL抽象化**: 異なるハードウェア間での移植性確保
- **リアルタイム制御**: 1ms精度のタイミング管理
- **型安全性**: Rustの型システムによるコンパイル時検証

## 📦 モジュール構成

```rust
pub mod types;        // データ型定義
pub mod fsm;          // 有限状態機械
pub mod controller;   // 入力制御・SuperKeyer
pub mod hal;          // HAL抽象化
```

## 🔧 基本的な使用方法

### 設定とセットアップ
```rust
use keyer_core::*;

// デフォルト設定（20 WPM、Mode A - 統一設定）
let config = keyer_core::default_config();

// カスタム設定
let config = KeyerConfig {
    mode: KeyerMode::ModeA,  // 統一デフォルト（最新推奨）  // 統一デフォルト（V203/V003互換）
    unit: Duration::from_millis(60), // 20 WPM
    char_space_enabled: true,
    debounce_ms: 10,  // 統一デバウンス（実用的ノイズ耐性）
    queue_size: 4, // 小容量MCU用
};

// FSMとキューの初期化
let mut fsm = KeyerFSM::new(config);
let (mut producer, mut consumer) = queue.split();
```

### メインループ実装
```rust
loop {
    // パドル状態読み取り + FSM更新
    let dit_pressed = /* GPIOから読み取り */;
    let dah_pressed = /* GPIOから読み取り */;
    
    let paddle = PaddleInput::new();
    paddle.update(PaddleSide::Dit, dit_pressed, system_time_ms);
    paddle.update(PaddleSide::Dah, dah_pressed, system_time_ms);
    
    fsm.update(&paddle, &mut producer);
    
    // 出力要素処理
    if let Some(element) = consumer.dequeue() {
        match element {
            Element::Dit => send_dit(config.unit),
            Element::Dah => send_dah(config.unit * 3),
            Element::CharSpace => delay(config.unit * 3),
        }
    }
}
```

## 📚 API詳細

### 🎛️ Core Types (`types`)

#### `Element` - モールス符号要素
```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Element {
    Dit,        // 短点（1 unit）
    Dah,        // 長点（3 units）
    CharSpace,  // 文字間隔（3 units）
}

impl Element {
    pub const fn duration_units(&self) -> u32;  // 単位時間数
    pub const fn is_keyed(&self) -> bool;       // キー出力要素判定
    pub const fn opposite(&self) -> Element;    // 対向要素取得
}
```

#### `KeyerMode` - キーヤー動作モード
```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyerMode {
    ModeA,      // 基本iambic（メモリなし）
    ModeB,      // Curtis A（1要素メモリ）
    SuperKeyer, // Dah優先（高度メモリ）
}
```

#### `KeyerConfig` - キーヤー設定
```rust
#[derive(Copy, Clone, Debug)]
pub struct KeyerConfig {
    pub mode: KeyerMode,
    pub char_space_enabled: bool,  // 文字間隔自動挿入
    pub unit: Duration,            // 基本単位時間
    pub debounce_ms: u32,          // デバウンス時間
    pub queue_size: usize,         // 出力キューサイズ
}

impl KeyerConfig {
    pub fn wpm(&self) -> u32;      // WPM計算
    pub fn validate(&self) -> Result<(), &'static str>;
}
```

#### `PaddleSide` - パドル識別
```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PaddleSide {
    Dit,  // Dit側パドル
    Dah,  // Dah側パドル
}
```

### 🎚️ Paddle Input (`controller`)

#### `PaddleInput` - パドル入力管理
```rust
pub struct PaddleInput {
    // 内部状態（Atomic操作）
}

impl PaddleInput {
    pub const fn new() -> Self;
    pub fn update(&self, side: PaddleSide, state: bool, now_ms: u32);
    pub fn dit(&self) -> bool;                    // Dit状態取得
    pub fn dah(&self) -> bool;                    // Dah状態取得
    pub fn both_pressed(&self) -> bool;           // 同時押下判定
    pub fn both_released(&self) -> bool;          // 同時解除判定
    pub fn current_single_element(&self) -> Option<Element>;  // 単一要素判定
    pub fn get_press_times(&self) -> (Option<u32>, Option<u32>);  // 押下時刻
}
```

#### `SuperKeyerController` - SuperKeyer制御
```rust
pub struct SuperKeyerController {
    // 内部履歴・優先度管理
}

impl SuperKeyerController {
    pub fn new() -> Self;
    pub fn update(&mut self, paddle_input: &PaddleInput);
    pub fn next_element(&mut self, squeeze: bool, last_element: Option<Element>) -> Option<Element>;
    pub fn set_memory(&mut self, element: Element);      // メモリ設定
    pub fn clear_history(&mut self);                     // 履歴クリア
}
```

### 🔄 Finite State Machine (`fsm`)

#### `KeyerFSM` - メイン状態機械
```rust
pub struct KeyerFSM {
    // 設定・状態・コントローラ
}

impl KeyerFSM {
    pub fn new(config: KeyerConfig) -> Self;
    pub fn update<P>(&mut self, paddle: &PaddleInput, producer: &mut P)
    where P: Producer<Element>;
    pub fn reset(&mut self);                             // 状態リセット
    pub fn config(&self) -> &KeyerConfig;               // 設定取得
}
```

#### `FSMState` - FSM内部状態
```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum FSMState {
    Idle,           // アイドル状態
    SendingDit,     // Dit送信中
    SendingDah,     // Dah送信中
    InterElement,   // 要素間隔
    InterCharacter, // 文字間隔
    Squeezed,       // スクイーズ処理
}
```

### 🔌 Hardware Abstraction Layer (`hal`)

#### トレイト定義
```rust
/// GPIO入力抽象化
pub trait InputPaddle {
    type Error;
    fn is_pressed(&mut self) -> Result<bool, Self::Error>;
    fn last_edge_time(&self) -> Option<Instant>;
    fn set_debounce_time(&mut self, time_ms: u32) -> Result<(), Self::Error>;
    fn enable_interrupt(&mut self) -> Result<(), Self::Error>;
    fn disable_interrupt(&mut self) -> Result<(), Self::Error>;
}

/// GPIO出力抽象化  
pub trait OutputKey {
    type Error;
    fn set_state(&mut self, state: bool) -> Result<(), Self::Error>;
    fn get_state(&self) -> Result<bool, Self::Error>;
}
```

#### 時刻・期間型
```rust
/// システム時刻（1ms精度）
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Instant(i64);

impl Instant {
    pub fn from_millis(ms: i64) -> Self;
    pub fn elapsed(&self) -> Duration;
    pub fn duration_since(&self, earlier: Instant) -> Duration;
}

/// 期間（1ms精度）
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Duration(u64);

impl Duration {
    pub const fn from_millis(ms: u64) -> Self;
    pub const fn as_millis(&self) -> u64;
    pub const fn from_secs(secs: u64) -> Self;
}
```

## 🎯 キーヤーモード詳細

### Mode A - 基本Iambic
```rust
// 特徴:
// - スクイーズ時に交互送出（DitDahDitDah...）
// - パドル解除時は即座に停止
// - メモリ機能なし
// - 初心者・精密制御向け

let config = KeyerConfig {
    mode: KeyerMode::ModeA,  // 統一デフォルト（最新推奨）
    // その他設定...
};
```

### Mode B - Curtis A
```rust
// 特徴:
// - Mode A + 1要素メモリ機能
// - スクイーズ解除時に反対要素を1回送出
// - Accu-Keyer互換
// - 最も一般的な設定

let config = KeyerConfig {
    mode: KeyerMode::ModeB,
    // その他設定...
};
```

### SuperKeyer - Dah優先
```rust
// 特徴:
// - Dah優先: 同時押下時は必ずDahを送出
// - 高度メモリ: 押下履歴に基づく制御
// - タイムスタンプ優先度判定
// - 上級者・高速運用向け

let config = KeyerConfig {
    mode: KeyerMode::SuperKeyer,
    // その他設定...
};
```

## 📊 性能特性

### メモリ使用量
```
Flash使用量: 約800B-3KB (モード・機能により変動)
RAM使用量: 約16-64B (キューサイズにより変動)
Stack使用量: 約256-512B (関数呼び出し深度)
```

### タイミング精度
```
基本精度: 1ms (HAL実装による)
WPM範囲: 5-100 WPM (推奨: 10-50 WPM)
ジッター: ±0.1ms (安定したシステムクロック前提)
応答性: <1ms (割り込み駆動時)
```

## 🧪 テスト

### テスト実行
```bash
# 全テスト実行
cargo test -p keyer-core --no-default-features

# HAL統合テスト (21テスト)
cargo test -p keyer-core --no-default-features hal_tests

# スクイーズ機能テスト
cargo test -p keyer-core --no-default-features squeeze
```

### テストカバレッジ
- ✅ **21/21 テスト合格** - 全機能動作確認
- ✅ **基本HAL機能** - GPIO・タイミング制御
- ✅ **スクイーズ動作** - 3モード完全検証  
- ✅ **境界条件** - タイミング境界・エラー処理
- ✅ **統合動作** - FSM・Controller連携

## 🔧 実装例

### CH32V003 ベアメタル実装
```rust
// hardware.rs
use keyer_core::*;

struct Ch32v003Hal;

impl InputPaddle for Ch32v003Hal {
    type Error = HalError;
    
    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        // GPIO直接読み取り
        Ok(read_dit_gpio() || read_dah_gpio())
    }
}

impl OutputKey for Ch32v003Hal {
    type Error = HalError;
    
    fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
        // GPIO直接制御
        write_key_gpio(state);
        write_led_gpio(state);
        set_sidetone_pwm(if state { 500 } else { 0 });
        Ok(())
    }
}
```

### Embassy非同期実装
```rust
// embassy_main.rs
use keyer_core::*;
use embassy_executor::Spawner;

#[embassy_executor::task]
async fn keyer_task(mut fsm: KeyerFSM, producer: Producer<Element>) {
    loop {
        let paddle = read_paddle_state().await;
        fsm.update(&paddle, &producer);
        Timer::after(Duration::from_millis(1)).await;
    }
}

#[embassy_executor::task] 
async fn sender_task(mut consumer: Consumer<Element>, unit: Duration) {
    while let Some(element) = consumer.dequeue() {
        send_element(element, unit).await;
    }
}
```

## 📖 関連ドキュメント

- **[CH32V003実装ガイド](../hardware/CH32V003_BAREMENTAL_GUIDE.md)** - ベアメタル実装詳細
- **[回路図](../hardware/CH32V003_CIRCUIT_DIAGRAM.md)** - ハードウェア回路例
- **[設計仕様](../../.kiro/specs/keyer-main/design.md)** - アーキテクチャ詳細

**keyer-coreは、Rust組み込み開発におけるHAL抽象化とリアルタイム制御の実装例を提供します。**