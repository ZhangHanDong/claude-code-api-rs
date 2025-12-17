# Claude Code API

[![バージョン](https://img.shields.io/badge/バージョン-0.2.0-blue.svg)](https://github.com/ZhangHanDong/claude-code-api-rs)
[![ライセンス](https://img.shields.io/badge/ライセンス-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

[中文文档](README_CN.md) | 日本語 | [English](README.md)

---

## 🦀 cc-sdk v0.4.0 - Claude Code Rust SDK

> **🎉 Python claude-agent-sdk v0.1.14 と 100% 機能同等を実現！**

[![Crates.io](https://img.shields.io/crates/v/cc-sdk.svg)](https://crates.io/crates/cc-sdk)
[![Documentation](https://docs.rs/cc-sdk/badge.svg)](https://docs.rs/cc-sdk)

**[cc-sdk](./claude-code-sdk-rs)** は Claude Code CLI の公式 Rust SDK です：

- 📥 **CLI 自動ダウンロード** - Claude Code CLI が見つからない場合に自動ダウンロード
- 📁 **ファイルチェックポイント** - 会話の任意の時点にファイル変更を巻き戻し
- 📊 **構造化出力** - レスポンスの JSON スキーマ検証
- 🔧 **完全なコントロールプロトコル** - パーミッション、フック、MCP サーバー
- 💰 **予算管理** - `max_budget_usd` と `fallback_model` サポート
- 🏖️ **サンドボックス** - ファイルシステム/ネットワークの Bash 隔離

```rust
use cc_sdk::{query, ClaudeCodeOptions};
use futures::StreamExt;

#[tokio::main]
async fn main() -> cc_sdk::Result<()> {
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

👉 **[完全な SDK ドキュメント](./claude-code-sdk-rs/README_JA.md)** | **[API ドキュメント](https://docs.rs/cc-sdk)**

---

Claude Code CLI 用の高性能な Rust 実装による OpenAI 互換 API ゲートウェイです。堅牢な [cc-sdk](https://github.com/ZhangHanDong/claude-code-api-rs/tree/main/claude-code-sdk-rs) をベースに構築されており、使い慣れた OpenAI API 形式で Claude Code と対話できる RESTful API インターフェースを提供します。

## 🎉 Claude Code API を使用しているプロジェクト

- **[url-preview v0.6.0](https://github.com/ZhangHanDong/url-preview/releases/tag/0.6.0)** - LLM を使用して Web ページから構造化データを抽出する Rust ライブラリ。claude-code-api を活用して、OpenAI サポートと並んで Claude による Web コンテンツ抽出を提供します。

## ✨ 機能

- **🔌 OpenAI API 互換** - OpenAI API のドロップイン置換、既存の OpenAI クライアントライブラリで動作
- **🚀 高性能** - Rust、Axum、Tokio で構築された卓越したパフォーマンス
- **📦 cc-sdk ベース** - Claude Code CLI との完全な統合を持つ堅牢な SDK で構築
- **⚡ 接続プーリング** - 最適化された接続プーリングで Claude プロセスを再利用、5-10 倍高速なレスポンス
- **💬 会話管理** - マルチターン会話のための組み込みセッションサポート
- **🖼️ マルチモーダルサポート** - リクエストで画像とテキストを同時に処理
- **⚡ レスポンスキャッシング** - レイテンシとコストを削減するインテリジェントキャッシングシステム
- **🔧 MCP サポート** - 外部ツールやサービスにアクセスするための Model Context Protocol 統合
- **📁 ファイルアクセス制御** - 安全な操作のための設定可能なファイルシステム権限
- **🌊 ストリーミングレスポンス** - 長文コンテンツのリアルタイムストリーミングサポート
- **🛡️ 堅牢なエラーハンドリング** - 自動リトライを含む包括的なエラーハンドリング
- **📊 統計 API** - 使用状況とパフォーマンスメトリクスの監視
- **🔄 複数のクライアントモード** - OneShot、Interactive、Batch 処理モード
- **🔧 ツール呼び出し** - AI ツール統合のための OpenAI tools 形式サポート

## 🚀 クイックスタート

### 前提条件

- Rust 1.75 以上
- [Claude CLI](https://claude.ai/download) がインストールおよび設定済み
- （オプション）拡張機能用の MCP サーバー

### インストール

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
git clone https://github.com/ZhangHanDong/claude-code-api-rs.git
cd claude-code-api-rs
```

ワークスペース全体（API サーバー + SDK）をビルド：
```bash
cargo build --release
```

サーバーを起動：
```bash
./target/release/claude-code-api
```

**注意**: API サーバーは Claude Code CLI とのすべてのやり取りに `cc-sdk` を自動的に含み、使用します。

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

## 🤖 サポートモデル（2025年12月）

API は最新の Claude モデルをサポートしています：

### 最新モデル
- **Opus 4.5** ⭐ NEW（2025年11月）- 最も高性能なモデル
  - 推奨: `"opus"`（最新版のエイリアス）
  - フルネーム: `"claude-opus-4-5-20251101"`
  - SWE-bench: 80.9%（業界トップ）
- **Sonnet 4.5** - バランスの取れたパフォーマンス
  - 推奨: `"sonnet"`（最新版のエイリアス）
  - フルネーム: `"claude-sonnet-4-5-20250929"`
- **Sonnet 4** - コスト効率
  - フルネーム: `"claude-sonnet-4-20250514"`

### 前世代
- **Claude 3.5 Sonnet** (`claude-3-5-sonnet-20241022`)
- **Claude 3.5 Haiku** (`claude-3-5-haiku-20241022`) - 最速レスポンス

## 📖 コア機能

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
    model="sonnet",
    messages=[
        {"role": "user", "content": "私の名前はアリスです"}
    ]
)
conversation_id = response.conversation_id

# 次のリクエスト - 会話を続ける
response = client.chat.completions.create(
    model="sonnet",
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

## 🔧 設定

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

## 📚 API エンドポイント

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

## 🤝 コントリビューション

コントリビューションは歓迎します！お気軽に Pull Request を提出してください。

## 📄 ライセンス

このプロジェクトは MIT ライセンスの下でライセンスされています - 詳細は [LICENSE](LICENSE) ファイルを参照してください。

## 🙏 謝辞

- [cc-sdk](https://github.com/ZhangHanDong/claude-code-api-rs/tree/main/claude-code-sdk-rs) をベースに構築 - Claude Code CLI のための堅牢な Rust SDK
- Anthropic の [Claude Code CLI](https://claude.ai/download) で動作
- 最大限の互換性のために OpenAI の API 設計にインスパイア

---

Made with ❤️ by the Claude Code API team
