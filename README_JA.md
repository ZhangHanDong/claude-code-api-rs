# Nexus - Claude Code SDK & API

[![バージョン](https://img.shields.io/badge/バージョン-0.5.0-blue.svg)](https://github.com/this-rs/nexus)
[![ライセンス](https://img.shields.io/badge/ライセンス-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

[中文文档](README_CN.md) | 日本語 | [English](README.md)

---

## nexus-claude v0.5.0 - 永続メモリ付き Rust SDK

[![Crates.io](https://img.shields.io/crates/v/nexus-claude.svg)](https://crates.io/crates/nexus-claude)
[![Documentation](https://docs.rs/nexus-claude/badge.svg)](https://docs.rs/nexus-claude)

**[nexus-claude](./claude-code-sdk-rs)** は Claude Code CLI の Rust SDK で、**永続メモリ**と**自律コンテキスト取得**機能を備えています：

- **永続メモリシステム** - 会話を保存してインデックス化し、将来の取得に使用
- **多因子関連性スコアリング** - セマンティック類似性、作業ディレクトリ、ファイル重複、時間減衰によるコンテキストスコアリング
- **自律コンテキスト注入** - 関連する過去のコンテキストをプロンプトに自動注入
- **CLI 自動ダウンロード** - Claude Code CLI が見つからない場合に自動ダウンロード
- **ファイルチェックポイント** - 会話の任意の時点にファイル変更を巻き戻し
- **構造化出力** - レスポンスの JSON スキーマ検証
- **完全なコントロールプロトコル** - パーミッション、フック、MCP サーバー

> **Fork 通知**：このプロジェクトは [ZhangHanDong/claude-code-api-rs](https://github.com/ZhangHanDong/claude-code-api-rs)（`cc-sdk`）のフォークで、永続メモリ機能を追加しています。

```rust
use nexus_claude::{query, ClaudeCodeOptions};
use futures::StreamExt;

#[tokio::main]
async fn main() -> nexus_claude::Result<()> {
    let options = ClaudeCodeOptions::builder()
        .model("claude-opus-4-5-20251101")  // 最新 Opus 4.5
        .auto_download_cli(true)             // CLI 自動ダウンロード
        .max_budget_usd(10.0)                // 予算制限
        .build();

    let mut stream = query("こんにちは、Claude！", Some(options)).await?;
    while let Some(msg) = stream.next().await {
        println!("{:?}", msg?);
    }
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

**[完全な SDK ドキュメント](./claude-code-sdk-rs/README_JA.md)** | **[API ドキュメント](https://docs.rs/nexus-claude)**

---

## Claude Code API サーバー

Claude Code CLI 用の高性能な Rust 実装による OpenAI 互換 API ゲートウェイです。堅牢な nexus-claude SDK をベースに構築されており、使い慣れた OpenAI API 形式で Claude Code と対話できる RESTful API インターフェースを提供します。

### 機能

- **OpenAI API 互換** - OpenAI API のドロップイン置換
- **高性能** - Rust、Axum、Tokio で構築
- **接続プーリング** - Claude プロセスの再利用で 5-10 倍高速なレスポンス
- **会話管理** - マルチターン会話のための組み込みセッションサポート
- **マルチモーダルサポート** - 画像とテキストを同時に処理
- **レスポンスキャッシング** - レイテンシとコストを削減するインテリジェントキャッシング
- **MCP サポート** - Model Context Protocol 統合
- **ストリーミングレスポンス** - リアルタイムストリーミングサポート
- **ツール呼び出し** - OpenAI tools 形式サポート

### クイックスタート

**オプション 1: crates.io からインストール**

```bash
cargo install claude-code-api
```

実行：
```bash
RUST_LOG=info claude-code-api
# または短いエイリアスを使用
RUST_LOG=info ccapi
```

**オプション 2: ソースからビルド**

```bash
git clone https://github.com/this-rs/nexus.git
cd nexus
cargo build --release
./target/release/claude-code-api
```

API サーバーはデフォルトで `http://localhost:8080` で起動します。

### クイックテスト

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-5-20251101",
    "messages": [
      {"role": "user", "content": "こんにちは、Claude！"}
    ]
  }'
```

## サポートモデル

### 最新モデル
- **Opus 4.5**（2025年11月）- 最も高性能なモデル
  - 推奨: `"opus"`（最新版のエイリアス）
  - フルネーム: `"claude-opus-4-5-20251101"`
  - SWE-bench: 80.9%（業界トップ）
- **Sonnet 4.5** - バランスの取れたパフォーマンス
  - 推奨: `"sonnet"`（最新版のエイリアス）
  - フルネーム: `"claude-sonnet-4-5-20250929"`
- **Sonnet 4** - コスト効率
  - フルネーム: `"claude-sonnet-4-20250514"`

### 前世代
- **Claude 3.5 Sonnet**（`claude-3-5-sonnet-20241022`）
- **Claude 3.5 Haiku**（`claude-3-5-haiku-20241022`）- 最速レスポンス

## コア機能

### 1. OpenAI 互換チャット API

```python
import openai

# Claude Code API を使用するようにクライアントを設定
client = openai.OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="not-needed"  # API キーは不要
)

response = client.chat.completions.create(
    model="opus",  # より高速なレスポンスには "sonnet"
    messages=[
        {"role": "user", "content": "Python で hello world を書いて"}
    ]
)

print(response.choices[0].message.content)
```

### 2. 会話管理

複数のリクエストにわたってコンテキストを維持：

```python
# 最初のリクエスト - 新しい会話を作成
response = client.chat.completions.create(
    model="sonnet-4",
    messages=[
        {"role": "user", "content": "私の名前はアリスです"}
    ]
)
conversation_id = response.conversation_id

# 次のリクエスト - 会話を続ける
response = client.chat.completions.create(
    model="sonnet-4",
    conversation_id=conversation_id,
    messages=[
        {"role": "user", "content": "私の名前は何ですか？"}
    ]
)
# Claude は覚えています: "あなたの名前はアリスです"
```

### 3. ストリーミングレスポンス

```python
stream = client.chat.completions.create(
    model="opus",
    messages=[{"role": "user", "content": "長い物語を書いて"}],
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

## 設定

### 環境変数

```bash
# サーバー設定
CLAUDE_CODE__SERVER__HOST=0.0.0.0
CLAUDE_CODE__SERVER__PORT=8080

# Claude CLI 設定
CLAUDE_CODE__CLAUDE__COMMAND=claude
CLAUDE_CODE__CLAUDE__TIMEOUT_SECONDS=300
CLAUDE_CODE__CLAUDE__MAX_CONCURRENT_SESSIONS=10

# キャッシュ設定
CLAUDE_CODE__CACHE__ENABLED=true
CLAUDE_CODE__CACHE__MAX_ENTRIES=1000
CLAUDE_CODE__CACHE__TTL_SECONDS=3600
```

## SDK を直接使用

独自の統合を構築する場合は、SDK を直接使用できます：

```toml
[dependencies]
nexus-claude = "0.5.0"
tokio = { version = "1.0", features = ["full"] }
```

永続メモリ付き：

```toml
[dependencies]
nexus-claude = { version = "0.5.0", features = ["memory"] }
```

## API エンドポイント

### チャット補完
- `POST /v1/chat/completions` - チャット補完を作成

### モデル
- `GET /v1/models` - 利用可能なモデルを一覧表示

### 会話
- `POST /v1/conversations` - 新しい会話を作成
- `GET /v1/conversations` - アクティブな会話を一覧表示
- `GET /v1/conversations/:id` - 会話の詳細を取得

### 統計
- `GET /stats` - API 使用統計を取得

### ヘルスチェック
- `GET /health` - サービスの健全性をチェック

## コントリビューション

コントリビューションは歓迎します！お気軽に Pull Request を提出してください。

## ライセンス

このプロジェクトは MIT ライセンスの下でライセンスされています - 詳細は [LICENSE](LICENSE) ファイルを参照してください。

## 謝辞

- オリジナル SDK: [ZhangHanDong/claude-code-api-rs](https://github.com/ZhangHanDong/claude-code-api-rs)（`cc-sdk`）
- Anthropic の [Claude Code CLI](https://claude.ai/download) で動作
- Web フレームワーク: [Axum](https://github.com/tokio-rs/axum)
- 非同期ランタイム: [Tokio](https://tokio.rs/)

## サポート

- [問題を報告](https://github.com/this-rs/nexus/issues)
- [ディスカッション](https://github.com/this-rs/nexus/discussions)

---

Nexus チームが Rust で作成
