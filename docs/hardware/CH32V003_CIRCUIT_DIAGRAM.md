# CH32V003/V203 iambic Keyer 回路図

**TLP785フォトカプラー使用** - 無線機安全接続設計（両プラットフォーム対応）

## 📋 概要

CH32V003とCH32V203の両プラットフォーム対応iambicキーヤーの実装回路です。TLP785フォトカプラーを使用することで、無線機との完全電気絶縁を実現し、安全で確実なキー制御を行います。

### 🎯 設計方針
- **電気絶縁**: フォトカプラーによる無線機との完全分離
- **最小構成**: 外付け部品数最小化（7部品）
- **信頼性**: アマチュア無線機器の標準的なキー入力に対応
- **拡張性**: サイドトーン・LED表示付き

## 🔌 回路図

### 全体回路
```
                    CH32V003F4P6
                  ┌─────────────┐
    +3.3V ────────┤20 VCC   PD7├─19─┤22 ├─── Status LED (赤)
                  │              │       4.7kΩ
    GND ──────────┤10 VSS   PD6├─18─┤27 ├─── Key Output → TLP785
                  │              │       1kΩ
    Dit Paddle ───┤19 PA2   PA1├─17──── PWM Sidetone → 圧電ブザー
        (10kΩ↑)  │              │
    Dah Paddle ───┤ 9 PA3   PA0├─16
        (10kΩ↑)  │              │
                  └─────────────┘

    ピン配置:
    PA1: TIM1_CH1 (PWM Sidetone出力)
    PA2: Dit入力 (プルアップ, EXTI2)
    PA3: Dah入力 (プルアップ, EXTI3)
    PD6: Key制御出力 → TLP785
    PD7: Status LED
```

### パドル入力回路
```
Dit Paddle Input:
    +3.3V
      │
    10kΩ (Pull-up)
      │
      ├─── PA2 (CH32V003)
      │
    [Dit Paddle] ──── GND

Dah Paddle Input:
    +3.3V
      │
    10kΩ (Pull-up)
      │
      ├─── PA3 (CH32V003)
      │
    [Dah Paddle] ──── GND

動作:
- 通常時: PA2/PA3 = HIGH (3.3V)
- パドル押下時: PA2/PA3 = LOW (0V)
- EXTI割り込み: 立ち下がりエッジで検出
```

### TLP785 キー出力回路
```
Key Output Circuit:
                        TLP785
                    ┌─────────────┐
PD6 ──1kΩ──── LED-1 │2           5│ ──── Key Tip (to Radio)
              (内蔵)│             │
GND ─────────── LED-2 │3           4│ ──── Key Ring (to Radio)
                    └─────────────┘
                    
TLP785の特徴:
- 絶縁電圧: 2500Vrms (入出力間完全絶縁)
- スイッチング可能電圧: 55V max (コレクタ・エミッタ間)
- スイッチング電流: 50mA max (無線機キー入力に十分)
- 入力側順方向電流: 20mA (典型値)
- 応答時間: 18μs (typ)
- DIP-4パッケージ

計算:
順方向電圧 VF = 1.2V (typ)
順方向電流 IF = (3.3V - 1.2V) / 1kΩ = 2.1mA
→ 十分な駆動電流確保

アマチュア無線機適合性:
- 一般的なキー入力電圧: 5-12V (55V対応で安全マージン十分)
- 一般的なキー入力電流: <10mA (50mA対応で余裕あり)
- 絶縁: MCUと無線機を完全分離、安全性確保
```

### サイドトーン回路
```
Sidetone Circuit:
PA1 (PWM) ────┐
              │  圧電ブザー
              │  (600Hz共振)
              │
GND ──────────┘

または

PA1 (PWM) ──┤│──1μF──[Speaker 8Ω]──GND
            (DC cut)

動作:
- PWM周波数: 600Hz (聞き取りやすい音程)
- デューティ比: 50% (キー押下時)
- デューティ比: 0% (キー解除時)
```

### ステータスLED
```
Status LED Circuit:
PD7 ─────220Ω──── LED (赤) ──── GND

動作:
- キー押下時: LED点灯
- キー解除時: LED消灯
- 順方向電流: (3.3V - 2.0V) / 220Ω = 5.9mA
```

## 🔧 部品表

### 必須部品
| 部品 | 数量 | 型番/仕様 | 用途 |
|------|------|-----------|------|
| **CH32V003F4P6** | 1個 | TSSOP-20 | メインMCU |
| **TLP785** | 1個 | DIP-4 | フォトカプラー |
| **抵抗 1kΩ** | 1個 | 1/4W | LED駆動制限 |
| **抵抗 10kΩ** | 2個 | 1/4W | パドルプルアップ |
| **抵抗 220Ω** | 1個 | 1/4W | LED電流制限 |
| **LED 赤** | 1個 | 3mm | ステータス表示 |

### オプション部品
| 部品 | 数量 | 型番/仕様 | 用途 |
|------|------|-----------|------|
| **圧電ブザー** | 1個 | 600Hz共振 | サイドトーン |
| **コンデンサ 1μF** | 1個 | セラミック | DC cut |
| **スピーカー** | 1個 | 8Ω 0.5W | サイドトーン |

### 接続部品
| 部品 | 数量 | 型番/仕様 | 用途 |
|------|------|-----------|------|
| **3.5mmジャック** | 1個 | ステレオ | 無線機キー接続 |
| **3.5mmジャック** | 2個 | モノラル | パドル接続 |
| **電源コネクタ** | 1個 | DCジャック/USB | 3.3V電源 |

## 🔌 接続仕様

### 無線機接続 (3.5mm ステレオジャック)
```
Tip (先端): Key line (キー信号)
Ring (中間): 通常未使用 (一部機種でPTT)
Sleeve (根本): GND (共通グランド)

対応機種例:
- ICOM: IC-7300, IC-705, IC-9700
- YAESU: FT-991A, FT-710, FT-DX10
- Kenwood: TS-890S, TS-590SG
- Elecraft: K3S, KX2, KX3
```

### パドル接続 (3.5mm モノラルジャック × 2)
```
Dit Paddle:
  Tip: Dit contact
  Sleeve: GND

Dah Paddle:
  Tip: Dah contact  
  Sleeve: GND

対応パドル例:
- Bencher BY-1, BY-2
- Vibroplex Iambic
- Kent Twin Paddle
- N3ZN Keys
```

## 🔧 プリント基板レイアウト

### 基板サイズ
```
推奨サイズ: 40mm × 30mm (単面基板)
厚さ: 1.6mm
材質: FR-4
```

### レイアウト方針
```
[電源部] ─ [MCU] ─ [出力部]
    │        │       │
    └─ [LED] │   [フォトカプラー]
            │
        [パドル入力]
            │
      [サイドトーン]
```

### グランドプレーン
```
- 単面基板の場合は可能な限り銅箔面積確保
- MCU直下にGNDプレーン配置
- 高周波ノイズ対策のため短配線
```

## ⚡ 電源仕様

### 電源要件
```
入力電圧: 3.3V ± 5%
消費電流: 
  - 待機時: 5mA (typ)
  - キー動作時: 8mA (typ)
  - 最大: 15mA (LED + ブザー同時)

電源オプション:
1. ACアダプター (3.3V 100mA以上)
2. USB電源 + 3.3Vレギュレータ
3. 電池 (単3×2本 + レギュレータ)
```

### 電源安定化
```
推奨構成:
5V (USB) → AMS1117-3.3 → 3.3V
                │
              100μF ─ 0.1μF (バイパス)
```

## 🧪 動作確認・調整

### 基本動作確認
```
1. 電源投入 → LED短時間点滅 (起動確認)
2. Ditパドル → LED点灯 + キー出力
3. Dahパドル → LED点灯 + キー出力  
4. 同時押し → 交互動作確認
5. サイドトーン → 600Hz音程確認
```

### TLP785動作確認
```
測定ポイント:
1. PD6出力電圧: 0V/3.3V切り替え確認
2. LED順方向電流: 2-3mA測定
3. 出力側絶縁: 無線機GNDと基板GND間絶縁確認
4. 応答時間: <20μs確認 (オシロスコープ)
```

### 無線機接続確認
```
接続前確認:
1. 無線機キー仕様確認 (電圧・電流)
2. 極性確認 (Tip/Ring/Sleeve)
3. 絶縁確認 (マルチメーター)

動作確認:
1. 低電力でCW送信テスト
2. タイミング精度確認 (20WPM基準)
3. 連続動作安定性確認
```

## 🔒 安全上の注意

### 電気安全
- **絶縁確認**: フォトカプラーによる完全絶縁維持
- **電圧確認**: 無線機の許容電圧・電流範囲内
- **極性確認**: 接続前の極性確認必須

### RF安全
- **接地**: 基板グランドと無線機グランド分離
- **遮蔽**: 必要に応じて金属ケース使用
- **配線**: キー線の最短化

## 🔧 CH32V203 回路配置 (NEW!)

### 🏆 V203専用ピン配置

CH32V203では異なるピン配置を使用します：

```
                    CH32V203 (48pin)
        ピン配置:
        PA0: Dit入力 (プルアップ, EXTI0) ← V003のPA2
        PA1: Dah入力 (プルアップ, EXTI1) ← V003のPA3  
        PA2: Key制御出力 → TLP785      ← V003のPD6
        PA3: PWM Sidetone出力          ← V003のPA1
        
        Status LED: 任意のGPIOピン (例: PC13)
```

### 📊 プラットフォーム比較

| **信号** | **CH32V003** | **CH32V203** | **機能** |
|:--------:|:------------:|:------------:|:--------:|
| **Dit入力** | PA2 (EXTI2) | PA0 (EXTI0) | パドル検出 |
| **Dah入力** | PA3 (EXTI3) | PA1 (EXTI1) | パドル検出 |
| **Key出力** | PD6 | PA2 | TLP785制御 |
| **PWM音** | PA1 (TIM1_CH1) | PA3 (TIM1_CH3) | サイドトーン |
| **LED** | PD7 | PC13 (例) | 状態表示 |

### 🔄 両エッジ検出共通実装

両プラットフォームで統一された動作を実現：

```
共通機能:
✅ 両エッジ（立ち上がり・立ち下がり）検出
✅ パドル押下・離脱の完全追跡  
✅ 1ms精度タイミング制御
✅ TLP785による完全絶縁
✅ 600Hz PWMサイドトーン
```

## 📖 関連ドキュメント

- **[CH32V003実装ガイド](CH32V003_BAREMENTAL_GUIDE.md)** - V003ソフトウェア実装詳細
- **[CH32V203実装ガイド](CH32V203_EMBASSY_GUIDE.md)** - V203ソフトウェア実装詳細 (将来追加)
- **[API Reference](../api/keyer-core-api.md)** - ライブラリ仕様
- **[組立て手順書](CH32V003_ASSEMBLY_GUIDE.md)** - 実装手順 (将来追加)

---

## 🎯 実装成果

この回路により以下を実現：
- **完全絶縁**: フォトカプラーによる無線機保護
- **最小構成**: 7部品での完全機能実装  
- **高信頼性**: プロ仕様の絶縁・保護回路
- **コスト効率**: V003: 500円程度、V203: 800円程度
- **クロスプラットフォーム**: 用途に応じた最適な選択肢

**CH32V003/V203 + TLP785による究極のコストパフォーマンス iambic keyer実現**