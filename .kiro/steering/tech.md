# Rustykeryer - Technology Stack

## Architecture
- **アーキテクチャパターン**：イベント駆動型非同期アーキテクチャ
- **システム設計**：組み込みモノリシック、タスクベース並行処理
- **メモリモデル**：静的メモリ確保、no_std環境（ヒープ不使用）
- **並行処理モデル**：協調的マルチタスキング（Embassy executor）

## Frontend
該当なし（組み込みファームウェアプロジェクト）

## Backend
### プログラミング言語
- **言語**：Rust（安定版）
- **環境**：no_std、no_main
- **ターゲット**：RISC-V 32ビット（rv32ec）

### 主要フレームワーク・ライブラリ
- **非同期ランタイム**：Embassy 0.1.0
  - `embassy-executor`：非同期タスクスケジューラ
  - `embassy-time`：非同期タイマー・遅延
  - `embassy-sync`：非同期プリミティブ（Signal）
- **ハードウェア抽象化**：ch32v0-hal（GPIO、割り込み、クロック）
- **データ構造**：heapless 0.8（静的キュー）
- **割り込み管理**：cortex-m、cortex-m-rt

## Development Environment
### 必須ツール
- **Rustツールチェーン**：rustup、cargo
- **ターゲット追加**：`rustup target add riscv32ec-unknown-none-elf`
- **ビルドシステム**：Cargo
- **フラッシャー**：OpenOCD、wchisp、またはch32v系フラッシュツール

### 推奨エディタ設定
- **VSCode拡張機能**：rust-analyzer、cortex-debug
- **デバッガー**：GDB（riscv32-unknown-elf-gdb）

## Common Commands
```bash
# ビルド
cargo build --release

# ターゲット向けビルド
cargo build --target riscv32ec-unknown-none-elf --release

# サイズ最適化ビルド
cargo build --release --features size-opt

# クリーン
cargo clean

# 依存関係更新
cargo update

# ドキュメント生成
cargo doc --no-deps --open

# テスト（ホスト環境でのユニットテスト）
cargo test --lib
```

## Environment Variables
```bash
# デバッグビルド用
CARGO_PROFILE_DEV_OPT_LEVEL=2

# リリースビルド最適化
CARGO_PROFILE_RELEASE_LTO=true
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1

# ログレベル（開発時）
DEFMT_LOG=trace
```

## Port Configuration
### GPIO割り当て
- **PA0**：Dit入力（EXTI0割り込み）
- **PA1**：Dah入力（EXTI1割り込み）  
- **PA2**：キー出力（プッシュプル）

### クロック設定
- **システムクロック**：48MHz（HSI由来）
- **APBクロック**：48MHz

### メモリマップ
- **Flash**：0x00000000から（16KB）
- **RAM**：0x20000000から（2KB）
- **スタックサイズ**：512バイト（設定可能）

## Build Configuration
### Cargo.tomlプロファイル
```toml
[profile.release]
opt-level = "z"      # サイズ最適化
lto = true           # リンク時最適化
codegen-units = 1    # 単一コンパイル単位
strip = true         # シンボル削除
```

### メモリレイアウト
- **memory.x**でリンカスクリプト定義
- スタック・ヒープサイズはビルド時に静的確保