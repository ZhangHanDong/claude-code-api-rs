# Claude Code SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/cc-sdk.svg)](https://crates.io/crates/cc-sdk)
[![Documentation](https://docs.rs/cc-sdk/badge.svg)](https://docs.rs/cc-sdk)
[![License](https://img.shields.io/crates/l/cc-sdk.svg)](LICENSE)

A Rust SDK for interacting with Claude Code CLI, providing both simple query interfaces and full interactive client capabilities.

## Features

- 🚀 **Simple Query Interface** - One-shot queries with the `query()` function
- 💬 **Interactive Client** - Stateful conversations with context retention
- 🔄 **Streaming Support** - Real-time message streaming
- 🛑 **Interrupt Capability** - Cancel ongoing operations
- 🔧 **Full Configuration** - Comprehensive options for Claude Code
- 📦 **Type Safety** - Strongly typed with serde support
- ⚡ **Async/Await** - Built on Tokio for async operations

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

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cc-sdk = "0.1.7"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

## Prerequisites

Install Claude Code CLI:

```bash
npm install -g @anthropic-ai/claude-code
```

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
use cc_sdk::{ClaudeCodeOptions, PermissionMode};

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
    .build();
```

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