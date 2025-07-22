# Claude Code SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/cc-sdk.svg)](https://crates.io/crates/cc-sdk)
[![Documentation](https://docs.rs/cc-sdk/badge.svg)](https://docs.rs/cc-sdk)
[![License](https://img.shields.io/crates/l/cc-sdk.svg)](LICENSE)

A Rust SDK for interacting with Claude Code CLI, providing both simple query interfaces and full interactive client capabilities with **complete feature parity with the official Python SDK**.

## Features

- 🚀 **Simple Query Interface** - One-shot queries with the `query()` function
- 💬 **Interactive Client** - Stateful conversations with context retention
- 🔄 **Streaming Support** - Real-time message streaming
- 🛑 **Interrupt Capability** - Cancel ongoing operations
- 🔧 **Full Configuration** - Comprehensive options matching Python SDK
- 📦 **Type Safety** - Strongly typed with serde support
- ⚡ **Async/Await** - Built on Tokio for async operations

## Python SDK Feature Parity

This Rust SDK provides **100% feature parity** with the official Python SDK (`claude_code_sdk`), including:

- ✅ **All client methods**: `query()`, `send_message()`, `receive_response()`, `interrupt()`
- ✅ **Interactive sessions**: Full stateful conversation support
- ✅ **Message streaming**: Real-time async message handling
- ✅ **All configuration options**: System prompts, models, permissions, tools, etc.
- ✅ **All message types**: User, Assistant, System, Result messages
- ✅ **Error handling**: Comprehensive error types matching Python SDK
- ✅ **Session management**: Multi-session support with context isolation

The API is designed to be familiar to Python SDK users while leveraging Rust's type safety and performance benefits.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cc-sdk = "0.1.5"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

## Prerequisites

Install Claude Code CLI:

```bash
npm install -g @anthropic-ai/claude-code
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

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.