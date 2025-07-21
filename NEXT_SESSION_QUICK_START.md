# 次回セッション クイックスタートガイド

## 即座に確認すべきコマンド

```bash
# 1. 現在のコンパイル状況確認
cargo check -p keyer-core --features std,test-utils,embassy-time

# 2. テスト環境の問題確認
cd /home/minoru/src/rustykeyer/tests && cargo check

# 3. 具体的エラー内容確認
cd /home/minoru/src/rustykeyer/tests && cargo check 2>&1 | head -20
```

## 問題の焦点

**test_utils.rsでstdが見つからない**
- Location: `/keyer-core/src/test_utils.rs:8:9`
- Error: `use std::sync::{Arc, Mutex};`
- Root cause: feature gate不具合

## 最優先作業

### Step 1: Feature設定修正
```toml
# keyer-core/Cargo.toml
[features]
default = []
std = []
embassy-time = ["dep:embassy-time"]
test-utils = ["embassy-time"]  # stdを削除してテスト
```

### Step 2: 条件分岐修正
```rust
// test_utils.rs
#[cfg(all(feature = "test-utils", feature = "std", feature = "embassy-time"))]
pub mod virtual_time {
    #[cfg(feature = "std")]
    use std::sync::{Arc, Mutex};
    // ...
}
```

## 作業可能な代替案

もしコンパイル問題が複雑な場合:

1. **スクイーズテスト実行**: `/tests/src/squeeze_tests.rs`は独立して動作可能
2. **HALテスト実行**: 基本機能テストから継続
3. **ドキュメント整備**: 既存の動作仕様の詳細化

## 成功判定

- [ ] `cargo check -p keyer-core --features std,test-utils,embassy-time` 成功
- [ ] `cargo test` (tests環境) 成功
- [ ] スクイーズテスト実行可能

## 現在のファイル状況

✅ Ready files:
- `/tests/src/squeeze_tests.rs` - 完成済み
- `/CURRENT_ISSUES_ANALYSIS.md` - 問題分析完了
- `/SESSION_PROGRESS_SUMMARY.md` - 進捗記録

❌ Requires fix:
- `/keyer-core/src/test_utils.rs` - std依存問題
- `/keyer-core/Cargo.toml` - feature設定