# Claude Code API

[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/yourusername/claude-code-api)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

[中文文档](README_CN.md) | English

A high-performance Rust implementation of an OpenAI-compatible API gateway for Claude Code CLI. This project provides a RESTful API interface that allows you to interact with Claude Code using the familiar OpenAI API format.

## ✨ Features

- **🔌 OpenAI API Compatibility** - Drop-in replacement for OpenAI API, works with existing OpenAI client libraries
- **🚀 High Performance** - Built with Rust, Axum, and Tokio for exceptional performance
- **⚡ Interactive Sessions** - Reuse Claude processes across requests for 5-10x faster responses
- **💬 Conversation Management** - Built-in session support for multi-turn conversations
- **🖼️ Multimodal Support** - Process images alongside text in your requests
- **⚡ Response Caching** - Intelligent caching system to reduce latency and costs
- **🔧 MCP Support** - Model Context Protocol integration for accessing external tools and services
- **📁 File Access Control** - Configurable file system permissions for secure operations
- **🌊 Streaming Responses** - Real-time streaming support for long-form content
- **🛡️ Robust Error Handling** - Comprehensive error handling with automatic retries
- **📊 Statistics API** - Monitor usage and performance metrics

## 🚀 Quick Start

### Prerequisites

- Rust 1.75 or higher
- [Claude CLI](https://claude.ai/download) installed and configured
- (Optional) MCP servers for extended functionality

### Installation

**Option 1**

```
cargo install claude-code-api
```

then run:

```
RUST_LOG=info claude-code-api
```

or

```
RUST_LOG=info ccapi
```

**Option 2**

```bash
git clone https://github.com/yourusername/claude-code-api.git
cd claude-code-api/rust-claude-code-api
```

 Build the project:
```bash
cargo build --release
```

Start the server:
```bash
./target/release/claude-code-api
```

The API server will start on `http://localhost:8080` by default.

### Quick Test

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [
      {"role": "user", "content": "Hello, Claude!"}
    ]
  }'
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
    model="claude-opus-4-20250514",
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
    model="claude-opus-4-20250514",
    messages=[
        {"role": "user", "content": "My name is Alice"}
    ]
)
conversation_id = response.conversation_id

# Subsequent request - continues the conversation
response = client.chat.completions.create(
    model="claude-opus-4-20250514",
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
use_interactive_sessions = true  # Enable process reuse for faster responses

[file_access]
skip_permissions = false
additional_dirs = ["/Users/me/projects", "/tmp"]

[mcp]
enabled = true
config_file = "./mcp_config.json"
strict = false
debug = false
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

### Interactive Sessions

The API supports interactive session management for dramatically improved performance:

- **First request**: 5-15 seconds (Claude process startup)
- **Subsequent requests**: 1-3 seconds (process reuse)

Enable interactive sessions (enabled by default):
```toml
[claude]
use_interactive_sessions = true
```

### Best Practices

1. **Use conversation IDs** for related requests to reuse sessions
2. **Enable response caching** for frequently repeated queries
3. **Configure appropriate timeouts** based on your use case
4. **Monitor active sessions** via the `/stats` endpoint

For detailed information, see [Interactive Session Guide](doc/INTERACTIVE_SESSION_GUIDE.md).

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

- Built on top of [Claude Code CLI](https://claude.ai/download)
- Inspired by OpenAI's API design
- Powered by [Axum](https://github.com/tokio-rs/axum) web framework

## 📞 Support

- [Report Issues](https://github.com/yourusername/claude-code-api/issues)
- [Documentation](https://github.com/yourusername/claude-code-api/wiki)
- [Discussions](https://github.com/yourusername/claude-code-api/discussions)

---

Made with ❤️ by the Claude Code API team
