# Claude Code API

[![版本](https://img.shields.io/badge/版本-0.1.5-blue.svg)](https://github.com/ZhangHanDong/claude-code-api-rs)
[![许可证](https://img.shields.io/badge/许可证-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

中文文档 | [English](README.md)

一个高性能的 Rust 实现的 OpenAI 兼容 API 网关，用于 Claude Code CLI。基于强大的 [claude-code-sdk-rs](https://github.com/ZhangHanDong/claude-code-api-rs/tree/main/claude-code-sdk-rs) 构建，该项目提供了一个 RESTful API 接口，让您可以使用熟悉的 OpenAI API 格式与 Claude Code 进行交互。

## ✨ 特性

- **🔌 OpenAI API 兼容** - 可直接替换 OpenAI API，兼容现有的 OpenAI 客户端库
- **🚀 高性能** - 使用 Rust、Axum 和 Tokio 构建，性能卓越
- **📦 基于 claude-code-sdk-rs** - 使用强大的 SDK 实现与 Claude Code CLI 的完整集成
- **⚡ 连接池优化** - 通过优化的连接池复用 Claude 进程，响应速度提升 5-10 倍
- **💬 会话管理** - 内置会话支持，实现多轮对话
- **🖼️ 多模态支持** - 在请求中同时处理图片和文本
- **⚡ 响应缓存** - 智能缓存系统，减少延迟和成本
- **🔧 MCP 支持** - 模型上下文协议集成，可访问外部工具和服务
- **📁 文件访问控制** - 可配置的文件系统权限，确保安全操作
- **🌊 流式响应** - 支持长文本的实时流式传输
- **🛡️ 健壮的错误处理** - 全面的错误处理和自动重试机制
- **📊 统计 API** - 监控使用情况和性能指标
- **🔄 多种客户端模式** - OneShot（单次查询）、Interactive（交互式）和 Batch（批处理）模式

## 🚀 快速开始

### 前置要求

- Rust 1.75 或更高版本
- 已安装并配置 [Claude CLI](https://claude.ai/download)
- （可选）用于扩展功能的 MCP 服务器

### 安装

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
git clone https://github.com/ZhangHanDong/claude-code-api-rs.git
cd claude-code-api-rs
```

构建整个工作区（API 服务器 + SDK）：
```bash
cargo build --release
```

启动服务器：
```bash
./target/release/claude-code-api
```

**注意**：API 服务器自动包含并使用 `claude-code-sdk-rs` 来处理所有与 Claude Code CLI 的交互。

API 服务器将默认在 `http://localhost:8080` 启动。

### 快速测试

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [
      {"role": "user", "content": "你好，Claude！"}
    ]
  }'
```

## 📖 核心功能

### 1. OpenAI 兼容的聊天 API

```python
import openai

# 配置客户端使用 Claude Code API
client = openai.OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="not-needed"  # 不需要 API 密钥
)

response = client.chat.completions.create(
    model="claude-opus-4-20250514",
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
    model="claude-opus-4-20250514",
    messages=[
        {"role": "user", "content": "我叫小明"}
    ]
)
conversation_id = response.conversation_id

# 后续请求 - 继续会话
response = client.chat.completions.create(
    model="claude-opus-4-20250514",
    conversation_id=conversation_id,
    messages=[
        {"role": "user", "content": "我叫什么名字？"}
    ]
)
# Claude 会记住："你叫小明"
```

### 3. 多模态支持

同时处理图片和文本：

```python
response = client.chat.completions.create(
    model="claude-opus-4-20250514",
    messages=[{
        "role": "user",
        "content": [
            {"type": "text", "text": "这张图片里有什么？"},
            {"type": "image_url", "image_url": {"url": "/path/to/image.png"}}
        ]
    }]
)
```

支持的图片格式：
- 本地文件路径
- HTTP/HTTPS URL
- Base64 编码的 data URL

### 4. 流式响应

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

### 5. MCP（模型上下文协议）

让 Claude 能够访问外部工具和服务：

```bash
# 创建 MCP 配置
cat > mcp_config.json << EOF
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/allowed/path"]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "your-token"
      }
    }
  }
}
EOF

# 启动时启用 MCP 支持
export CLAUDE_CODE__MCP__ENABLED=true
export CLAUDE_CODE__MCP__CONFIG_FILE="./mcp_config.json"
./target/release/claude-code-api
```

## 🔧 配置

### 环境变量

```bash
# 服务器配置
CLAUDE_CODE__SERVER__HOST=0.0.0.0
CLAUDE_CODE__SERVER__PORT=8080

# Claude CLI 配置
CLAUDE_CODE__CLAUDE__COMMAND=claude
CLAUDE_CODE__CLAUDE__TIMEOUT_SECONDS=300
CLAUDE_CODE__CLAUDE__MAX_CONCURRENT_SESSIONS=10
CLAUDE_CODE__CLAUDE__USE_INTERACTIVE_SESSIONS=true

# 文件访问权限
CLAUDE_CODE__FILE_ACCESS__SKIP_PERMISSIONS=false
CLAUDE_CODE__FILE_ACCESS__ADDITIONAL_DIRS='["/path1", "/path2"]'

# MCP 配置
CLAUDE_CODE__MCP__ENABLED=true
CLAUDE_CODE__MCP__CONFIG_FILE="./mcp_config.json"
CLAUDE_CODE__MCP__STRICT=false
CLAUDE_CODE__MCP__DEBUG=false

# 缓存配置
CLAUDE_CODE__CACHE__ENABLED=true
CLAUDE_CODE__CACHE__MAX_ENTRIES=1000
CLAUDE_CODE__CACHE__TTL_SECONDS=3600

# 会话管理
CLAUDE_CODE__CONVERSATION__MAX_HISTORY_MESSAGES=20
CLAUDE_CODE__CONVERSATION__SESSION_TIMEOUT_MINUTES=30
```

### 配置文件

创建 `config/local.toml`：

```toml
[server]
host = "0.0.0.0"
port = 8080

[claude]
command = "claude"
timeout_seconds = 300
max_concurrent_sessions = 10
use_interactive_sessions = false  # 默认禁用，因为存在稳定性问题

[file_access]
skip_permissions = false
additional_dirs = ["/Users/me/projects", "/tmp"]

[mcp]
enabled = true
config_file = "./mcp_config.json"
strict = false
debug = false
```

## 📦 基于 claude-code-sdk-rs 构建

本 API 服务器基于 [claude-code-sdk-rs](https://github.com/ZhangHanDong/claude-code-api-rs/tree/main/claude-code-sdk-rs) 构建，这是一个功能强大的 Claude Code CLI Rust SDK，提供：

- **与官方 Python SDK 完全兼容** - 100% 功能对等
- **多种客户端类型**：
  - `query()` - 简单的一次性查询
  - `InteractiveClient` - 有状态的会话，保持上下文
  - `OptimizedClient` - 带连接池和性能优化的高级客户端
- **流式支持** - 实时消息流
- **完整的类型安全** - 使用 serde 提供强类型支持
- **异步/等待** - 基于 Tokio 的高性能异步操作

### 直接使用 SDK

如果您想构建自己的集成，可以直接使用 SDK：

```toml
[dependencies]
cc-sdk = "0.1.5"
tokio = { version = "1.0", features = ["full"] }
```

```rust
use cc_sdk::{query, ClaudeCodeOptions, PermissionMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 简单查询
    let response = query("解释量子计算").await?;
    println!("{}", response);

    // 使用选项
    let options = ClaudeCodeOptions::builder()
        .model("claude-3.5-sonnet")
        .permission_mode(PermissionMode::AcceptAll)
        .build();
    
    let response = cc_sdk::query_with_options("写一首俳句", options).await?;
    println!("{}", response);
    
    Ok(())
}
```

## 📚 API 端点

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

## 🛠️ 高级用法

### 与 LangChain 一起使用

```python
from langchain.chat_models import ChatOpenAI

llm = ChatOpenAI(
    base_url="http://localhost:8080/v1",
    api_key="not-needed",
    model="claude-opus-4-20250514"
)

response = llm.invoke("解释量子计算")
print(response.content)
```

### 与 Node.js 一起使用

```javascript
const OpenAI = require('openai');

const client = new OpenAI({
  baseURL: 'http://localhost:8080/v1',
  apiKey: 'not-needed'
});

async function chat() {
  const response = await client.chat.completions.create({
    model: 'claude-opus-4-20250514',
    messages: [{ role: 'user', content: '你好！' }]
  });

  console.log(response.choices[0].message.content);
}
```

### 使用 curl

```bash
# 基本请求
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{"role": "user", "content": "你好"}]
  }'

# 带会话 ID
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "conversation_id": "uuid-here",
    "messages": [{"role": "user", "content": "继续我们的对话"}]
  }'

# 带图片
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": [
        {"type": "text", "text": "这是什么？"},
        {"type": "image_url", "image_url": {"url": "/path/to/image.png"}}
      ]
    }]
  }'
```

## 🔒 安全性

- 通过可配置的权限控制文件访问
- MCP 服务器在隔离的进程中运行
- 无需 API 密钥（依赖 Claude CLI 认证）
- 支持 CORS，适用于 Web 应用
- 请求 ID 跟踪，便于审计

## ⚡ 性能优化

### 交互式会话

API 支持交互式会话管理，可显著提升性能：

- **首次请求**：5-15 秒（Claude 进程启动）
- **后续请求**：< 0.1 秒（带缓存）

由于稳定性问题，交互式会话当前默认禁用：
```toml
[claude]
use_interactive_sessions = false  # 默认值
```

**注意**：交互式会话模式存在已知的并发问题，不建议在生产环境使用。

### 最佳实践

1. **使用会话 ID** 为相关请求复用会话
2. **启用响应缓存** 为频繁重复的查询
3. **配置适当的超时** 基于您的使用场景
4. **监控活跃会话** 通过 `/stats` 端点

详细信息请参见 [交互式会话指南](doc/INTERACTIVE_SESSION_GUIDE.md)。

## 🐛 故障排除

### 常见问题

1. **"权限被拒绝"错误**
   ```bash
   # 启用文件权限
   export CLAUDE_CODE__FILE_ACCESS__SKIP_PERMISSIONS=true
   # 或使用启动脚本
   ./start_with_permissions.sh
   ```

2. **MCP 服务器不工作**
   ```bash
   # 启用调试模式
   export CLAUDE_CODE__MCP__DEBUG=true
   # 检查 MCP 服务器安装
   npx -y @modelcontextprotocol/server-filesystem --version
   ```

3. **首次请求延迟高**
   - 这是正常的，因为 Claude CLI 需要启动
   - 后续请求会因为进程复用而更快

## 🤝 贡献

欢迎贡献！请随时提交 Pull Request。

1. Fork 仓库
2. 创建功能分支（`git checkout -b feature/amazing-feature`）
3. 提交更改（`git commit -m 'Add some amazing feature'`）
4. 推送到分支（`git push origin feature/amazing-feature`）
5. 开启 Pull Request

## 📄 许可证

本项目基于 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- 基于 [Claude Code CLI](https://claude.ai/download) 构建
- 受 OpenAI API 设计启发
- 由 [Axum](https://github.com/tokio-rs/axum) Web 框架驱动

## 📞 支持

- [报告问题](https://github.com/yourusername/claude-code-api/issues)
- [文档](https://github.com/yourusername/claude-code-api/wiki)
- [讨论](https://github.com/yourusername/claude-code-api/discussions)

---

由 Claude Code API 团队用 ❤️ 制作
