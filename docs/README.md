# 📖 Rusty Keyer Documentation

**総合ドキュメンテーション** - Rust iambic keyerの設計から実装まで

## 📋 ドキュメント構成

### 🚀 メインドキュメント
- **[README.md](../README.md)** - プロジェクト概要・クイックスタート
- **[Kiro仕様書](../.kiro/specs/keyer-main/)** - 要件・設計・実装状況

### 🔌 ハードウェア実装
- **[CH32V003ベアメタル実装ガイド](hardware/CH32V003_BAREMENTAL_GUIDE.md)** 📍 **必読**
  - 1KB Flash / 2KB RAM での完全実装
  - レジスタ直接制御・割り込み・PWM詳細
  - メモリ配分・性能測定・実機対応手順
- **[CH32V003 Implementation Guide (English)](hardware/CH32V003_BAREMENTAL_GUIDE_EN.md)**
- **[ピン配置・回路図](hardware/)** - 実装回路例

### 🦀 API仕様
- **[keyer-core API](api/)** - コアライブラリ仕様
- **[HAL抽象化](api/)** - ハードウェア抽象レイヤー
- **[FSM仕様](api/)** - 有限状態機械設計

### 📋 開発記録
- **[セッション記録](archive/)** - 実装過程の詳細記録
- **[メモリ分析](archive/)** - フットプリント最適化過程
- **[テストレポート](archive/)** - 21テスト検証結果

## 🎯 レベル別ガイド

### 🔰 初心者向け
1. **[README.md](../README.md)** - 概要把握
2. **[要件仕様](../.kiro/specs/keyer-main/requirements.md)** - 機能理解  
3. **[クイックスタート](../README.md#🚀-クイックスタート)** - ビルド・テスト

### ⚡ 実装者向け
1. **[技術設計](../.kiro/specs/keyer-main/design.md)** - アーキテクチャ理解
2. **[CH32V003ガイド](hardware/CH32V003_BAREMENTAL_GUIDE.md)** - 詳細実装
3. **[API仕様](api/)** - ライブラリ活用

### 🎛️ 上級者・カスタマイズ
1. **[HAL抽象化設計](api/)** - 移植性対応
2. **[セッション記録](archive/)** - 最適化手法学習
3. **[FSM詳細](api/)** - SuperKeyer拡張

## 🌟 注目ドキュメント

### 📍 **CH32V003 ベアメタル実装ガイド**
**究極の最適化実装** - この実装により以下を達成：

- **Flash使用量**: 1,070B（目標4KBを73%下回る）
- **RAM使用量**: 2,048B（2KB制約に完璧適合）
- **機能**: iambic keyer全機能（3モード対応）
- **性能**: 1msリアルタイム制御、21テスト合格

**技術的意義**: Rust組み込み開発における極限最適化の実践例

### 📊 実装成果まとめ

| フェーズ | 期間 | 主要成果 | ドキュメント |
|----------|------|----------|------------|
| **Phase 1** | 1日 | Embassy実装・HAL設計 | [設計書](../.kiro/specs/keyer-main/design.md) |
| **Phase 2** | 1日 | テスト統合・21テスト合格 | [テストレポート](archive/) |  
| **Phase 3** | 1日 | ベアメタル実装・極限最適化 | [実装ガイド](hardware/CH32V003_BAREMENTAL_GUIDE.md) |

**総開発期間**: 3日間  
**合格テスト**: 21/21 (100%)  
**対応MCU**: 2チップ (CH32V003/V203)  
**実装方式**: 2種類 (Embassy/ベアメタル)

## 🔧 活用方法

### プロジェクト参加者
- 実装: [技術設計書](../.kiro/specs/keyer-main/design.md) → [実装ガイド](hardware/CH32V003_BAREMENTAL_GUIDE.md)
- テスト: [テストレポート](archive/) → `cargo test`実行
- カスタマイズ: [API仕様](api/) → コア機能拡張

### 学習者・研究者
- Rust組み込み学習: [ベアメタル実装](hardware/CH32V003_BAREMENTAL_GUIDE.md)
- メモリ最適化技法: [分析レポート](archive/)
- RISC-V活用事例: [セッション記録](archive/)

### 商用利用検討者
- 製品化可能性: [実装ガイド](hardware/CH32V003_BAREMENTAL_GUIDE.md#🚀-展開可能性)
- コスト分析: ハードウェア仕様・部品表
- 技術移転: オープンソースライセンス (MIT)

---

## 📞 サポート

- **GitHub Issues**: 技術的質問・バグ報告
- **Documentation**: `cargo doc --open --package keyer-core`
- **Community**: アマチュア無線・Rust組み込みコミュニティ

**Kiro Spec-Driven Development により体系的に開発されたプロジェクトです。**