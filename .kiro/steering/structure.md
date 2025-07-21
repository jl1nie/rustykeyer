# Rustykeryer - Project Structure

## Root Directory Organization
```
rustykeryer/
├── src/                    # ソースコードディレクトリ（Cargoプロジェクト標準）
├── .kiro/                  # Kiro開発フレームワーク設定
│   └── steering/          # プロジェクトステアリングドキュメント
├── target/                # ビルド成果物（.gitignore対象）
├── Cargo.toml            # プロジェクト定義・依存関係
├── Cargo.lock            # 依存関係ロックファイル
├── memory.x              # リンカスクリプト（メモリレイアウト）
├── build.rs              # ビルドスクリプト（必要に応じて）
├── .cargo/               # Cargo設定
│   └── config.toml      # ターゲット・リンカ設定
├── CLAUDE.md             # Kiro-style開発仕様
├── keyer.md              # Iambicキーヤー技術仕様書
├── main.rs               # エントリーポイント（src外の特殊配置）
└── keyer.rs              # キーヤーコアロジック（src外の特殊配置）
```

## Subdirectory Structures
### .kiro/ディレクトリ
```
.kiro/
├── steering/             # プロジェクト基本情報
│   ├── product.md       # プロダクト概要
│   ├── tech.md          # 技術スタック
│   └── structure.md     # プロジェクト構造（本ファイル）
├── specs/               # 機能仕様（フィーチャー別）
│   └── [feature-name]/
│       ├── spec.json    # 承認状態管理
│       ├── requirements.md
│       ├── design.md
│       └── tasks.md
└── commands/            # カスタムスラッシュコマンド
```

### src/ディレクトリ（標準構成時）
```
src/
├── lib.rs              # ライブラリクレート（テスト可能なロジック）
├── bin/                # バイナリクレート
├── drivers/            # ハードウェアドライバ
├── tasks/              # 非同期タスクモジュール
└── config/             # 設定・定数定義
```

## Code Organization Patterns
### モジュール構成
- **エントリーポイント**：`main.rs`でハードウェア初期化とタスク起動
- **コアロジック**：`keyer.rs`にキーヤー固有のロジックを集約
- **状態管理**：FSM（有限状態機械）パターンでenum型による状態遷移
- **非同期タスク**：各機能を独立したasyncタスクとして実装
- **メッセージパッシング**：タスク間通信はSignal/Channelで実現

### レイヤー設計
1. **ハードウェア層**：HALによる抽象化、割り込みハンドラ
2. **制御層**：FSM、入力処理、タイミング制御
3. **アプリケーション層**：キーヤーモード実装、ユーザーインターフェース

## File Naming Conventions
### ソースファイル
- **モジュール名**：snake_case（例：`paddle_input.rs`）
- **型定義ファイル**：型名と同じ（例：`super_keyer.rs`）
- **タスクファイル**：`_task`サフィックス（例：`sender_task.rs`）

### ドキュメント
- **仕様書**：機能名.md（例：`keyer.md`）
- **Kiro仕様**：`.kiro/specs/[feature-name]/`配下に配置

### 設定ファイル
- **Cargo設定**：`Cargo.toml`（プロジェクトルート）
- **リンカ設定**：`memory.x`、`.cargo/config.toml`

## Import Organization
### 標準的なインポート順序
```rust
// 1. 標準ライブラリ（no_stdでは不使用）
use core::sync::atomic::{AtomicBool, Ordering};

// 2. 外部クレート
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

// 3. ローカルモジュール
use crate::keyer::{KeyerMode, PaddleInput};

// 4. selfインポート
use self::State::*;
```

### モジュール公開規則
- **pub**：外部公開API
- **pub(crate)**：クレート内公開
- **pub(super)**：親モジュールへの公開
- **private**：モジュール内限定（デフォルト）

## Key Architectural Principles
### 設計原則
1. **ゼロコスト抽象化**：実行時オーバーヘッドを避ける設計
2. **静的メモリ確保**：動的メモリ確保を使用しない
3. **割り込み安全性**：critical sectionでの排他制御
4. **型安全性**：Rustの型システムを活用した設計時保証
5. **テスタビリティ**：ハードウェア依存部分の抽象化

### コーディング規約
- **エラー処理**：`Result<T, E>`型、panicは最小限に
- **ライフタイム**：明示的な指定で所有権を明確化
- **非同期処理**：`async`/`await`でブロッキングを回避
- **定数定義**：マジックナンバーは`const`で定義
- **ドキュメント**：公開APIには必ずdocコメント

### 並行処理パターン
- **タスク分離**：機能単位でタスクを分割
- **メッセージパッシング**：共有状態より優先
- **Signal/Channel**：タスク間通信の基本
- **Mutex回避**：可能な限りロックフリー設計