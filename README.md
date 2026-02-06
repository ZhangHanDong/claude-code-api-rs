# Nexus - Claude Code SDK & API

[![CI](https://github.com/this-rs/nexus/actions/workflows/ci.yml/badge.svg)](https://github.com/this-rs/nexus/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/this-rs/nexus/branch/main/graph/badge.svg)](https://codecov.io/gh/this-rs/nexus)
[![Version](https://img.shields.io/badge/version-0.5.0-blue.svg)](https://github.com/this-rs/nexus)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88+-orange.svg)](https://www.rust-lang.org)

---

## nexus-claude v0.5.0 - Rust SDK with Persistent Memory

[![Crates.io](https://img.shields.io/crates/v/nexus-claude.svg)](https://crates.io/crates/nexus-claude)
[![Documentation](https://docs.rs/nexus-claude/badge.svg)](https://docs.rs/nexus-claude)

**[nexus-claude](./claude-code-sdk-rs)** is a Rust SDK for Claude Code CLI with **persistent memory** and **autonomous context retrieval**:

- **Persistent Memory System** - Conversations stored and indexed for future retrieval
- **Multi-Factor Relevance Scoring** - Context scored by semantic similarity, working directory, file overlap, and recency
- **Autonomous Context Injection** - Relevant historical context automatically injected into prompts
- **Auto CLI Download** - Automatically downloads Claude Code CLI if not found
- **File Checkpointing** - Rewind file changes to any conversation point
- **Structured Output** - JSON schema validation for responses
- **Full Control Protocol** - Permissions, hooks, MCP servers

> **Fork Notice**: This project is a fork of [ZhangHanDong/claude-code-api-rs](https://github.com/ZhangHanDong/claude-code-api-rs) (`cc-sdk`), extended with persistent memory capabilities.

```rust
use nexus_claude::{query, ClaudeCodeOptions};
use futures::StreamExt;

#[tokio::main]
async fn main() -> nexus_claude::Result<()> {
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

### With Persistent Memory

```rust
use nexus_claude::memory::{MemoryIntegrationBuilder, ContextInjector, MemoryConfig};

// Create a memory manager for conversation tracking
let mut manager = MemoryIntegrationBuilder::new()
    .enabled(true)
    .cwd("/projects/my-app")
    .url("http://localhost:7700")  // Meilisearch URL
    .min_relevance_score(0.3)
    .max_context_items(5)
    .build();

// Record messages and tool calls during conversation
manager.record_user_message("How do I implement JWT auth?");
manager.process_tool_call("Read", &serde_json::json!({
    "file_path": "/projects/my-app/src/auth.rs"
}));
manager.record_assistant_message("I've analyzed your auth module...");
```

**[Full SDK Documentation](./claude-code-sdk-rs/README.md)** | **[API Docs](https://docs.rs/nexus-claude)**

---

## Claude Code API Server

A high-performance Rust implementation of an OpenAI-compatible API gateway for Claude Code CLI. Built on top of the robust nexus-claude SDK, this project provides a RESTful API interface that allows you to interact with Claude Code using the familiar OpenAI API format.

### Features

- **OpenAI API Compatibility** - Drop-in replacement for OpenAI API
- **High Performance** - Built with Rust, Axum, and Tokio
- **Connection Pooling** - Reuse Claude processes for 5-10x faster responses
- **Conversation Management** - Built-in session support for multi-turn conversations
- **Multimodal Support** - Process images alongside text
- **Response Caching** - Intelligent caching to reduce latency and costs
- **MCP Support** - Model Context Protocol integration
- **Streaming Responses** - Real-time streaming support
- **Tool Calling** - OpenAI tools format support

### Quick Start

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
git clone https://github.com/this-rs/nexus.git
cd nexus
cargo build --release
./target/release/claude-code-api
```

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

## Supported Models

### Latest Models
- **Opus 4.5** (November 2025) - Most capable model
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

## Core Features

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
```

## Configuration

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
use_interactive_sessions = false

[file_access]
skip_permissions = false
additional_dirs = ["/Users/me/projects", "/tmp"]

[mcp]
enabled = true
config_file = "./mcp_config.json"
strict = false
debug = false
```

## Using the SDK Directly

If you prefer to build your own integration, you can use the SDK directly:

```toml
[dependencies]
nexus-claude = "0.5.0"
tokio = { version = "1.0", features = ["full"] }
```

With persistent memory:

```toml
[dependencies]
nexus-claude = { version = "0.5.0", features = ["memory"] }
```

```rust
use nexus_claude::{query, ClaudeCodeOptions, PermissionMode};
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

## API Endpoints

### Chat Completions
- `POST /v1/chat/completions` - Create a chat completion

### Models
- `GET /v1/models` - List available models

### Conversations
- `POST /v1/conversations` - Create a new conversation
- `GET /v1/conversations` - List active conversations
- `GET /v1/conversations/:id` - Get conversation details

### Statistics
- `GET /stats` - Get API usage statistics

### Health Check
- `GET /health` - Check service health

## Advanced Usage

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

## Performance Optimization

### Connection Pooling
- **First request**: 2-5 seconds (with pre-warmed connection pool)
- **Subsequent requests**: < 0.1 seconds (reusing existing connections)
- **Concurrent handling**: Multiple requests can share the connection pool

### Client Modes
1. **OneShot Mode**: Simple, stateless queries (default)
2. **Interactive Mode**: Maintains conversation context across requests
3. **Batch Mode**: Process multiple queries concurrently for high throughput

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

## Security

- File access is controlled through configurable permissions
- MCP servers run in isolated processes
- No API key required (relies on Claude CLI authentication)
- Supports CORS for web applications
- Request ID tracking for audit trails

## Troubleshooting

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

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Original SDK: [ZhangHanDong/claude-code-api-rs](https://github.com/ZhangHanDong/claude-code-api-rs) (`cc-sdk`)
- Powered by [Claude Code CLI](https://claude.ai/download) from Anthropic
- Web framework: [Axum](https://github.com/tokio-rs/axum) for high-performance HTTP serving
- Async runtime: [Tokio](https://tokio.rs/) for blazing-fast async I/O

## Support

- [Report Issues](https://github.com/this-rs/nexus/issues)
- [Discussions](https://github.com/this-rs/nexus/discussions)

---

Made with Rust by the Nexus team
