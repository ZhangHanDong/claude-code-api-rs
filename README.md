# Claude Code API

[![Version](https://img.shields.io/badge/version-0.2.0-blue.svg)](https://github.com/ZhangHanDong/claude-code-api-rs)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

[中文文档](README_CN.md) | [日本語](README_JA.md) | English

---

## 🦀 cc-sdk v0.6.0 - Rust SDK for Claude Code

> **🎉 100% Feature Parity with Python claude-agent-sdk v0.1.14!** | **🌐 WebSocket Transport (v0.6.0)**

[![Crates.io](https://img.shields.io/crates/v/cc-sdk.svg)](https://crates.io/crates/cc-sdk)
[![Documentation](https://docs.rs/cc-sdk/badge.svg)](https://docs.rs/cc-sdk)

**[cc-sdk](./claude-code-sdk-rs)** is the official Rust SDK for Claude Code CLI, providing:

- 📥 **Auto CLI Download** - Automatically downloads Claude Code CLI if not found
- 📁 **File Checkpointing** - Rewind file changes to any conversation point
- 📊 **Structured Output** - JSON schema validation for responses
- 🔧 **Full Control Protocol** - Permissions, hooks, MCP servers
- 💰 **Budget Control** - `max_budget_usd` and `fallback_model` support
- 🏖️ **Sandbox** - Bash isolation for filesystem/network

```rust
use cc_sdk::{query, ClaudeCodeOptions};
use futures::StreamExt;

#[tokio::main]
async fn main() -> cc_sdk::Result<()> {
    let options = ClaudeCodeOptions::builder()
        .model("claude-opus-4-5-20251101")  // Latest Opus 4.5
        .auto_download_cli(true)             // Auto-download CLI
        .max_budget_usd(10.0)                // Budget limit
        .build();

    let mut stream = query("Hello, Claude!", Some(options)).await?;
    while let Some(msg) = stream.next().await {
        println!("{:?}", msg?);
    }
    Ok(())
}
```

👉 **[Full SDK Documentation](./claude-code-sdk-rs/README.md)** | **[API Docs](https://docs.rs/cc-sdk)**

---

A high-performance Rust implementation of an OpenAI-compatible API gateway for Claude Code CLI. Built on top of the robust [cc-sdk](https://github.com/ZhangHanDong/claude-code-api-rs/tree/main/claude-code-sdk-rs), this project provides a RESTful API interface that allows you to interact with Claude Code using the familiar OpenAI API format.

## 🎉 Who's Using Claude Code API

- **[url-preview v0.6.0](https://github.com/ZhangHanDong/url-preview/releases/tag/0.6.0)** - A Rust library for extracting structured data from web pages using LLMs. It leverages claude-code-api to provide Claude-powered web content extraction alongside OpenAI support.

## ✨ Features

- **🔌 OpenAI API Compatibility** - Drop-in replacement for OpenAI API, works with existing OpenAI client libraries
- **🚀 High Performance** - Built with Rust, Axum, and Tokio for exceptional performance
- **📦 Powered by claude-code-sdk-rs** - Built on a robust SDK with full Claude Code CLI integration
- **⚡ Connection Pooling** - Reuse Claude processes with optimized connection pooling for 5-10x faster responses
- **💬 Conversation Management** - Built-in session support for multi-turn conversations
- **🖼️ Multimodal Support** - Process images alongside text in your requests
- **⚡ Response Caching** - Intelligent caching system to reduce latency and costs
- **🔧 MCP Support** - Model Context Protocol integration for accessing external tools and services
- **📁 File Access Control** - Configurable file system permissions for secure operations
- **🌊 Streaming Responses** - Real-time streaming support for long-form content
- **🛡️ Robust Error Handling** - Comprehensive error handling with automatic retries
- **📊 Statistics API** - Monitor usage and performance metrics
- **🔄 Multiple Client Modes** - OneShot, Interactive, and Batch processing modes
- **🔧 Tool Calling** - OpenAI tools format support for AI tool integrations
- **🌐 WebSocket Bridge** - Real-time bidirectional communication between CLI and external clients via WebSocket

## 🚀 Quick Start

### Prerequisites

- Rust 1.75 or higher
- [Claude CLI](https://claude.ai/download) installed and configured
- (Optional) MCP servers for extended functionality

### Installation

**Option 1: Install from crates.io**

```bash
cargo install claude-code-api
```

Then run:
```bash
RUST_LOG=info claude-code-api
# or use the short alias
RUST_LOG=info ccapi
```

**Option 2: Build from source**

```bash
git clone https://github.com/ZhangHanDong/claude-code-api-rs.git
cd claude-code-api-rs
```

Build the entire workspace (API server + SDK):
```bash
cargo build --release
```

Start the server:
```bash
./target/release/claude-code-api
```

**Note**: The API server automatically includes and uses `claude-code-sdk-rs` for all Claude Code CLI interactions.

The API server will start on `http://localhost:8080` by default.

### Quick Test

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-5-20251101",
    "messages": [
      {"role": "user", "content": "Hello, Claude!"}
    ]
  }'
```

## 🤖 Supported Models (December 2025)

The API supports the latest Claude models:

### Latest Models
- **Opus 4.5** ⭐ NEW (November 2025) - Most capable model
  - Recommended: `"opus"` (alias for latest)
  - Full name: `"claude-opus-4-5-20251101"`
  - SWE-bench: 80.9% (industry-leading)
- **Sonnet 4.5** - Balanced performance
  - Recommended: `"sonnet"` (alias for latest)
  - Full name: `"claude-sonnet-4-5-20250929"`
- **Sonnet 4** - Cost-effective
  - Full name: `"claude-sonnet-4-20250514"`

### Previous Generation
- **Claude 3.5 Sonnet** (`claude-3-5-sonnet-20241022`)
- **Claude 3.5 Haiku** (`claude-3-5-haiku-20241022`) - Fastest response times

### Model Usage Examples

```bash
# Using Opus 4.1 (recommended: use alias)
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "opus", "messages": [{"role": "user", "content": "Hello"}]}'

# Using Sonnet 4 (recommended: use alias)
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "sonnet", "messages": [{"role": "user", "content": "Hello"}]}'

# Using latest aliases
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "opus", "messages": [{"role": "user", "content": "Hello"}]}'
```

## 📖 Core Features

### 1. OpenAI-Compatible Chat API

```python
import openai

# Configure the client to use Claude Code API
client = openai.OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="not-needed"  # API key is not required
)

response = client.chat.completions.create(
    model="opus",  # or "sonnet" for faster responses
    messages=[
        {"role": "user", "content": "Write a hello world in Python"}
    ]
)

print(response.choices[0].message.content)
```

### 2. Conversation Management

Maintain context across multiple requests:

```python
# First request - creates a new conversation
response = client.chat.completions.create(
    model="sonnet-4",
    messages=[
        {"role": "user", "content": "My name is Alice"}
    ]
)
conversation_id = response.conversation_id

# Subsequent request - continues the conversation
response = client.chat.completions.create(
    model="sonnet-4",
    conversation_id=conversation_id,
    messages=[
        {"role": "user", "content": "What's my name?"}
    ]
)
# Claude will remember: "Your name is Alice"
```

### 3. Multimodal Support

Process images with text:

```python
response = client.chat.completions.create(
    model="claude-opus-4-20250514",
    messages=[{
        "role": "user",
        "content": [
            {"type": "text", "text": "What's in this image?"},
            {"type": "image_url", "image_url": {"url": "/path/to/image.png"}}
        ]
    }]
)
```

Supported image formats:
- Local file paths
- HTTP/HTTPS URLs
- Base64 encoded data URLs

### 4. Streaming Responses

```python
stream = client.chat.completions.create(
    model="claude-opus-4-20250514",
    messages=[{"role": "user", "content": "Write a long story"}],
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

### 5. MCP (Model Context Protocol)

Enable Claude to access external tools and services:

```bash
# Create MCP configuration
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

# Start with MCP support
export CLAUDE_CODE__MCP__ENABLED=true
export CLAUDE_CODE__MCP__CONFIG_FILE="./mcp_config.json"
./target/release/claude-code-api
```

### 6. Tool Calling (OpenAI Compatible)

Use tools for AI integrations:

```python
response = client.chat.completions.create(
    model="claude-3-5-haiku-20241022",
    messages=[
        {"role": "user", "content": "Please preview this URL: https://rust-lang.org"}
    ],
    tools=[
        {
            "type": "function",
            "function": {
                "name": "url_preview",
                "description": "Preview a URL and extract its content",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "description": "The URL to preview"}
                    },
                    "required": ["url"]
                }
            }
        }
    ],
    tool_choice="auto"
)

# Response will include tool_calls:
# {
#   "choices": [{
#     "message": {
#       "role": "assistant",
#       "tool_calls": [{
#         "id": "call_xxx",
#         "type": "function",
#         "function": {
#           "name": "url_preview",
#           "arguments": "{\"url\": \"https://rust-lang.org\"}"
#         }
#       }]
#     }
#   }]
# }
```

This feature enables seamless integration with modern AI tools like [url-preview](https://github.com/ZhangHanDong/url-preview) and other OpenAI-compatible tool chains. url-preview v0.6.0+ uses this exact format to extract structured data from web pages using Claude.

### Agent Tools & Permissions

- Tools control (SDK): set `allowed_tools` / `disallowed_tools` and `permission_mode` in `ClaudeCodeOptions` to whitelist/blacklist and choose approval behavior.
- Runtime approvals (SDK): implement `CanUseTool` to decide `{allow, input?/reason?}` per tool call.
- MCP (server): configure via `config/*.toml` or `mcp_config.json` and use scripts under `script/` (e.g., `start_with_mcp.sh`). The API reuses the SDK’s MCP wiring.
- Programmatic agents (SDK): define `agents` and `setting_sources` to pass structured agent definitions to CLI.

See `claude-code-sdk-rs/README.md` for a full Rust example enabling tools and MCP.

## 🔧 Configuration

### Environment Variables

```bash
# Server configuration
CLAUDE_CODE__SERVER__HOST=0.0.0.0
CLAUDE_CODE__SERVER__PORT=8080

# Claude CLI configuration
CLAUDE_CODE__CLAUDE__COMMAND=claude
CLAUDE_CODE__CLAUDE__TIMEOUT_SECONDS=300
CLAUDE_CODE__CLAUDE__MAX_CONCURRENT_SESSIONS=10
CLAUDE_CODE__CLAUDE__USE_INTERACTIVE_SESSIONS=true

# File access permissions
CLAUDE_CODE__FILE_ACCESS__SKIP_PERMISSIONS=false
CLAUDE_CODE__FILE_ACCESS__ADDITIONAL_DIRS='["/path1", "/path2"]'

# MCP configuration
CLAUDE_CODE__MCP__ENABLED=true
CLAUDE_CODE__MCP__CONFIG_FILE="./mcp_config.json"
CLAUDE_CODE__MCP__STRICT=false
CLAUDE_CODE__MCP__DEBUG=false

# Cache configuration
CLAUDE_CODE__CACHE__ENABLED=true
CLAUDE_CODE__CACHE__MAX_ENTRIES=1000
CLAUDE_CODE__CACHE__TTL_SECONDS=3600

# Conversation management
CLAUDE_CODE__CONVERSATION__MAX_HISTORY_MESSAGES=20
CLAUDE_CODE__CONVERSATION__SESSION_TIMEOUT_MINUTES=30
```

### Configuration File

Create `config/local.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080

[claude]
command = "claude"
timeout_seconds = 300
max_concurrent_sessions = 10
use_interactive_sessions = false  # Disabled by default due to stability issues

[file_access]
skip_permissions = false
additional_dirs = ["/Users/me/projects", "/tmp"]

[mcp]
enabled = true
config_file = "./mcp_config.json"
strict = false
debug = false
```

## 📦 Built on claude-code-sdk-rs

This API server is built on top of [claude-code-sdk-rs](https://github.com/ZhangHanDong/claude-code-api-rs/tree/main/claude-code-sdk-rs), a powerful Rust SDK for Claude Code CLI that provides:

- **Full Feature Parity** with the official Python SDK
- **Multiple Client Types**: 
  - `query()` - Simple one-shot queries
  - `InteractiveClient` - Stateful conversations with context
  - `OptimizedClient` - Advanced client with connection pooling and performance features
- **Streaming Support** - Real-time message streaming
- **Complete Type Safety** - Strongly typed with serde support
- **Async/Await** - Built on Tokio for high-performance async operations

### Using the SDK Directly

If you prefer to build your own integration, you can use the SDK directly:

```toml
[dependencies]
cc-sdk = "0.1.5"
tokio = { version = "1.0", features = ["full"] }
```

```rust
use cc_sdk::{query, ClaudeCodeOptions, PermissionMode};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple query (streaming messages)
    let mut messages = query("Explain quantum computing", None).await?;
    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }

    // With options
    let options = ClaudeCodeOptions::builder()
        .model("claude-3.5-sonnet")
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    let mut messages = query("Write a haiku", Some(options)).await?;
    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }

    Ok(())
}
```

## 📚 API Endpoints

### Chat Completions
- `POST /v1/chat/completions` - Create a chat completion

### Models
- `GET /v1/models` - List available models

### Conversations
- `POST /v1/conversations` - Create a new conversation
- `GET /v1/conversations` - List active conversations
- `GET /v1/conversations/:id` - Get conversation details

### WebSocket Sessions
- `POST /v1/sessions` - Create a new WebSocket session (spawns CLI with `--sdk-url`)
- `GET /v1/sessions` - List all active WebSocket sessions
- `GET /v1/sessions/{id}` - Get session details
- `DELETE /v1/sessions/{id}` - Delete a session (kills CLI process)
- `WS /ws/cli/{session_id}` - WebSocket endpoint for CLI process connection
- `WS /ws/session/{session_id}` - WebSocket endpoint for external client connection

### Statistics
- `GET /stats` - Get API usage statistics

### Health Check
- `GET /health` - Check service health

## 🛠️ Advanced Usage

### Using with LangChain

```python
from langchain.chat_models import ChatOpenAI

llm = ChatOpenAI(
    base_url="http://localhost:8080/v1",
    api_key="not-needed",
    model="claude-opus-4-20250514"
)

response = llm.invoke("Explain quantum computing")
print(response.content)
```

### Using with Node.js

```javascript
const OpenAI = require('openai');

const client = new OpenAI({
  baseURL: 'http://localhost:8080/v1',
  apiKey: 'not-needed'
});

async function chat() {
  const response = await client.chat.completions.create({
    model: 'claude-opus-4-20250514',
    messages: [{ role: 'user', content: 'Hello!' }]
  });

  console.log(response.choices[0].message.content);
}
```

### Using with curl

```bash
# Basic request
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{"role": "user", "content": "Hello"}]
  }'

# With conversation ID
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "conversation_id": "uuid-here",
    "messages": [{"role": "user", "content": "Continue our chat"}]
  }'

# With image
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": [
        {"type": "text", "text": "What is this?"},
        {"type": "image_url", "image_url": {"url": "/path/to/image.png"}}
      ]
    }]
  }'
```

## 🔒 Security

- File access is controlled through configurable permissions
- MCP servers run in isolated processes
- No API key required (relies on Claude CLI authentication)
- Supports CORS for web applications
- Request ID tracking for audit trails

## ⚡ Performance Optimization

### Advanced Performance Features (via claude-code-sdk-rs)

The API leverages advanced performance optimizations from the underlying SDK:

#### Connection Pooling
- **First request**: 2-5 seconds (with pre-warmed connection pool)
- **Subsequent requests**: < 0.1 seconds (reusing existing connections)
- **Concurrent handling**: Multiple requests can share the connection pool

#### Client Modes
1. **OneShot Mode**: Simple, stateless queries (default)
2. **Interactive Mode**: Maintains conversation context across requests
3. **Batch Mode**: Process multiple queries concurrently for high throughput

#### Performance Metrics
```bash
# Example performance improvements with OptimizedClient:
- Sequential queries: ~5s for 5 queries
- Batch processing: ~1.5s for 5 queries (3x speedup)
- With connection pooling: < 100ms per query after warm-up
```

### Configuration for Performance

```toml
[claude]
max_concurrent_sessions = 10  # Increase for higher throughput
use_interactive_sessions = true  # Enable for conversation context
timeout_seconds = 300  # Adjust based on query complexity

[cache]
enabled = true
max_entries = 1000
ttl_seconds = 3600
```

### Best Practices

1. **Use the optimized REST endpoints** that leverage `OptimizedClient` from the SDK
2. **Enable connection pooling** for frequently used endpoints
3. **Use batch endpoints** for processing multiple queries
4. **Monitor performance** via the `/stats` endpoint
5. **Configure appropriate connection pool size** based on load

For detailed performance tuning, see the [SDK Performance Guide](https://github.com/ZhangHanDong/claude-code-api-rs/tree/main/claude-code-sdk-rs#performance-optimization).

## 🐛 Troubleshooting

### Common Issues

1. **"Permission denied" errors**
   ```bash
   # Enable file permissions
   export CLAUDE_CODE__FILE_ACCESS__SKIP_PERMISSIONS=true
   # Or use the startup script
   ./start_with_permissions.sh
   ```

2. **MCP servers not working**
   ```bash
   # Enable debug mode
   export CLAUDE_CODE__MCP__DEBUG=true
   # Check MCP server installation
   npx -y @modelcontextprotocol/server-filesystem --version
   ```

3. **High latency on first request**
   - This is normal as Claude CLI needs to start up
   - Subsequent requests will be faster due to process reuse

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built on top of [claude-code-sdk-rs](https://github.com/ZhangHanDong/claude-code-api-rs/tree/main/claude-code-sdk-rs) - The robust Rust SDK for Claude Code CLI
- Powered by [Claude Code CLI](https://claude.ai/download) from Anthropic
- Inspired by OpenAI's API design for maximum compatibility
- Web framework: [Axum](https://github.com/tokio-rs/axum) for high-performance HTTP serving
- Async runtime: [Tokio](https://tokio.rs/) for blazing-fast async I/O

## 📞 Support

- [Report Issues](https://github.com/ZhangHanDong/claude-code-api-rs/issues)
- [Documentation](https://github.com/ZhangHanDong/claude-code-api-rs/wiki)
- [Discussions](https://github.com/ZhangHanDong/claude-code-api-rs/discussions)

---

Made with ❤️ by the Claude Code API team
