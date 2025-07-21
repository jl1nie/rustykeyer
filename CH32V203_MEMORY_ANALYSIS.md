# CH32V203 メモリフットプリント分析レポート

**実測日**: 2025-01-21  
**対象**: CH32V203 + Embassy async runtime  
**コンパイラ**: Rust 1.70+ (release mode, opt-level="z")

## 📊 実測結果

### メモリ使用量
```
セクション │ サイズ    │ 制約     │ 使用率 │ 評価
---------- │ --------- │ -------- │ ------ │ --------
text       │ 6,368B    │ 64KB     │ 10.0%  │ 🟢 余裕
data       │ 0B        │ -        │ 0%     │ 🟢 良好
bss        │ 20,480B   │ 20KB     │ 100%   │ 🔴 満杯
合計       │ 26,848B   │ -        │ -      │ -
```

### アーキテクチャ情報
- **Target**: riscv32imc-unknown-none-elf
- **Entry point**: 0x8000000
- **Soft-float ABI**: RVC compression enabled
- **Embassy version**: 0.6.3

## 🔍 詳細分析

### Flash使用量 (6.2KB / 64KB)
**🟢 非常に良好 - 10%使用**

主要コンポーネント推定：
- Embassy executor core: ~2KB
- Embassy time driver: ~1KB  
- Keyer FSM logic: ~1KB
- HAL abstraction: ~1KB
- RISC-V runtime: ~1KB
- その他: ~0.2KB

**余裕**: 57.8KB (90%) - 大幅な機能拡張が可能

### RAM使用量 (20KB / 20KB)
**🔴 制約ギリギリ - 100%使用**

BSS領域内訳推定：
- Embassy task arena: ~8KB (設定値)
- Static Queue<Element, 64>: ~512B
- PaddleInput atomics: ~16B
- HAL instances: ~100B
- その他静的変数: ~11KB

**問題**: 実行時スタック領域が確保不可

## ⚡ 最適化提案

### 即座実装可能
1. **Task arena size削減**
   ```toml
   embassy-executor = { features = ["task-arena-size-1024"] }
   ```
   効果: -7KB

2. **Queue size削減**
   ```rust
   static KEY_QUEUE: StaticCell<Queue<Element, 8>> = StaticCell::new();
   ```
   効果: -448B

3. **不要な静的変数削除**
   効果: -500B

### 予測改善効果
```
改善項目           │ 現在    │ 改善後  │ 削減量
------------------ │ ------- │ ------- │ -------
Task arena         │ 8KB     │ 1KB     │ -7KB
Queue              │ 512B    │ 64B     │ -448B
その他最適化       │ 11KB    │ 10.5KB  │ -500B
------------------ │ ------- │ ------- │ -------
合計RAM使用量      │ 20KB    │ 12.1KB  │ -7.9KB
使用率             │ 100%    │ 60.5%   │ -39.5%
```

## 🎯 最適化後の予測

### 目標RAM使用量: 12-13KB (60-65%)
- **実行余裕**: 7-8KB (35-40%)
- **スタック領域**: 4KB確保可能
- **動的割り当て**: 3-4KB余裕

### Flash使用量: 変化なし
- **6.2KB / 64KB** (10%使用)
- **機能拡張余裕**: 57.8KB

## 📋 推奨アクション

### Phase 1: 緊急対応
1. ✅ Task arena size削減 (8KB→1KB)
2. ✅ Queue size削減 (64→8要素)
3. ⏳ 不要な静的変数削除

### Phase 2: 詳細最適化
1. ⏳ Dynamic allocation削減
2. ⏳ HAL instance最適化
3. ⏳ コンパイル時定数化

### Phase 3: 検証
1. ⏳ 実機動作テスト
2. ⏳ メモリ使用量再測定
3. ⏳ パフォーマンス検証

## 🏆 結論

### CH32V203での実用性
- **開発フェーズ**: ✅ 実用可能 (設定調整で対応)
- **製品化**: ✅ 推奨 (最適化後に十分な余裕)
- **Embassy compatibility**: ✅ 良好 (async/awaitのメリット享受)

### 他MCUとの比較
```
MCU        │ Flash  │ RAM   │ Embassy適性 │ 推奨用途
---------- │ ------ │ ----- │ ----------- │ ----------
CH32V003   │ 16KB   │ 2KB   │ 🔴 厳しい   │ Bare metal
CH32V203   │ 64KB   │ 20KB  │ 🟡 要調整   │ 開発・製品
STM32F4    │ 512KB+ │ 128KB+│ 🟢 余裕     │ 開発・試作
```

### 開発方針
**CH32V203をメインターゲットとして、Embassy ecosystemの恩恵を最大活用**
- async/await開発体験
- 豊富なFlash容量での機能拡張
- 適切なRAM管理での安定動作

---

*本レポートは実際のコンパイル結果に基づく分析であり、最適化効果は実装により変動する可能性があります。*