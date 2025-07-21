# 現在の状況分析と解決計画

## 現在の問題状況

### 1. コンパイルエラーの状況
- **keyer-core単体**: ✅ 正常にコンパイル可能
- **test環境**: ❌ stdモジュールが見つからないエラー

### 2. 具体的なエラー内容
```
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `std`
 --> keyer-core/src/test_utils.rs:8:9
  |
8 |     use std::sync::{Arc, Mutex};
  |         ^^^ use of unresolved module or unlinked crate `std`
```

### 3. 根本原因の分析

#### Feature設定の矛盾
- `tests/Cargo.toml`: `keyer-core = { path = "../keyer-core", features = ["std", "test-utils", "embassy-time"] }`
- `keyer-core/Cargo.toml`: `test-utils = ["std", "embassy-time"]`
- **問題**: no_std環境でstdを参照しようとしている

#### モジュール条件分岐の問題
- `test_utils.rs`でstdとheaplessの使い分けが不適切
- feature gateが正しく機能していない

## 解決計画

### Phase 1: Feature設定の統一 (優先度: 高)
1. keyer-coreのfeature体系を整理
2. std/no_std環境の明確な分離
3. test-utilsの依存関係修正

### Phase 2: test_utils.rsの修正 (優先度: 高)
1. stdを使用する部分の条件分岐修正
2. heaplessとstdの適切な使い分け
3. embassy-timeの条件付き使用

### Phase 3: テスト環境の検証 (優先度: 中)
1. コンパイル成功の確認
2. HALテストの実行
3. スクイーズテストの実装継続

## 具体的な修正手順

### Step 1: Feature体系の再設計
```toml
# keyer-core/Cargo.toml
[features]
default = []
std = []
embassy-time = ["dep:embassy-time"]
test-utils = ["embassy-time"]  # stdは必要に応じて有効化
```

### Step 2: 条件分岐の修正
- `#[cfg(feature = "std")]`でstd使用部分を囲む
- `#[cfg(not(feature = "std"))]`でno_std代替実装
- embassy-timeの条件付き使用を統一

### Step 3: テスト実行確認
- `cargo test -p keyer-core --features std,test-utils,embassy-time`
- `cargo test` (tests環境)

## 期待される結果
1. keyer-coreがstd/no_std両環境でコンパイル可能
2. テスト環境でのコンパイル成功
3. HALテストとスクイーズテストの実装継続が可能

## リスク要因
- feature gateの複雑さによる新たなコンパイルエラー
- existing codeとの互換性問題
- embassy-timeの依存関係問題

## 成功指標
- [ ] keyer-core単体ビルド成功
- [ ] tests環境ビルド成功
- [ ] HALテスト実行可能
- [ ] 既存機能の回帰なし