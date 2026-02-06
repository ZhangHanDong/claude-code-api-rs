# Claude Code SDK Rust 功能和参数文档

## 概述

Claude Code SDK Rust 是一个用于与 Claude Code CLI 交互的 Rust SDK，提供简单查询接口和完整的交互式客户端功能，与官方 Python SDK 具有完全的功能对等性。

## 核心功能

### 1. 简单查询接口
通过 `query()` 函数提供一次性查询功能，适用于无需保持会话状态的简单交互。

```rust
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeCodeOptions>
) -> Result<impl Stream<Item = Result<Message>>>
```

### 2. 交互式客户端
通过 `InteractiveClient` 提供有状态的对话功能，支持上下文保持和连续对话。

主要方法：
- `new(options: ClaudeCodeOptions) -> Result<Self>` - 创建新客户端
- `connect() -> Result<()>` - 连接到 Claude CLI
- `send_and_receive(prompt: String) -> Result<Vec<Message>>` - 发送消息并等待完整响应
- `send_message(prompt: String) -> Result<()>` - 发送消息而不等待响应
- `receive_response() -> Result<Vec<Message>>` - 接收消息直到收到结果消息
- `interrupt() -> Result<()>` - 取消正在进行的操作
- `disconnect() -> Result<()>` - 断开与 Claude CLI 的连接

### 3. 优化客户端
`OptimizedClient` 提供高性能操作模式，支持三种客户端模式：

```rust
pub enum ClientMode {
    Single,      // 单次请求模式
    Batch,       // 批处理模式
    Interactive  // 交互式会话模式
}
```

### 4. 流式支持
支持异步消息流处理，可以实时接收和处理来自 Claude 的响应。

### 5. 中断功能
支持在操作过程中随时中断正在进行的任务，提供更好的控制能力。

## 配置参数详解

### ClaudeCodeOptions 结构体

所有配置选项都通过 `ClaudeCodeOptions` 结构体进行管理，支持通过构建器模式进行配置：

```rust
let options = ClaudeCodeOptions::builder()
    .system_prompt("You are a helpful assistant")
    .model("claude-3-5-sonnet-20241022")
    .permission_mode(PermissionMode::AcceptEdits)
    .build();
```

### 参数说明

#### 系统提示词配置
- **`system_prompt: Option<String>`**
  - 在所有消息前添加的系统提示词
  - 用于设定 Claude 的角色和行为准则

- **`append_system_prompt: Option<String>`**
  - 追加的系统提示词
  - 在原有系统提示词后添加额外指令

#### 工具权限控制
- **`allowed_tools: Vec<String>`**
  - 允许使用的工具列表
  - 示例：`["read", "write", "bash"]`
  - 只有在此列表中的工具才能被使用

- **`disallowed_tools: Vec<String>`**
  - 禁止使用的工具列表
  - 优先级高于 allowed_tools
  - 在此列表中的工具将被禁用

#### 权限模式
- **`permission_mode: PermissionMode`**
  - 控制工具执行的权限模式
  - 可选值：
    - `Default` - 默认模式，危险操作需要用户确认
    - `AcceptEdits` - 自动接受文件编辑操作
    - `Plan` - 计划模式，用于规划任务（v0.1.7 新增）
    - `BypassPermissions` - 允许所有工具操作无需确认（谨慎使用）

#### MCP (Model Context Protocol) 配置
- **`mcp_servers: HashMap<String, McpServerConfig>`**
  - MCP 服务器配置映射
  - 支持三种类型的 MCP 服务器：
    - `Stdio` - 标准输入/输出模式
    - `Sse` - Server-Sent Events 模式
    - `Http` - HTTP 模式

- **`mcp_tools: Vec<String>`**
  - 要启用的 MCP 工具列表

#### 对话控制
- **`max_turns: Option<i32>`**
  - 最大对话轮数限制
  - 超过此限制后对话将自动结束

- **`max_thinking_tokens: i32`**
  - 最大思考 token 数
  - 默认值：根据模型自动设置
  - 控制 Claude 内部推理的复杂度

- **`model: Option<String>`**
  - 指定使用的模型
  - 示例：`"claude-3-5-sonnet-20241022"`

- **`cwd: Option<PathBuf>`**
  - 工作目录路径
  - Claude 执行文件操作时的基础目录

- **`continue_conversation: bool`**
  - 是否继续之前的对话
  - 设为 true 时会保持对话上下文

- **`resume: Option<String>`**
  - 恢复特定对话 ID
  - 用于恢复之前中断的对话

- **`permission_prompt_tool_name: Option<String>`**
  - 自定义权限提示工具名称

## 消息类型

SDK 支持以下消息类型：

### 1. UserMessage
用户输入的消息
```rust
pub struct UserMessage {
    pub content: String,
}
```

### 2. AssistantMessage
Claude 的响应消息
```rust
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
}
```

### 3. SystemMessage
系统通知消息
```rust
Message::System {
    subtype: String,
    data: serde_json::Value,
}
```

### 4. ResultMessage
操作结果消息，包含时间和成本信息
```rust
Message::Result {
    subtype: String,
    duration_ms: i64,
    duration_api_ms: i64,
    is_error: bool,
    num_turns: i32,
    session_id: String,
    total_cost_usd: Option<f64>,
    usage: Option<serde_json::Value>,
    result: Option<String>,
}
```

## 内容块类型

`ContentBlock` 支持以下类型：

### 1. TextContent
纯文本内容
```rust
pub struct TextContent {
    pub text: String,
}
```

### 2. ToolUseContent
工具使用请求
```rust
pub struct ToolUseContent {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}
```

### 3. ToolResultContent
工具执行结果
```rust
pub struct ToolResultContent {
    pub tool_use_id: String,
    pub content: Option<ContentValue>,
    pub is_error: Option<bool>,
}
```

## 错误处理

SDK 提供全面的错误类型：

- `CLINotFoundError` - Claude Code CLI 未安装
- `CLIConnectionError` - 连接失败
- `ProcessError` - CLI 进程错误
- `InvalidState` - 无效的操作状态

## 使用示例

### 基础查询
```rust
use nexus_claude::{query, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut messages = query("解释一下 Rust 的所有权系统", None).await?;
    
    while let Some(msg) = messages.next().await {
        match msg? {
            nexus_claude::Message::Assistant { message } => {
                for content in message.content {
                    if let nexus_claude::ContentBlock::Text(text) = content {
                        println!("{}", text.text);
                    }
                }
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

### 交互式对话
```rust
use nexus_claude::{InteractiveClient, ClaudeCodeOptions, PermissionMode, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .system_prompt("你是一个 Rust 编程专家")
        .permission_mode(PermissionMode::AcceptEdits)
        .allowed_tools(vec!["read".to_string(), "write".to_string()])
        .max_turns(20)
        .build();
        
    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;
    
    // 第一轮对话
    let messages = client.send_and_receive(
        "帮我创建一个简单的 HTTP 服务器".to_string()
    ).await?;
    
    // 继续对话
    let messages = client.send_and_receive(
        "添加日志记录功能".to_string()
    ).await?;
    
    client.disconnect().await?;
    Ok(())
}
```

### 使用优化客户端
```rust
use nexus_claude::{OptimizedClient, ClientMode, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let options = ClaudeCodeOptions::default();
    
    // 单次请求模式
    let mut client = OptimizedClient::new(ClientMode::Single, options.clone())?;
    let response = client.query("What is 2 + 2?").await?;
    
    // 批处理模式
    let mut batch_client = OptimizedClient::new(ClientMode::Batch, options.clone())?;
    let queries = vec![
        "解释异步编程",
        "什么是 Future",
        "如何使用 tokio"
    ];
    
    for query in queries {
        let response = batch_client.query(query).await?;
        println!("Response: {}", response);
    }
    
    Ok(())
}
```

## 性能优化建议

1. **使用适当的客户端模式**
   - 单次查询使用 `query()` 函数或 `OptimizedClient::Single`
   - 批量处理使用 `OptimizedClient::Batch`
   - 交互式对话使用 `InteractiveClient`

2. **合理配置参数**
   - 设置适当的 `max_turns` 避免无限对话
   - 根据需要调整 `max_thinking_tokens`
   - 只启用必要的工具以提高安全性和性能

3. **错误处理**
   - 始终处理可能的错误情况
   - 使用 `interrupt()` 方法及时取消长时间运行的操作

4. **资源管理**
   - 使用完毕后调用 `disconnect()` 释放资源
   - 避免创建过多的客户端实例

## 与 Python SDK 的功能对等性

本 Rust SDK 提供与官方 Python SDK 100% 的功能对等性：

- ✅ 所有客户端方法：`query()`、`send_message()`、`receive_response()`、`interrupt()`
- ✅ 交互式会话：完整的有状态对话支持
- ✅ 消息流：实时异步消息处理
- ✅ 所有配置选项：系统提示、模型、权限、工具等
- ✅ 所有消息类型：用户、助手、系统、结果消息
- ✅ 错误处理：与 Python SDK 匹配的全面错误类型
- ✅ 会话管理：多会话支持与上下文隔离

## 更多信息

- GitHub 仓库：[claude-code-sdk-rs](https://github.com/your-repo/claude-code-sdk-rs)
- API 文档：[docs.rs/cc-sdk](https://docs.rs/cc-sdk)
- 示例代码：查看 `examples/` 目录

## 许可证

本项目采用 MIT 许可证。详见 [LICENSE](../LICENSE) 文件。