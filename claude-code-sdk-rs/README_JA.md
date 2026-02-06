# nexus-claude - Claude Code SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/nexus-claude.svg)](https://crates.io/crates/nexus-claude)
[![Documentation](https://docs.rs/nexus-claude/badge.svg)](https://docs.rs/nexus-claude)
[![License](https://img.shields.io/crates/l/nexus-claude.svg)](LICENSE)

Claude Code CLIと対話するためのRust SDKです。シンプルなクエリインターフェースと完全なインタラクティブクライアント機能を提供しています。

> **v0.5.0**: 🎉 **Python SDK v0.1.14と100%機能同等** - CLI自動ダウンロードと永続メモリ対応！

> **Fork 通知**：このプロジェクトは [ZhangHanDong/claude-code-api-rs](https://github.com/ZhangHanDong/claude-code-api-rs)（`cc-sdk`）のフォークで、永続メモリ機能を追加しています。

## 機能

- 🚀 **シンプルクエリインターフェース** - `query()` 関数による一度きりのクエリ
- 💬 **インタラクティブクライアント** - コンテキストを保持したステートフルな会話
- 🔄 **ストリーミングサポート** - リアルタイムメッセージストリーミング
- 🛑 **中断機能** - 実行中の操作をキャンセル
- 🔧 **完全な設定** - Python SDKと同等の包括的なオプション
- 📦 **型安全性** - serdeによる強い型付けサポート
- ⚡ **非同期/待機** - Tokioベースの非同期操作
- 🔒 **コントロールプロトコル** - パーミッション、フック、MCPサーバーの完全サポート
- 💰 **トークン最適化** - コスト最小化と使用量追跡の組み込みツール
- 📥 **CLI自動ダウンロード** - 見つからない場合に自動ダウンロード（v0.4.0+）
- 📁 **ファイルチェックポイント** - 会話の任意の時点にファイル変更を巻き戻し（v0.4.0+）
- 📊 **構造化出力** - レスポンスのJSONスキーマ検証（v0.4.0+）
- 🧠 **永続メモリ** - 会話を保存してインデックス化し、将来の取得に使用（v0.5.0+）

## Python SDK機能同等（v0.4.0）

このRust SDKは公式Python `claude-agent-sdk` v0.1.14と**100%機能同等**を達成：

| 機能 | Python SDK | Rust SDK | 状態 |
|------|-----------|----------|------|
| シンプルクエリAPI | ✅ | ✅ | ✅ 同等 |
| インタラクティブクライアント | ✅ | ✅ | ✅ 同等 |
| メッセージストリーミング | ✅ | ✅ | ✅ 同等 |
| `tools`（ベースツールセット） | ✅ | ✅ | ✅ 同等 |
| `permission_mode` | ✅ | ✅ | ✅ 同等 |
| `max_budget_usd` | ✅ | ✅ | ✅ 同等 |
| `fallback_model` | ✅ | ✅ | ✅ 同等 |
| `output_format`（構造化） | ✅ | ✅ | ✅ 同等 |
| `enable_file_checkpointing` | ✅ | ✅ | ✅ 同等 |
| `rewind_files()` | ✅ | ✅ | ✅ 同等 |
| `sandbox` | ✅ | ✅ | ✅ 同等 |
| `plugins` | ✅ | ✅ | ✅ 同等 |
| `betas`（SDKベータ機能） | ✅ | ✅ | ✅ 同等 |
| パーミッションコールバック | ✅ | ✅ | ✅ 同等 |
| フックコールバック | ✅ | ✅ | ✅ 同等 |
| MCPサーバー（全タイプ） | ✅ | ✅ | ✅ 同等 |
| 内蔵/自動CLI | ✅（内蔵） | ✅（自動ダウンロード） | ✅ 同等 |

> **注意**: `user`（OS setuid）のみプラットフォーム/権限の要件により未実装。

## インストール

`Cargo.toml` に以下を追加：

```toml
[dependencies]
nexus-claude = "0.5.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

### 永続メモリ付き

```toml
[dependencies]
nexus-claude = { version = "0.5.0", features = ["memory"] }
```

### CLI自動ダウンロード（デフォルト有効）

SDKはClaude Code CLIが見つからない場合に自動ダウンロードします：

```rust
let options = ClaudeCodeOptions::builder()
    .auto_download_cli(true)  // デフォルトで有効
    .build();
```

CLIはプラットフォーム固有の場所にキャッシュされます：
- **macOS**: `~/Library/Caches/nexus-claude/cli/`
- **Linux**: `~/.cache/nexus-claude/cli/`
- **Windows**: `%LOCALAPPDATA%\nexus-claude\cli\`

自動ダウンロードを無効にする場合：

```toml
[dependencies]
nexus-claude = { version = "0.5.0", default-features = false }
```

## 前提条件

Claude Code CLIはSDKにより**自動ダウンロード**されます（v0.4.0+）。

手動インストール：

```bash
npm install -g @anthropic-ai/claude-code
```

## サポートされるモデル（2025年）

SDKは2025年最新のClaudeモデルをサポート：

### 最新モデル
- **Opus 4.5** - 最も高性能なモデル
  - 完全名：`"claude-opus-4-5-20251101"`
  - 別名：`"opus"`（推奨 - 最新Opusを使用）

- **Sonnet 4.5** - バランスの取れたパフォーマンス
  - 完全名：`"claude-sonnet-4-5-20250929"`
  - 別名：`"sonnet"`（推奨 - 最新Sonnetを使用）

### 前世代モデル
- **Claude 3.5 Sonnet** - `"claude-3-5-sonnet-20241022"`
- **Claude 3.5 Haiku** - `"claude-3-5-haiku-20241022"`（最速）

### コードでのモデル使用

```rust
use nexus_claude::{query, ClaudeCodeOptions, Result};

// Opus 4.5を使用（別名推奨）
let options = ClaudeCodeOptions::builder()
    .model("opus")  // または "claude-opus-4-5-20251101" で指定
    .build();

// Sonnet 4.5を使用（別名推奨）
let options = ClaudeCodeOptions::builder()
    .model("sonnet")  // または "claude-sonnet-4-5-20250929" で指定
    .build();

let mut messages = query("プロンプト", Some(options)).await?;
```

## クイックスタート

### シンプルクエリ（一度きり）

```rust
use nexus_claude::{query, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut messages = query("2 + 2はいくつですか？", None).await?;

    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }

    Ok(())
}
```

### インタラクティブクライアント

```rust
use nexus_claude::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // メッセージを送信してレスポンスを受信
    let messages = client.send_and_receive(
        "Pythonのウェブサーバーを書いてください".to_string()
    ).await?;

    // レスポンスを処理
    for msg in &messages {
        match msg {
            nexus_claude::Message::Assistant { message } => {
                println!("Claude: {:?}", message);
            }
            _ => {}
        }
    }

    // フォローアップを送信
    let messages = client.send_and_receive(
        "async/awaitを使うようにしてください".to_string()
    ).await?;

    client.disconnect().await?;
    Ok(())
}
```

### 永続メモリ付き

```rust
use nexus_claude::memory::{MemoryIntegrationBuilder, ContextInjector, MemoryConfig};

// 会話追跡用のメモリマネージャを作成
let mut manager = MemoryIntegrationBuilder::new()
    .enabled(true)
    .cwd("/projects/my-app")
    .url("http://localhost:7700")  // Meilisearch URL
    .min_relevance_score(0.3)
    .max_context_items(5)
    .build();

// 会話中のメッセージとツール呼び出しを記録
manager.record_user_message("JWT 認証を実装するには？");
manager.process_tool_call("Read", &serde_json::json!({
    "file_path": "/projects/my-app/src/auth.rs"
}));
manager.record_assistant_message("認証モジュールを分析しました...");
```

### 高度な使用方法

```rust
use nexus_claude::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // レスポンスを待たずにメッセージを送信
    client.send_message("円周率を100桁まで計算してください".to_string()).await?;

    // 他の作業を実行...

    // 準備ができたらレスポンスを受信（Resultメッセージで停止）
    let messages = client.receive_response().await?;

    // 長時間実行される操作をキャンセル
    client.send_message("10000語のエッセイを書いてください".to_string()).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    client.interrupt().await?;

    client.disconnect().await?;
    Ok(())
}
```

## 設定オプション

```rust
use nexus_claude::{ClaudeCodeOptions, PermissionMode};

let options = ClaudeCodeOptions::builder()
    .system_prompt("あなたは役立つコーディングアシスタントです")
    .model("claude-3-5-sonnet-20241022")
    .permission_mode(PermissionMode::AcceptEdits)
    .max_turns(10)
    .max_thinking_tokens(10000)
    .allowed_tools(vec!["read_file".to_string(), "write_file".to_string()])
    .cwd("/path/to/project")
    .build();
```

### コントロールプロトコル（v0.1.12+）

Python Agent SDK と整合する新しいランタイム制御とオプション：

- `Query::set_permission_mode("acceptEdits" | "default" | "plan" | "bypassPermissions")`
- `Query::set_model(Some("sonnet"))` または `set_model(None)` で解除
- `ClaudeCodeOptions::builder().include_partial_messages(true)` で部分チャンクを有効化
- `Query::stream_input(stream)` は送信完了後に `end_input()` を自動呼び出し

### Agent ツール & MCP

- ツールの許可/禁止リスト：`ClaudeCodeOptions` の `allowed_tools` / `disallowed_tools`
- 権限モード：`PermissionMode::{Default, AcceptEdits, Plan, BypassPermissions}`
- 実行時承認：`CanUseTool` を実装して `PermissionResult::{Allow,Deny}` を返す
- MCP サーバー：`options.mcp_servers` で構成（stdio/http/sse/sdk）。SDK は `--mcp-config` に打包

## API リファレンス

### `query()`

一度きりの対話のためのシンプルでステートレスなクエリ関数。

```rust
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeCodeOptions>
) -> Result<impl Stream<Item = Result<Message>>>
```

### `InteractiveClient`

ステートフルでインタラクティブな会話のためのメインクライアント。

#### メソッド

- `new(options: ClaudeCodeOptions) -> Result<Self>` - 新しいクライアントを作成
- `connect() -> Result<()>` - Claude CLIに接続
- `send_and_receive(prompt: String) -> Result<Vec<Message>>` - メッセージを送信して完全なレスポンスを待つ
- `send_message(prompt: String) -> Result<()>` - 待機せずにメッセージを送信
- `receive_response() -> Result<Vec<Message>>` - Resultメッセージまでメッセージを受信
- `interrupt() -> Result<()>` - 実行中の操作をキャンセル
- `disconnect() -> Result<()>` - Claude CLIから切断

## メッセージタイプ

- `UserMessage` - ユーザー入力メッセージ
- `AssistantMessage` - Claudeのレスポンス
- `SystemMessage` - システム通知
- `ResultMessage` - タイミングとコスト情報を含む操作結果

## エラーハンドリング

SDKは包括的なエラー型を提供：

- `CLINotFoundError` - Claude Code CLIがインストールされていない
- `CLIConnectionError` - 接続エラー
- `ProcessError` - CLIプロセスエラー
- `InvalidState` - 無効な操作状態

## 例

使用例については `examples/` ディレクトリを参照：

- `interactive_demo.rs` - インタラクティブ会話デモ
- `query_simple.rs` - シンプルクエリ例
- `file_operations.rs` - ファイル操作例

## ライセンス

このプロジェクトはMITライセンスの下でライセンスされています - 詳細は [LICENSE](LICENSE) ファイルを参照してください。

## 貢献

貢献を歓迎します！お気軽にPull Requestを提出してください。

## サポート

- [問題を報告](https://github.com/this-rs/nexus/issues)
- [ディスカッション](https://github.com/this-rs/nexus/discussions)

---

Nexus チームが Rust で作成
