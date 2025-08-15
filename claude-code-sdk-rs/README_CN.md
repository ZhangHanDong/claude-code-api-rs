# Claude Code SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/cc-sdk.svg)](https://crates.io/crates/cc-sdk)
[![Documentation](https://docs.rs/cc-sdk/badge.svg)](https://docs.rs/cc-sdk)
[![License](https://img.shields.io/crates/l/cc-sdk.svg)](LICENSE)

一个用于与 Claude Code CLI 交互的 Rust SDK，提供简单查询接口和完整的交互式客户端功能。

## 功能特性

- 🚀 **简单查询接口** - 使用 `query()` 函数进行一次性查询
- 💬 **交互式客户端** - 支持有状态的对话，保持上下文
- 🔄 **流式支持** - 实时消息流
- 🛑 **中断功能** - 取消正在进行的操作
- 🔧 **完整配置** - Claude Code 的全面配置选项
- 📦 **类型安全** - 使用 serde 的强类型支持
- ⚡ **异步/等待** - 基于 Tokio 的异步操作

## 完整功能集

此 Rust SDK 提供全面的 Claude Code 交互功能：

- ✅ **客户端方法**：`query()`、`send_message()`、`receive_response()`、`interrupt()`
- ✅ **交互式会话**：完整的有状态对话支持
- ✅ **消息流**：实时异步消息处理
- ✅ **配置选项**：系统提示、模型、权限、工具等
- ✅ **消息类型**：用户、助手、系统、结果消息
- ✅ **错误处理**：全面的错误类型和详细诊断
- ✅ **会话管理**：支持多会话和上下文隔离
- ✅ **类型安全**：充分利用 Rust 的类型系统确保代码可靠性

## 安装

在你的 `Cargo.toml` 中添加：

```toml
[dependencies]
cc-sdk = "0.1.9"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

## 前置要求

安装 Claude Code CLI：

```bash
npm install -g @anthropic-ai/claude-code
```

## 支持的模型（2025年）

SDK 支持 2025 年最新的 Claude 模型：

### 最新模型
- **Opus 4.1** - 最强大的模型
  - 完整名称：`"claude-opus-4-1-20250805"`
  - 别名：`"opus"`（推荐 - 使用最新 Opus）
  
- **Sonnet 4** - 平衡的性能
  - 完整名称：`"claude-sonnet-4-20250514"`
  - 别名：`"sonnet"`（推荐 - 使用最新 Sonnet）

### 上一代模型
- **Claude 3.5 Sonnet** - `"claude-3-5-sonnet-20241022"`
- **Claude 3.5 Haiku** - `"claude-3-5-haiku-20241022"`（最快）

### 在代码中使用模型

```rust
use cc_sdk::{query, ClaudeCodeOptions, Result};

// 使用 Opus 4.1（推荐使用别名）
let options = ClaudeCodeOptions::builder()
    .model("opus")  // 或 "claude-opus-4-1-20250805" 指定版本
    .build();

// 使用 Sonnet 4（推荐使用别名）
let options = ClaudeCodeOptions::builder()
    .model("sonnet")  // 或 "claude-sonnet-4-20250514" 指定版本
    .build();

let mut messages = query("你的提示", Some(options)).await?;
```

## 快速开始

### 简单查询（一次性）

```rust
use cc_sdk::{query, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut messages = query("2 + 2 等于多少？", None).await?;

    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }

    Ok(())
}
```

### 交互式客户端

```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // 发送消息并接收响应
    let messages = client.send_and_receive(
        "帮我写一个 Python 网络服务器".to_string()
    ).await?;

    // 处理响应
    for msg in &messages {
        match msg {
            cc_sdk::Message::Assistant { message } => {
                println!("Claude: {:?}", message);
            }
            _ => {}
        }
    }

    // 发送后续消息
    let messages = client.send_and_receive(
        "让它使用 async/await".to_string()
    ).await?;

    client.disconnect().await?;
    Ok(())
}
```

### 高级用法

```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // 发送消息但不等待响应
    client.send_message("计算圆周率到100位".to_string()).await?;

    // 做其他工作...

    // 准备好时接收响应（在 Result 消息处停止）
    let messages = client.receive_response().await?;

    // 取消长时间运行的操作
    client.send_message("写一篇10000字的文章".to_string()).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    client.interrupt().await?;

    client.disconnect().await?;
    Ok(())
}
```

## 配置选项

```rust
use cc_sdk::{ClaudeCodeOptions, PermissionMode};

let options = ClaudeCodeOptions::builder()
    .system_prompt("你是一个有帮助的编程助手")
    .model("claude-3-5-sonnet-20241022")
    .permission_mode(PermissionMode::AcceptEdits)
    .max_turns(10)
    .max_thinking_tokens(10000)
    .allowed_tools(vec!["read_file".to_string(), "write_file".to_string()])
    .cwd("/path/to/project")
    .build();
```

## API 参考

### `query()`

用于一次性交互的简单无状态查询函数。

```rust
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeCodeOptions>
) -> Result<impl Stream<Item = Result<Message>>>
```

### `InteractiveClient`

用于有状态交互式对话的主要客户端。

#### 方法

- `new(options: ClaudeCodeOptions) -> Result<Self>` - 创建新客户端
- `connect() -> Result<()>` - 连接到 Claude CLI
- `send_and_receive(prompt: String) -> Result<Vec<Message>>` - 发送消息并等待完整响应
- `send_message(prompt: String) -> Result<()>` - 发送消息但不等待
- `receive_response() -> Result<Vec<Message>>` - 接收消息直到 Result 消息
- `interrupt() -> Result<()>` - 取消正在进行的操作
- `disconnect() -> Result<()>` - 断开与 Claude CLI 的连接

## 消息类型

- `UserMessage` - 用户输入消息
- `AssistantMessage` - Claude 的响应
- `SystemMessage` - 系统通知
- `ResultMessage` - 包含时间和成本信息的操作结果

## 错误处理

SDK 提供全面的错误类型：

- `CLINotFoundError` - Claude Code CLI 未安装
- `CLIConnectionError` - 连接失败
- `ProcessError` - CLI 进程错误
- `InvalidState` - 无效的操作状态

## 示例

查看 `examples/` 目录获取更多使用示例：

- `interactive_demo.rs` - 交互式对话演示
- `query_simple.rs` - 简单查询示例
- `file_operations.rs` - 文件操作示例

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 贡献

欢迎贡献！请随时提交 Pull Request。
