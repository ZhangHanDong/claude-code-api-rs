# Claude Code SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/cc-sdk.svg)](https://crates.io/crates/cc-sdk)
[![Documentation](https://docs.rs/cc-sdk/badge.svg)](https://docs.rs/cc-sdk)
[![License](https://img.shields.io/crates/l/cc-sdk.svg)](LICENSE)

A Rust SDK for interacting with Claude Code CLI, providing both simple query interfaces and full interactive client capabilities.

> **v0.6.0**: WebSocket transport support! Connect to Claude Code CLI via `ws://` endpoints.

## Features

- 🚀 **Simple Query Interface** - One-shot queries with the `query()` function
- 💬 **Interactive Client** - Stateful conversations with context retention
- 🔄 **Streaming Support** - Real-time message streaming
- 🛑 **Interrupt Capability** - Cancel ongoing operations
- 🔧 **Full Configuration** - Comprehensive options for Claude Code
- 📦 **Type Safety** - Strongly typed with serde support
- ⚡ **Async/Await** - Built on Tokio for async operations
- 🔒 **Control Protocol** - Full support for permissions, hooks, and MCP servers
- 💰 **Token Optimization** - Built-in tools to minimize costs and track usage
- 📥 **Auto CLI Download** - Automatically downloads Claude Code CLI if not found (v0.4.0+)
- 📁 **File Checkpointing** - Rewind file changes to any point in conversation (v0.4.0+)
- 📊 **Structured Output** - JSON schema validation for responses (v0.4.0+)
- 🌐 **WebSocket Transport** - Connect via WebSocket instead of subprocess (v0.6.0+, feature-gated)

## Python SDK Parity (v0.4.0)

This Rust SDK achieves **100% feature parity** with the official Python `claude-agent-sdk` v0.1.14:

| Feature | Python SDK | Rust SDK | Status |
|---------|-----------|----------|--------|
| Simple query API | ✅ | ✅ | ✅ Parity |
| Interactive client | ✅ | ✅ | ✅ Parity |
| Streaming messages | ✅ | ✅ | ✅ Parity |
| `tools` (base tool set) | ✅ | ✅ | ✅ Parity |
| `permission_mode` | ✅ | ✅ | ✅ Parity |
| `max_budget_usd` | ✅ | ✅ | ✅ Parity |
| `fallback_model` | ✅ | ✅ | ✅ Parity |
| `output_format` (structured) | ✅ | ✅ | ✅ Parity |
| `enable_file_checkpointing` | ✅ | ✅ | ✅ Parity |
| `rewind_files()` | ✅ | ✅ | ✅ Parity |
| `sandbox` | ✅ | ✅ | ✅ Parity |
| `plugins` | ✅ | ✅ | ✅ Parity |
| `betas` (SDK beta features) | ✅ | ✅ | ✅ Parity |
| Permission callbacks | ✅ | ✅ | ✅ Parity |
| Hook callbacks | ✅ | ✅ | ✅ Parity |
| MCP servers (all types) | ✅ | ✅ | ✅ Parity |
| Bundled/Auto CLI | ✅ (bundled) | ✅ (auto-download) | ✅ Equivalent |

> **Note**: Only `user` (OS setuid) is not implemented due to platform/privilege requirements.

## Token Optimization (New in v0.1.12)

Minimize token consumption and control costs with built-in optimization tools:

```rust
use cc_sdk::{ClaudeCodeOptions, ClaudeSDKClient, PermissionMode};
use cc_sdk::token_tracker::BudgetLimit;
use cc_sdk::model_recommendation::ModelRecommendation;

// 1. Choose cost-effective model
let recommender = ModelRecommendation::default();
let model = recommender.suggest("simple").unwrap(); // → Haiku (cheapest)
// Or use latest Sonnet 4.5 for balanced tasks
let latest = recommender.suggest("latest").unwrap(); // → Sonnet 4.5

// 2. Configure for minimal token usage
let options = ClaudeCodeOptions::builder()
    .model(model)
    .max_turns(Some(3))              // Limit conversation length
    .max_output_tokens(2000)          // Cap response size (NEW)
    .allowed_tools(vec!["Read".to_string()])  // Restrict tools
    .permission_mode(PermissionMode::BypassPermissions)
    .build();

let mut client = ClaudeSDKClient::new(options);

// 3. Set budget with alerts
client.set_budget_limit(
    BudgetLimit::with_cost(5.0),      // $5 max
    Some(|msg| eprintln!("⚠️  {}", msg))  // Alert at 80%
).await;

// ... run your queries ...

// 4. Monitor usage
let usage = client.get_usage_stats().await;
println!("Tokens: {}, Cost: ${:.2}", usage.total_tokens(), usage.total_cost_usd);
```

**Key Features:**
- ✅ `max_output_tokens` - Precise output control (1-32000, overrides env var)
- ✅ `TokenUsageTracker` - Real-time token and cost monitoring
- ✅ `BudgetLimit` - Set cost/token caps with 80% warning threshold
- ✅ `ModelRecommendation` - Smart model selection (Haiku/Sonnet/Opus)
- ✅ Automatic usage tracking from `ResultMessage`

**Model Cost Comparison:**
- Haiku 3.5: **1x** (baseline, cheapest)
- Sonnet 4.5 (Latest): **~5x** more expensive, best balance ⭐
- Opus 4.1: **~15x** more expensive, most capable

See [Token Optimization Guide](docs/TOKEN_OPTIMIZATION.md) for complete strategies and examples.

## Complete Feature Set

This Rust SDK provides comprehensive functionality for Claude Code interactions:

- ✅ **Client methods**: `query()`, `send_message()`, `receive_response()`, `interrupt()`
- ✅ **Interactive sessions**: Full stateful conversation support
- ✅ **Message streaming**: Real-time async message handling
- ✅ **Configuration options**: System prompts, models, permissions, tools, etc.
- ✅ **Message types**: User, Assistant, System, Result messages
- ✅ **Error handling**: Comprehensive error types with detailed diagnostics
- ✅ **Session management**: Multi-session support with context isolation
- ✅ **Type safety**: Leveraging Rust's type system for reliable code
- ✅ **Control Protocol**: Permission callbacks, hook system, MCP servers (SDK type)
- ✅ **CLI Compatibility**: Configurable protocol format for maximum compatibility
- ✅ **Account Information**: Retrieve current account details programmatically

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cc-sdk = "0.6.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

### WebSocket Transport (Optional)

Enable the `websocket` feature to connect via WebSocket instead of spawning a subprocess:

```toml
[dependencies]
cc-sdk = { version = "0.6.0", features = ["websocket"] }
```

```rust
use cc_sdk::{WebSocketTransport, WebSocketConfig};
use cc_sdk::transport::Transport;

#[tokio::main]
async fn main() -> cc_sdk::Result<()> {
    let config = WebSocketConfig {
        auth_token: Some("my-bearer-token".into()),
        ..Default::default()
    };

    let mut transport = WebSocketTransport::new(
        "ws://localhost:8080/ws/cli/my-session",
        config,
    );
    transport.connect().await?;

    // Use transport.send_message(), transport.receive_messages(), etc.
    // Same Transport trait as SubprocessTransport.

    transport.disconnect().await?;
    Ok(())
}
```

This is useful when the CLI is managed externally (e.g., by an API server with `--sdk-url`), and you want to communicate over a network connection rather than stdin/stdout.

### Automatic CLI Download (Default)

The SDK will automatically download Claude Code CLI if it's not found on your system:

```rust
let options = ClaudeCodeOptions::builder()
    .auto_download_cli(true)  // Enabled by default
    .build();
```

CLI is cached in platform-specific locations:
- **macOS**: `~/Library/Caches/cc-sdk/cli/`
- **Linux**: `~/.cache/cc-sdk/cli/`
- **Windows**: `%LOCALAPPDATA%\cc-sdk\cli\`

To disable auto-download, use:

```toml
[dependencies]
cc-sdk = { version = "0.4.0", default-features = false }
```

## Prerequisites

Claude Code CLI is **automatically downloaded** by the SDK if not found (v0.4.0+).

For manual installation:

```bash
npm install -g @anthropic-ai/claude-code
```

## Environment Setup

For reliable SDK operation, set the `ANTHROPIC_USER_EMAIL` environment variable:

```bash
export ANTHROPIC_USER_EMAIL="your-email@example.com"
```

Or create a `.env` file in your project:

```bash
# .env
ANTHROPIC_USER_EMAIL=your-email@example.com
CLAUDE_MODEL=claude-sonnet-4-5-20250929
```

See [Environment Variables Guide](docs/ENVIRONMENT_VARIABLES.md) for complete details.

## Supported Models (2025)

The SDK supports the latest Claude models available in 2025:

### Latest Models
- **Opus 4.1** - Most capable model
  - Full name: `"claude-opus-4-1-20250805"`
  - Alias: `"opus"` (recommended - uses latest Opus)
  
- **Sonnet 4** - Balanced performance
  - Full name: `"claude-sonnet-4-20250514"`
  - Alias: `"sonnet"` (recommended - uses latest Sonnet)

### Previous Generation
- **Claude 3.5 Sonnet** - `"claude-3-5-sonnet-20241022"`
- **Claude 3.5 Haiku** - `"claude-3-5-haiku-20241022"` (fastest)

### Using Models in Code

```rust
use cc_sdk::{query, ClaudeCodeOptions, Result};

// Using Opus 4.1 (recommended: use alias)
let options = ClaudeCodeOptions::builder()
    .model("opus")  // or "claude-opus-4-1-20250805" for specific version
    .build();

// Using Sonnet 4 (recommended: use alias)
let options = ClaudeCodeOptions::builder()
    .model("sonnet")  // or "claude-sonnet-4-20250514" for specific version
    .build();

let mut messages = query("Your prompt", Some(options)).await?;
```

## Quick Start

### Simple Query (One-shot)

```rust
use cc_sdk::{query, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut messages = query("What is 2 + 2?", None).await?;
    
    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }
    
    Ok(())
}
```

### Interactive Client

```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // Send a message and receive response
    let messages = client.send_and_receive(
        "Help me write a Python web server".to_string()
    ).await?;

    // Process responses
    for msg in &messages {
        match msg {
            cc_sdk::Message::Assistant { message } => {
                println!("Claude: {:?}", message);
            }
            _ => {}
        }
    }

    // Send follow-up
    let messages = client.send_and_receive(
        "Make it use async/await".to_string()
    ).await?;

    client.disconnect().await?;
    Ok(())
}
```

### Account Information (New)

```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    // Get current account information
    let account_info = client.get_account_info().await?;
    println!("Current account: {}", account_info);

    client.disconnect().await?;
    Ok(())
}
```

### Streaming Output (Since v0.1.8)

```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;
    
    // Send a message
    client.send_message("Explain quantum computing".to_string()).await?;
    
    // Receive messages as a stream
    let mut stream = client.receive_messages_stream().await;
    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                println!("Received: {:?}", message);
                if matches!(message, cc_sdk::Message::Result { .. }) {
                    break;
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    // Or use the convenience method that stops at Result message
    client.send_message("What's 2 + 2?".to_string()).await?;
    let mut stream = client.receive_response_stream().await;
    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => println!("Message: {:?}", message),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    client.disconnect().await?;
    Ok(())
}
```

### Advanced Usage

```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;
    
    // Send message without waiting for response
    client.send_message("Calculate pi to 100 digits".to_string()).await?;
    
    // Do other work...
    
    // Receive response when ready (stops at Result message)
    let messages = client.receive_response().await?;
    
    // Cancel long-running operations
    client.send_message("Write a 10,000 word essay".to_string()).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    client.interrupt().await?;
    
    client.disconnect().await?;
    Ok(())
}
```

## Configuration Options

```rust
use cc_sdk::{ClaudeCodeOptions, PermissionMode, ControlProtocolFormat};

let options = ClaudeCodeOptions::builder()
    .system_prompt("You are a helpful coding assistant")
    .model("claude-3-5-sonnet-20241022")
    .permission_mode(PermissionMode::AcceptEdits)
    .max_turns(10)
    .max_thinking_tokens(10000)
    .allowed_tools(vec!["read_file".to_string(), "write_file".to_string()])
    .cwd("/path/to/project")
    // New in v0.1.6
    .settings("claude-settings.json")  // Use custom settings file
    .add_dir("/path/to/related/project")  // Add additional working directories
    .add_dirs(vec![PathBuf::from("/dir1"), PathBuf::from("/dir2")])  // Add multiple dirs
    // New in v0.1.11: Control protocol format configuration
    .control_protocol_format(ControlProtocolFormat::Legacy)  // Default: maximum compatibility
    .build();
```

### Control Protocol (v0.1.12+)

New request helpers and options aligned with the Python SDK:

- `Query::set_permission_mode("acceptEdits" | "default" | "plan" | "bypassPermissions")`
- `Query::set_model(Some("sonnet"))` or `set_model(None)` to clear
- `ClaudeCodeOptions::builder().include_partial_messages(true)` to include partial assistant chunks
- `Query::stream_input(stream)` automatically calls end_input when finished

Example:

```rust
use cc_sdk::{Query, ClaudeCodeOptions};
use cc_sdk::transport::SubprocessTransport;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

# async fn demo() -> cc_sdk::Result<()> {
let options = ClaudeCodeOptions::builder()
    .model("sonnet")
    .include_partial_messages(true)
    .build();

let transport: Box<dyn cc_sdk::transport::Transport + Send> =
    Box::new(SubprocessTransport::new(options)?);
let transport = Arc::new(Mutex::new(transport));

let mut q = Query::new(transport, true, None, None, HashMap::new());
q.start().await?;                  // start routing
q.set_permission_mode("acceptEdits").await?;
q.set_model(Some("opus".into())).await?;

// Stream input; end_input is called automatically when the stream completes
let inputs = vec![serde_json::json!("Hello"), serde_json::json!({"content":"Ping"})];
q.stream_input(futures::stream::iter(inputs)).await?;
# Ok(()) }
```

Advanced flags mapped to CLI:
- `fork_session(true)` → `--fork-session`
- `setting_sources(vec![User, Project, Local])` → `--setting-sources user,project,local`
- `agents(map)` → `--agents '<json>'`

### Agent Tools & MCP

- Tools whitelist/blacklist: set `allowed_tools` / `disallowed_tools` in `ClaudeCodeOptions`.
- Permission mode: `PermissionMode::{Default, AcceptEdits, Plan, BypassPermissions}`.
- Runtime approvals: implement `CanUseTool` and return `PermissionResult::{Allow,Deny}`.
- MCP servers: configure via `options.mcp_servers` (stdio/http/sse/sdk), SDK packs JSON for `--mcp-config`.

```rust
use cc_sdk::{ClaudeCodeOptions, PermissionMode, CanUseTool, ToolPermissionContext, PermissionResult,
             PermissionResultAllow, transport::{Transport, SubprocessTransport}, Query};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

struct AllowRead;
#[async_trait::async_trait]
impl CanUseTool for AllowRead {
  async fn can_use_tool(&self, tool:&str, _input:&serde_json::Value, _ctx:&ToolPermissionContext) -> PermissionResult {
    if tool == "Read" { PermissionResult::Allow(PermissionResultAllow{updated_input: None, updated_permissions: None}) }
    else { cc_sdk::PermissionResult::Deny(cc_sdk::PermissionResultDeny{ message: "Not allowed".into(), interrupt: false }) }
  }
}

# async fn demo() -> cc_sdk::Result<()> {
let mut opts = ClaudeCodeOptions::builder()
  .permission_mode(PermissionMode::AcceptEdits)
  .include_partial_messages(true)
  .build();
opts.allowed_tools = vec!["Read".into()];

let mut mcp = HashMap::new();
mcp.insert("filesystem".into(), cc_sdk::McpServerConfig::Stdio{ command: "npx".into(), args: Some(vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), "/allowed".into()]), env: None });
opts.mcp_servers = mcp;

let transport: Box<dyn Transport + Send> = Box::new(SubprocessTransport::new(opts)?);
let transport = Arc::new(Mutex::new(transport));
let mut q = Query::new(transport, true, Some(Arc::new(AllowRead)), None, HashMap::new());
q.start().await?;
# Ok(()) }
```

### Control Protocol Compatibility (v0.1.11+)

The SDK supports configurable control protocol formats for CLI compatibility:

- **Legacy** (default): Uses `sdk_control_request/response` format - works with all CLI versions
- **Control**: Uses new `type=control` format - for newer CLI versions
- **Auto**: Currently defaults to Legacy, will auto-negotiate in future

```rust
// Use environment variable to override (useful for testing)
// export CLAUDE_CODE_CONTROL_FORMAT=legacy  # or "control"

// Or configure programmatically
let options = ClaudeCodeOptions::builder()
    .control_protocol_format(ControlProtocolFormat::Legacy)
    .build();
```

See [CONTROL_PROTOCOL_COMPATIBILITY.md](CONTROL_PROTOCOL_COMPATIBILITY.md) for detailed information.

## API Reference

### `query()`

Simple, stateless query function for one-shot interactions.

```rust
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeCodeOptions>
) -> Result<impl Stream<Item = Result<Message>>>
```

### `InteractiveClient`

Main client for stateful, interactive conversations.

#### Methods

- `new(options: ClaudeCodeOptions) -> Result<Self>` - Create a new client
- `connect() -> Result<()>` - Connect to Claude CLI
- `send_and_receive(prompt: String) -> Result<Vec<Message>>` - Send message and wait for complete response
- `send_message(prompt: String) -> Result<()>` - Send message without waiting
- `receive_response() -> Result<Vec<Message>>` - Receive messages until Result message
- `interrupt() -> Result<()>` - Cancel ongoing operation
- `disconnect() -> Result<()>` - Disconnect from Claude CLI

## Message Types

- `UserMessage` - User input messages
- `AssistantMessage` - Claude's responses
- `SystemMessage` - System notifications
- `ResultMessage` - Operation results with timing and cost info

## Error Handling

The SDK provides comprehensive error types:

- `CLINotFoundError` - Claude Code CLI not installed
- `CLIConnectionError` - Connection failures
- `ProcessError` - CLI process errors
- `InvalidState` - Invalid operation state

## Examples

Check the `examples/` directory for more usage examples:

- `interactive_demo.rs` - Interactive conversation demo
- `query_simple.rs` - Simple query example
- `file_operations.rs` - File manipulation example

### New Features (v0.1.6)

Test the latest features with these examples:

- `test_settings.rs` - Using custom settings files
- `test_settings_safe.rs` - Safe settings file handling with path detection
- `test_add_dirs.rs` - Adding multiple working directories
- `test_combined_features.rs` - Combining settings and add_dirs
- `test_new_options.rs` - Testing the new builder methods

Example settings files are provided:
- `examples/claude-settings.json` - Basic settings configuration
- `examples/custom-claude-settings.json` - Advanced settings with MCP servers

**Note**: When running examples from the project root, use:
```bash
cargo run --example test_settings
```

The settings files use relative paths from the project root (e.g., `examples/claude-settings.json`)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
