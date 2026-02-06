# Nexus - Claude Code SDK & API

[![版本](https://img.shields.io/badge/版本-0.5.0-blue.svg)](https://github.com/this-rs/nexus)
[![许可证](https://img.shields.io/badge/许可证-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

中文文档 | [日本語](README_JA.md) | [English](README.md)

---

## nexus-claude v0.5.0 - 带持久记忆的 Rust SDK

[![Crates.io](https://img.shields.io/crates/v/nexus-claude.svg)](https://crates.io/crates/nexus-claude)
[![Documentation](https://docs.rs/nexus-claude/badge.svg)](https://docs.rs/nexus-claude)

**[nexus-claude](./claude-code-sdk-rs)** 是 Claude Code CLI 的 Rust SDK，具有**持久记忆**和**自动上下文检索**功能：

- **持久记忆系统** - 对话被存储和索引以供将来检索
- **多因素相关性评分** - 根据语义相似性、工作目录、文件重叠和时间衰减对上下文评分
- **自动上下文注入** - 相关的历史上下文自动注入到提示中
- **CLI 自动下载** - 找不到 Claude Code CLI 时自动下载
- **文件检查点** - 将文件更改回滚到任意会话节点
- **结构化输出** - 响应的 JSON Schema 验证
- **完整控制协议** - 权限、钩子、MCP 服务器

> **Fork 声明**：本项目是 [ZhangHanDong/claude-code-api-rs](https://github.com/ZhangHanDong/claude-code-api-rs)（`cc-sdk`）的 fork，增加了持久记忆功能。

```rust
use nexus_claude::{query, ClaudeCodeOptions};
use futures::StreamExt;

#[tokio::main]
async fn main() -> nexus_claude::Result<()> {
    let options = ClaudeCodeOptions::builder()
        .model("claude-opus-4-5-20251101")  // 最新 Opus 4.5
        .auto_download_cli(true)             // 自动下载 CLI
        .max_budget_usd(10.0)                // 预算限制
        .build();

    let mut stream = query("你好，Claude！", Some(options)).await?;
    while let Some(msg) = stream.next().await {
        println!("{:?}", msg?);
    }
    Ok(())
}
```

### 带持久记忆

```rust
use nexus_claude::memory::{MemoryIntegrationBuilder, ContextInjector, MemoryConfig};

// 创建用于对话跟踪的记忆管理器
let mut manager = MemoryIntegrationBuilder::new()
    .enabled(true)
    .cwd("/projects/my-app")
    .url("http://localhost:7700")  // Meilisearch URL
    .min_relevance_score(0.3)
    .max_context_items(5)
    .build();

// 记录对话中的消息和工具调用
manager.record_user_message("如何实现 JWT 认证？");
manager.process_tool_call("Read", &serde_json::json!({
    "file_path": "/projects/my-app/src/auth.rs"
}));
manager.record_assistant_message("我已经分析了您的认证模块...");
```

**[完整 SDK 文档](./claude-code-sdk-rs/README_CN.md)** | **[API 文档](https://docs.rs/nexus-claude)**

---

## Claude Code API 服务器

一个高性能的 Rust 实现的 OpenAI 兼容 API 网关，用于 Claude Code CLI。基于强大的 nexus-claude SDK 构建，该项目提供了一个 RESTful API 接口，让您可以使用熟悉的 OpenAI API 格式与 Claude Code 进行交互。

### 特性

- **OpenAI API 兼容** - 可直接替换 OpenAI API
- **高性能** - 使用 Rust、Axum 和 Tokio 构建
- **连接池优化** - 复用 Claude 进程，响应速度提升 5-10 倍
- **会话管理** - 内置会话支持，实现多轮对话
- **多模态支持** - 同时处理图片和文本
- **响应缓存** - 智能缓存系统，减少延迟和成本
- **MCP 支持** - 模型上下文协议集成
- **流式响应** - 实时流式传输支持
- **工具调用** - 支持 OpenAI tools 格式

### 快速开始

**方式一：从 crates.io 安装**

```bash
cargo install claude-code-api
```

然后运行：
```bash
RUST_LOG=info claude-code-api
# 或使用短别名
RUST_LOG=info ccapi
```

**方式二：从源码构建**

```bash
git clone https://github.com/this-rs/nexus.git
cd nexus
cargo build --release
./target/release/claude-code-api
```

API 服务器将默认在 `http://localhost:8080` 启动。

### 快速测试

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-5-20251101",
    "messages": [
      {"role": "user", "content": "你好，Claude！"}
    ]
  }'
```

## 支持的模型

### 最新模型
- **Opus 4.5**（2025年11月）- 最强大的模型
  - 推荐：`"opus"`（最新版别名）
  - 完整名称：`"claude-opus-4-5-20251101"`
  - SWE-bench: 80.9%（业界领先）
- **Sonnet 4.5** - 平衡的性能
  - 推荐：`"sonnet"`（最新版别名）
  - 完整名称：`"claude-sonnet-4-5-20250929"`
- **Sonnet 4** - 成本效益
  - 完整名称：`"claude-sonnet-4-20250514"`

### 上一代模型
- **Claude 3.5 Sonnet**（`claude-3-5-sonnet-20241022`）
- **Claude 3.5 Haiku**（`claude-3-5-haiku-20241022`）- 最快响应

## 核心功能

### 1. OpenAI 兼容的聊天 API

```python
import openai

# 配置客户端使用 Claude Code API
client = openai.OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="not-needed"  # 不需要 API 密钥
)

response = client.chat.completions.create(
    model="opus",  # 或 "sonnet" 获得更快响应
    messages=[
        {"role": "user", "content": "用 Python 写一个 hello world"}
    ]
)

print(response.choices[0].message.content)
```

### 2. 会话管理

跨多个请求保持上下文：

```python
# 第一次请求 - 创建新会话
response = client.chat.completions.create(
    model="sonnet-4",
    messages=[
        {"role": "user", "content": "我叫小明"}
    ]
)
conversation_id = response.conversation_id

# 后续请求 - 继续会话
response = client.chat.completions.create(
    model="sonnet-4",
    conversation_id=conversation_id,
    messages=[
        {"role": "user", "content": "我叫什么名字？"}
    ]
)
# Claude 会记住："你叫小明"
```

### 3. 流式响应

```python
stream = client.chat.completions.create(
    model="claude-opus-4-20250514",
    messages=[{"role": "user", "content": "写一个长故事"}],
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

## 配置

### 环境变量

```bash
# 服务器配置
CLAUDE_CODE__SERVER__HOST=0.0.0.0
CLAUDE_CODE__SERVER__PORT=8080

# Claude CLI 配置
CLAUDE_CODE__CLAUDE__COMMAND=claude
CLAUDE_CODE__CLAUDE__TIMEOUT_SECONDS=300
CLAUDE_CODE__CLAUDE__MAX_CONCURRENT_SESSIONS=10

# 缓存配置
CLAUDE_CODE__CACHE__ENABLED=true
CLAUDE_CODE__CACHE__MAX_ENTRIES=1000
CLAUDE_CODE__CACHE__TTL_SECONDS=3600
```

## 直接使用 SDK

如果您想构建自己的集成，可以直接使用 SDK：

```toml
[dependencies]
nexus-claude = "0.5.0"
tokio = { version = "1.0", features = ["full"] }
```

带持久记忆：

```toml
[dependencies]
nexus-claude = { version = "0.5.0", features = ["memory"] }
```

## API 端点

### 聊天补全
- `POST /v1/chat/completions` - 创建聊天补全

### 模型
- `GET /v1/models` - 列出可用模型

### 会话
- `POST /v1/conversations` - 创建新会话
- `GET /v1/conversations` - 列出活跃会话
- `GET /v1/conversations/:id` - 获取会话详情

### 统计
- `GET /stats` - 获取 API 使用统计

### 健康检查
- `GET /health` - 检查服务健康状态

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 许可证

本项目基于 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 致谢

- 原始 SDK：[ZhangHanDong/claude-code-api-rs](https://github.com/ZhangHanDong/claude-code-api-rs)（`cc-sdk`）
- 基于 Anthropic 的 [Claude Code CLI](https://claude.ai/download)
- Web 框架：[Axum](https://github.com/tokio-rs/axum)
- 异步运行时：[Tokio](https://tokio.rs/)

## 支持

- [报告问题](https://github.com/this-rs/nexus/issues)
- [讨论](https://github.com/this-rs/nexus/discussions)

---

由 Nexus 团队用 Rust 制作
